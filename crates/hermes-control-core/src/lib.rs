use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

use hermes_control_types::{
    AiProviderKind, AnthropicDefaults, CommandPreview, ControlConfig, EndpointStatus, HealthStatus,
    HermesAction, ModelAction, ModelRuntimeConfig, ModelRuntimeSummary, ModelRuntimeVariant,
    ModelRuntimesConfig, OpenWebUiAction, ProviderAccountConfig, ProviderConfig,
    ProviderSecretSource, ProvidersConfig, ReadOnlyStatus, RiskLevel, StateSummary, WslAction,
    WslDistroStatus,
};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("invalid TOML config: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("invalid JSON response: {0}")]
    Json(#[from] serde_json::Error),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("provider import failed: {0}")]
    ProviderImport(String),
    #[error("fixed command {program} failed with status {status}: {stderr}")]
    CommandFailed {
        program: &'static str,
        status: String,
        stderr: String,
    },
    #[error("LAN bind is disabled by policy but daemon.bind is {0}")]
    LanBindDisallowed(String),
    #[error("raw shell is disabled for Hermes Control")]
    RawShellAllowed,
}

pub fn parse_control_config(input: &str) -> Result<ControlConfig, ConfigError> {
    let config = toml::from_str::<ControlConfig>(input)?;
    validate_control_config(&config)?;
    Ok(config)
}

pub fn parse_providers_config(input: &str) -> Result<ProvidersConfig, ConfigError> {
    Ok(toml::from_str(input)?)
}

pub fn import_provider_json(input: &str) -> Result<ProvidersConfig, ConfigError> {
    let value = serde_json::from_str::<Value>(input)?;

    if value.get("providers").is_some() {
        return serde_json::from_value::<ProvidersConfig>(value).map_err(Into::into);
    }

    import_env_style_provider(value)
}

fn import_env_style_provider(value: Value) -> Result<ProvidersConfig, ConfigError> {
    let object = value
        .as_object()
        .ok_or_else(|| ConfigError::ProviderImport("JSON root must be an object".to_owned()))?;
    let import_type = object
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("claude-relay");
    if !matches!(import_type, "claude-relay" | "anthropic-relay") {
        return Err(ConfigError::ProviderImport(format!(
            "unsupported provider import type: {import_type}"
        )));
    }

    let token_ref = object
        .get("ANTHROPIC_AUTH_TOKEN")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            ConfigError::ProviderImport(
                "ANTHROPIC_AUTH_TOKEN must be an env or secret reference".to_owned(),
            )
        })?;
    let (secret_ref, secret_env_key, secret_source) = parse_secret_reference(token_ref)?;
    let base_url = get_json_string(object, "ANTHROPIC_BASE_URL")
        .ok_or_else(|| ConfigError::ProviderImport("ANTHROPIC_BASE_URL is required".to_owned()))?;
    let default_model = get_json_string(object, "ANTHROPIC_MODEL");
    let sonnet = get_json_string(object, "ANTHROPIC_DEFAULT_SONNET_MODEL");
    let haiku = get_json_string(object, "ANTHROPIC_DEFAULT_HAIKU_MODEL");
    let opus = get_json_string(object, "ANTHROPIC_DEFAULT_OPUS_MODEL");
    let models = [
        default_model.clone(),
        sonnet.clone(),
        haiku.clone(),
        opus.clone(),
    ]
    .into_iter()
    .flatten()
    .collect::<BTreeSet<_>>()
    .into_iter()
    .collect::<Vec<_>>();
    let mut runtime_env = BTreeMap::new();
    for key in [
        "API_TIMEOUT_MS",
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "NO_PROXY",
        "effortLevel",
    ] {
        if let Some(value) = get_json_string(object, key) {
            runtime_env.insert(key.to_owned(), value);
        }
    }

    let provider = ProviderConfig {
        id: get_json_string(object, "id").unwrap_or_else(|| "external.api-relay".to_owned()),
        kind: AiProviderKind::AnthropicClaude,
        display_name: get_json_string(object, "name").unwrap_or_else(|| "Claude Relay".to_owned()),
        base_url: Some(base_url),
        api_key_ref: Some(secret_ref.clone()),
        models,
        default_account_id: Some("main".to_owned()),
        default_model: default_model.clone().or_else(|| sonnet.clone()),
        anthropic_defaults: Some(AnthropicDefaults {
            model: default_model,
            sonnet,
            haiku,
            opus,
        }),
        runtime_env,
        accounts: vec![ProviderAccountConfig {
            id: "main".to_owned(),
            display_name: "Main relay token".to_owned(),
            secret_ref,
            secret_env_key,
            secret_source,
            enabled: true,
            priority: 10,
        }],
        model_runtime: None,
        served_model_name: None,
    };

    Ok(ProvidersConfig {
        providers: vec![provider],
    })
}

fn get_json_string(object: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn parse_secret_reference(
    value: &str,
) -> Result<(String, String, ProviderSecretSource), ConfigError> {
    let trimmed = value.trim();
    let env_key = trimmed
        .strip_prefix("$env:")
        .or_else(|| trimmed.strip_prefix("env:"));
    if let Some(env_key) = env_key {
        validate_env_key(env_key)?;
        return Ok((
            format!("env:{env_key}"),
            env_key.to_owned(),
            ProviderSecretSource::Env,
        ));
    }

    if let Some(secret_ref) = trimmed.strip_prefix("secret_ref:") {
        if secret_ref.trim().is_empty() {
            return Err(ConfigError::ProviderImport(
                "secret_ref import reference is empty".to_owned(),
            ));
        }
        return Ok((
            secret_ref.to_owned(),
            "ANTHROPIC_AUTH_TOKEN".to_owned(),
            ProviderSecretSource::SecretRef,
        ));
    }

    Err(ConfigError::ProviderImport(
        "raw secret values are not allowed in provider JSON imports".to_owned(),
    ))
}

fn validate_env_key(value: &str) -> Result<(), ConfigError> {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return Err(ConfigError::ProviderImport("env key is empty".to_owned()));
    };
    if !first.is_ascii_uppercase() {
        return Err(ConfigError::ProviderImport(format!(
            "env key must start with an uppercase letter: {value}"
        )));
    }
    if !chars.all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_') {
        return Err(ConfigError::ProviderImport(format!(
            "env key contains unsupported characters: {value}"
        )));
    }
    Ok(())
}

pub fn parse_model_runtimes_config(input: &str) -> Result<ModelRuntimesConfig, ConfigError> {
    Ok(toml::from_str(input)?)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigSet {
    pub control: ControlConfig,
    pub providers: ProvidersConfig,
    pub model_runtimes: ModelRuntimesConfig,
}

pub fn load_config_dir(config_dir: impl AsRef<Path>) -> Result<ConfigSet, ConfigError> {
    let config_dir = config_dir.as_ref();
    let control = parse_control_config(&fs::read_to_string(config_dir.join("control.toml"))?)?;
    let providers =
        parse_providers_config(&fs::read_to_string(config_dir.join("providers.toml"))?)?;
    let model_runtimes =
        parse_model_runtimes_config(&fs::read_to_string(config_dir.join("model-runtimes.toml"))?)?;

    Ok(ConfigSet {
        control,
        providers,
        model_runtimes,
    })
}

pub fn validate_control_config(config: &ControlConfig) -> Result<(), ConfigError> {
    if !config.policy.allow_lan_bind && !is_local_bind(&config.daemon.bind) {
        return Err(ConfigError::LanBindDisallowed(config.daemon.bind.clone()));
    }

    if config.policy.allow_raw_shell {
        return Err(ConfigError::RawShellAllowed);
    }

    Ok(())
}

fn is_local_bind(bind: &str) -> bool {
    bind.starts_with("127.") || bind.starts_with("localhost:") || bind.starts_with("[::1]:")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixedProgram {
    WslExe,
}

impl FixedProgram {
    pub fn executable(self) -> &'static str {
        match self {
            Self::WslExe => "wsl.exe",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnownWslOperation {
    ListVerbose,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WslCommandSpec {
    operation: KnownWslOperation,
}

impl WslCommandSpec {
    pub fn new(operation: KnownWslOperation) -> Self {
        Self { operation }
    }

    pub fn to_command(&self) -> FixedCommand {
        match self.operation {
            KnownWslOperation::ListVerbose => FixedCommand {
                program: FixedProgram::WslExe,
                args: vec!["--list".to_owned(), "--verbose".to_owned()],
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedCommand {
    pub program: FixedProgram,
    pub args: Vec<String>,
}

impl FixedCommand {
    fn preview(&self) -> CommandPreview {
        CommandPreview {
            program: self.program.executable().to_owned(),
            args: self.args.clone(),
            env: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationPlan {
    pub risk: RiskLevel,
    pub summary: String,
    pub commands: Vec<CommandPreview>,
    pub requires_confirmation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WslController {
    distro: String,
    default_user: String,
}

impl WslController {
    pub fn new(distro: impl Into<String>) -> Self {
        Self {
            distro: distro.into(),
            default_user: "root".to_owned(),
        }
    }

    pub fn with_default_user(distro: impl Into<String>, default_user: impl Into<String>) -> Self {
        Self {
            distro: distro.into(),
            default_user: default_user.into(),
        }
    }

    pub fn plan(&self, action: WslAction) -> OperationPlan {
        match action {
            WslAction::Wake => OperationPlan {
                risk: RiskLevel::NormalMutating,
                summary: format!("Wake WSL distro {}", self.distro),
                commands: vec![self.wake_command()],
                requires_confirmation: false,
            },
            WslAction::StopDistro => OperationPlan {
                risk: RiskLevel::Destructive,
                summary: format!(
                    "Stop WSL distro {} and terminate in-distro processes.",
                    self.distro
                ),
                commands: vec![self.terminate_command()],
                requires_confirmation: true,
            },
            WslAction::RestartDistro => OperationPlan {
                risk: RiskLevel::Destructive,
                summary: format!(
                    "Restart WSL distro {}. Hermes runtime and local model processes in that distro may stop.",
                    self.distro
                ),
                commands: vec![self.terminate_command(), self.wake_command()],
                requires_confirmation: true,
            },
            WslAction::ShutdownAll => OperationPlan {
                risk: RiskLevel::Destructive,
                summary: "Shutdown all WSL distributions and terminate all in-distro processes."
                    .to_owned(),
                commands: vec![
                    FixedCommand {
                        program: FixedProgram::WslExe,
                        args: vec!["--shutdown".to_owned()],
                    }
                    .preview(),
                ],
                requires_confirmation: true,
            },
        }
    }

    fn terminate_command(&self) -> CommandPreview {
        FixedCommand {
            program: FixedProgram::WslExe,
            args: vec!["--terminate".to_owned(), self.distro.clone()],
        }
        .preview()
    }

    fn wake_command(&self) -> CommandPreview {
        FixedCommand {
            program: FixedProgram::WslExe,
            args: vec![
                "--distribution".to_owned(),
                self.distro.clone(),
                "--user".to_owned(),
                self.default_user.clone(),
                "--exec".to_owned(),
                "true".to_owned(),
            ],
        }
        .preview()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HermesRuntimeController {
    agent_root: String,
    health_url: String,
    wsl_distro: Option<String>,
    wsl_user: Option<String>,
}

const HERMES_CONTROL_WSL_BIN: &str = "/opt/hermes-control/bin";
const HERMES_CONTROL_START_SCRIPT: &str = "hermes-control-start.sh";
const HERMES_CONTROL_STOP_SCRIPT: &str = "hermes-control-stop.sh";
const HERMES_CONTROL_RESTART_SCRIPT: &str = "hermes-control-restart.sh";
const HERMES_CONTROL_KILL_SCRIPT: &str = "hermes-control-kill.sh";
const HERMES_CONTROL_HEALTH_SCRIPT: &str = "hermes-control-health.sh";
const HERMES_CONTROL_VLLM_START_SCRIPT: &str = "hermes-control-vllm-start.sh";
const HERMES_CONTROL_VLLM_START_WITH_FALLBACK_SCRIPT: &str =
    "hermes-control-vllm-start-with-fallback.sh";
const HERMES_CONTROL_VLLM_STOP_SCRIPT: &str = "hermes-control-vllm-stop.sh";
const HERMES_CONTROL_VLLM_HEALTH_SCRIPT: &str = "hermes-control-vllm-health.sh";
const HERMES_CONTROL_VLLM_LOGS_SCRIPT: &str = "hermes-control-vllm-logs.sh";
const HERMES_CONTROL_VLLM_BENCHMARK_SCRIPT: &str = "hermes-control-vllm-benchmark.sh";
const HERMES_CONTROL_VLLM_BOOTSTRAP_SCRIPT: &str = "hermes-control-vllm-bootstrap.sh";
const HERMES_CONTROL_OPENWEBUI_STATUS_SCRIPT: &str = "hermes-control-openwebui-status.sh";
const HERMES_CONTROL_OPENWEBUI_REFRESH_SCRIPT: &str = "hermes-control-openwebui-refresh.sh";
const HERMES_CONTROL_OPENWEBUI_STOP_SCRIPT: &str = "hermes-control-openwebui-stop.sh";

impl HermesRuntimeController {
    pub fn new(agent_root: impl Into<String>, health_url: impl Into<String>) -> Self {
        Self {
            agent_root: agent_root.into(),
            health_url: health_url.into(),
            wsl_distro: None,
            wsl_user: None,
        }
    }

    pub fn with_wsl(
        agent_root: impl Into<String>,
        health_url: impl Into<String>,
        wsl_distro: impl Into<String>,
        wsl_user: impl Into<String>,
    ) -> Self {
        Self {
            agent_root: agent_root.into(),
            health_url: health_url.into(),
            wsl_distro: Some(wsl_distro.into()),
            wsl_user: Some(wsl_user.into()),
        }
    }

    pub fn plan(&self, action: HermesAction) -> OperationPlan {
        match action {
            HermesAction::Wake => OperationPlan {
                risk: RiskLevel::NormalMutating,
                summary: format!(
                    "Wake Hermes runtime at {} and verify health at {}.",
                    self.agent_root, self.health_url
                ),
                commands: self
                    .hermes_commands(&[HERMES_CONTROL_START_SCRIPT, HERMES_CONTROL_HEALTH_SCRIPT]),
                requires_confirmation: false,
            },
            HermesAction::Stop => OperationPlan {
                risk: RiskLevel::Destructive,
                summary: format!("Stop Hermes runtime at {}.", self.agent_root),
                commands: self.hermes_commands(&[HERMES_CONTROL_STOP_SCRIPT]),
                requires_confirmation: true,
            },
            HermesAction::Restart => OperationPlan {
                risk: RiskLevel::Destructive,
                summary: format!(
                    "Restart Hermes runtime at {} and verify health at {}.",
                    self.agent_root, self.health_url
                ),
                commands: self.hermes_commands(&[
                    HERMES_CONTROL_RESTART_SCRIPT,
                    HERMES_CONTROL_HEALTH_SCRIPT,
                ]),
                requires_confirmation: true,
            },
            HermesAction::Kill => OperationPlan {
                risk: RiskLevel::Destructive,
                summary: format!(
                    "Kill Hermes runtime at {}. Daemon and last-known-good route stay outside the runtime.",
                    self.agent_root
                ),
                commands: self.hermes_commands(&[HERMES_CONTROL_KILL_SCRIPT]),
                requires_confirmation: true,
            },
        }
    }

    fn hermes_commands(&self, scripts: &[&str]) -> Vec<CommandPreview> {
        let (Some(distro), Some(user)) = (&self.wsl_distro, &self.wsl_user) else {
            return Vec::new();
        };

        scripts
            .iter()
            .map(|script| self.hermes_script_command(distro, user, script))
            .collect()
    }

    fn hermes_script_command(&self, distro: &str, user: &str, script: &str) -> CommandPreview {
        let mut args = vec![
            "--distribution".to_owned(),
            distro.to_owned(),
            "--user".to_owned(),
            user.to_owned(),
            "--exec".to_owned(),
            format!("{HERMES_CONTROL_WSL_BIN}/{script}"),
        ];
        if script == HERMES_CONTROL_HEALTH_SCRIPT {
            args.extend(["30".to_owned(), "ready".to_owned()]);
        }

        FixedCommand {
            program: FixedProgram::WslExe,
            args,
        }
        .preview()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenWebUiController {
    wsl_distro: String,
    wsl_user: String,
}

impl OpenWebUiController {
    pub fn new(wsl_distro: impl Into<String>, wsl_user: impl Into<String>) -> Self {
        Self {
            wsl_distro: wsl_distro.into(),
            wsl_user: wsl_user.into(),
        }
    }

    pub fn plan(&self, action: OpenWebUiAction) -> OperationPlan {
        match action {
            OpenWebUiAction::Wake => OperationPlan {
                risk: RiskLevel::NormalMutating,
                summary: format!("Wake Open WebUI in WSL distro {}.", self.wsl_distro),
                commands: vec![
                    self.openwebui_command(HERMES_CONTROL_OPENWEBUI_REFRESH_SCRIPT, &["force"]),
                ],
                requires_confirmation: false,
            },
            OpenWebUiAction::Stop => OperationPlan {
                risk: RiskLevel::Destructive,
                summary: format!("Stop Open WebUI in WSL distro {}.", self.wsl_distro),
                commands: vec![self.openwebui_command(HERMES_CONTROL_OPENWEBUI_STOP_SCRIPT, &[])],
                requires_confirmation: true,
            },
            OpenWebUiAction::Restart => OperationPlan {
                risk: RiskLevel::Destructive,
                summary: format!("Restart Open WebUI in WSL distro {}.", self.wsl_distro),
                commands: vec![
                    self.openwebui_command(HERMES_CONTROL_OPENWEBUI_REFRESH_SCRIPT, &["force"]),
                ],
                requires_confirmation: true,
            },
            OpenWebUiAction::Status => OperationPlan {
                risk: RiskLevel::ReadOnly,
                summary: format!("Check Open WebUI status in WSL distro {}.", self.wsl_distro),
                commands: vec![self.openwebui_command(HERMES_CONTROL_OPENWEBUI_STATUS_SCRIPT, &[])],
                requires_confirmation: false,
            },
        }
    }

    fn openwebui_command(&self, script: &str, script_args: &[&str]) -> CommandPreview {
        let mut args = vec![
            "--distribution".to_owned(),
            self.wsl_distro.clone(),
            "--user".to_owned(),
            self.wsl_user.clone(),
            "--exec".to_owned(),
            format!("{HERMES_CONTROL_WSL_BIN}/{script}"),
        ];
        args.extend(script_args.iter().map(|arg| (*arg).to_owned()));

        FixedCommand {
            program: FixedProgram::WslExe,
            args,
        }
        .preview()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRuntimeController<'a> {
    model_runtimes: &'a ModelRuntimesConfig,
    wsl_user: String,
}

impl<'a> ModelRuntimeController<'a> {
    pub fn new(model_runtimes: &'a ModelRuntimesConfig, wsl_user: impl Into<String>) -> Self {
        Self {
            model_runtimes,
            wsl_user: wsl_user.into(),
        }
    }

    pub fn plan(&self, model_id: &str, action: ModelAction) -> Option<OperationPlan> {
        let (runtime, variant) = self.find_variant(model_id)?;
        let variant_id = variant.id.as_str();
        let served_model_name = variant.served_model_name.as_str();

        Some(match action {
            ModelAction::Install => OperationPlan {
                risk: RiskLevel::NormalMutating,
                summary: format!(
                    "Install or repair vLLM runtime for {variant_id} under {}.",
                    runtime.workspace
                ),
                commands: vec![self.vllm_command(
                    &runtime.wsl_distro,
                    HERMES_CONTROL_VLLM_BOOTSTRAP_SCRIPT,
                    &[variant_id],
                )],
                requires_confirmation: false,
            },
            ModelAction::Start => self.start_plan(runtime, variant),
            ModelAction::Stop => OperationPlan {
                risk: RiskLevel::Destructive,
                summary: format!("Stop vLLM model {served_model_name}."),
                commands: vec![self.vllm_command(
                    &runtime.wsl_distro,
                    HERMES_CONTROL_VLLM_STOP_SCRIPT,
                    &[served_model_name],
                )],
                requires_confirmation: true,
            },
            ModelAction::Restart => self.restart_plan(runtime, variant),
            ModelAction::Health => OperationPlan {
                risk: RiskLevel::ReadOnly,
                summary: format!("Check vLLM model {served_model_name} readiness."),
                commands: vec![self.vllm_command(
                    &runtime.wsl_distro,
                    HERMES_CONTROL_VLLM_HEALTH_SCRIPT,
                    &[served_model_name, "10", "ready"],
                )],
                requires_confirmation: false,
            },
            ModelAction::Logs => OperationPlan {
                risk: RiskLevel::ReadOnly,
                summary: format!("Tail vLLM logs for {variant_id}."),
                commands: vec![self.vllm_command(
                    &runtime.wsl_distro,
                    HERMES_CONTROL_VLLM_LOGS_SCRIPT,
                    &[variant_id, "200"],
                )],
                requires_confirmation: false,
            },
            ModelAction::Benchmark => OperationPlan {
                risk: RiskLevel::Experimental,
                summary: format!("Run controlled vLLM benchmark for {variant_id}."),
                commands: vec![self.vllm_command(
                    &runtime.wsl_distro,
                    HERMES_CONTROL_VLLM_BENCHMARK_SCRIPT,
                    &[variant_id],
                )],
                requires_confirmation: true,
            },
        })
    }

    fn find_variant(
        &self,
        model_id: &str,
    ) -> Option<(&'a ModelRuntimeConfig, &'a ModelRuntimeVariant)> {
        let served_match = self
            .model_runtimes
            .runtimes
            .iter()
            .flat_map(|runtime| {
                runtime
                    .variants
                    .iter()
                    .map(move |variant| (runtime, variant))
            })
            .find(|(_, variant)| variant.served_model_name == model_id);

        self.model_runtimes
            .runtimes
            .iter()
            .flat_map(|runtime| {
                runtime
                    .variants
                    .iter()
                    .map(move |variant| (runtime, variant))
            })
            .find(|(_, variant)| variant.id == model_id)
            .or(served_match)
    }

    fn start_plan(
        &self,
        runtime: &'a ModelRuntimeConfig,
        variant: &'a ModelRuntimeVariant,
    ) -> OperationPlan {
        let variant_id = variant.id.as_str();
        let served_model_name = variant.served_model_name.as_str();

        if let Some(fallback) = fallback_variant(runtime, variant) {
            return OperationPlan {
                risk: RiskLevel::NormalMutating,
                summary: format!(
                    "Start vLLM model {variant_id} on {} and wait for readiness at {}; fallback {} is used if MTP startup does not become healthy.",
                    runtime.wsl_distro, runtime.models_endpoint, fallback.id
                ),
                commands: vec![self.vllm_command(
                    &runtime.wsl_distro,
                    HERMES_CONTROL_VLLM_START_WITH_FALLBACK_SCRIPT,
                    &[variant_id, fallback.id.as_str()],
                )],
                requires_confirmation: false,
            };
        }

        OperationPlan {
            risk: RiskLevel::NormalMutating,
            summary: format!(
                "Start vLLM model {variant_id} on {} and wait for {} at {}.",
                runtime.wsl_distro, served_model_name, runtime.models_endpoint
            ),
            commands: vec![
                self.vllm_command(
                    &runtime.wsl_distro,
                    HERMES_CONTROL_VLLM_START_SCRIPT,
                    &[variant_id],
                ),
                self.vllm_command(
                    &runtime.wsl_distro,
                    HERMES_CONTROL_VLLM_HEALTH_SCRIPT,
                    &[served_model_name, "180", "ready"],
                ),
            ],
            requires_confirmation: false,
        }
    }

    fn restart_plan(
        &self,
        runtime: &'a ModelRuntimeConfig,
        variant: &'a ModelRuntimeVariant,
    ) -> OperationPlan {
        let variant_id = variant.id.as_str();
        let served_model_name = variant.served_model_name.as_str();

        if let Some(fallback) = fallback_variant(runtime, variant) {
            return OperationPlan {
                risk: RiskLevel::Destructive,
                summary: format!(
                    "Restart vLLM model {variant_id} and allow fallback {} if MTP startup does not become healthy.",
                    fallback.id
                ),
                commands: vec![
                    self.vllm_command(
                        &runtime.wsl_distro,
                        HERMES_CONTROL_VLLM_STOP_SCRIPT,
                        &[served_model_name],
                    ),
                    self.vllm_command(
                        &runtime.wsl_distro,
                        HERMES_CONTROL_VLLM_START_WITH_FALLBACK_SCRIPT,
                        &[variant_id, fallback.id.as_str()],
                    ),
                ],
                requires_confirmation: true,
            };
        }

        OperationPlan {
            risk: RiskLevel::Destructive,
            summary: format!(
                "Restart vLLM model {variant_id} and wait for {served_model_name} at {}.",
                runtime.models_endpoint
            ),
            commands: vec![
                self.vllm_command(
                    &runtime.wsl_distro,
                    HERMES_CONTROL_VLLM_STOP_SCRIPT,
                    &[served_model_name],
                ),
                self.vllm_command(
                    &runtime.wsl_distro,
                    HERMES_CONTROL_VLLM_START_SCRIPT,
                    &[variant_id],
                ),
                self.vllm_command(
                    &runtime.wsl_distro,
                    HERMES_CONTROL_VLLM_HEALTH_SCRIPT,
                    &[served_model_name, "180", "ready"],
                ),
            ],
            requires_confirmation: true,
        }
    }

    fn vllm_command(&self, distro: &str, script: &str, script_args: &[&str]) -> CommandPreview {
        let mut args = vec![
            "--distribution".to_owned(),
            distro.to_owned(),
            "--user".to_owned(),
            self.wsl_user.clone(),
            "--exec".to_owned(),
            format!("{HERMES_CONTROL_WSL_BIN}/{script}"),
        ];
        args.extend(script_args.iter().map(|arg| (*arg).to_owned()));

        FixedCommand {
            program: FixedProgram::WslExe,
            args,
        }
        .preview()
    }
}

fn fallback_variant<'a>(
    runtime: &'a ModelRuntimeConfig,
    variant: &'a ModelRuntimeVariant,
) -> Option<&'a ModelRuntimeVariant> {
    let speculative_method = variant.speculative_method.as_deref()?;
    if !speculative_method.eq_ignore_ascii_case("mtp") {
        return None;
    }

    runtime
        .variants
        .iter()
        .find(|candidate| {
            candidate.id != variant.id && candidate.mode.eq_ignore_ascii_case("stable")
        })
        .or_else(|| {
            runtime.variants.iter().find(|candidate| {
                candidate.id != variant.id && candidate.id.to_ascii_lowercase().contains("awq")
            })
        })
}

pub fn run_wsl_list_verbose() -> Result<Vec<WslDistroStatus>, ConfigError> {
    let command = WslCommandSpec::new(KnownWslOperation::ListVerbose).to_command();
    let output = Command::new(command.program.executable())
        .args(&command.args)
        .output()?;

    if !output.status.success() {
        return Err(ConfigError::CommandFailed {
            program: command.program.executable(),
            status: output.status.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }

    Ok(parse_wsl_list_verbose(&decode_command_output(
        &output.stdout,
    )))
}

fn decode_command_output(bytes: &[u8]) -> String {
    if bytes.windows(2).any(|pair| pair[1] == 0) {
        let words = bytes
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        String::from_utf16_lossy(&words)
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

pub fn parse_wsl_list_verbose(output: &str) -> Vec<WslDistroStatus> {
    output
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim().trim_matches('\u{feff}');
            if trimmed.is_empty() || trimmed.starts_with("NAME") {
                return None;
            }

            let default = trimmed.starts_with('*');
            let without_marker = trimmed.trim_start_matches('*').trim();
            let parts = without_marker.split_whitespace().collect::<Vec<_>>();
            if parts.len() < 2 {
                return None;
            }

            Some(WslDistroStatus {
                name: parts[0].to_owned(),
                state: parts[1].to_owned(),
                version: parts.get(2).and_then(|value| value.parse::<u8>().ok()),
                default,
            })
        })
        .collect()
}

pub fn parse_wsl_hostname_ips(output: &str) -> Vec<String> {
    output
        .split_whitespace()
        .filter(|part| {
            part.chars()
                .all(|character| character.is_ascii_digit() || character == '.')
        })
        .map(ToOwned::to_owned)
        .collect()
}

pub fn build_wsl_models_endpoint(models_endpoint: &str, wsl_ip: &str) -> Option<String> {
    let localhost_prefixes = ["http://127.0.0.1:", "http://localhost:"];
    let prefix = localhost_prefixes
        .iter()
        .find(|prefix| models_endpoint.starts_with(**prefix))?;
    Some(models_endpoint.replacen(prefix, &format!("http://{wsl_ip}:"), 1))
}

pub fn run_wsl_hostname_ips(distro: &str, user: &str) -> Result<Vec<String>, ConfigError> {
    let output = Command::new(FixedProgram::WslExe.executable())
        .args([
            "--distribution",
            distro,
            "--user",
            user,
            "--exec",
            "hostname",
            "-I",
        ])
        .output()?;

    if !output.status.success() {
        return Err(ConfigError::CommandFailed {
            program: FixedProgram::WslExe.executable(),
            status: output.status.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }

    Ok(parse_wsl_hostname_ips(&decode_command_output(
        &output.stdout,
    )))
}

pub fn vllm_helper_response_ready(
    body: &str,
    served_model_name: &str,
) -> Result<bool, ConfigError> {
    let value = serde_json::from_str::<Value>(body)?;
    Ok(value.get("ready").and_then(Value::as_bool).unwrap_or(false)
        && value
            .get("served_model_name")
            .and_then(Value::as_str)
            .is_some_and(|model| model == served_model_name))
}

pub fn vllm_health_command(
    distro: &str,
    user: &str,
    models_endpoint: &str,
    served_model_name: &str,
) -> FixedCommand {
    FixedCommand {
        program: FixedProgram::WslExe,
        args: vec![
            "--distribution".to_owned(),
            distro.to_owned(),
            "--user".to_owned(),
            user.to_owned(),
            "--exec".to_owned(),
            "/usr/bin/env".to_owned(),
            format!("HERMES_CONTROL_VLLM_MODELS_ENDPOINT_OVERRIDE={models_endpoint}"),
            format!("{HERMES_CONTROL_WSL_BIN}/{HERMES_CONTROL_VLLM_HEALTH_SCRIPT}"),
            served_model_name.to_owned(),
            "1".to_owned(),
            "ready".to_owned(),
        ],
    }
}

pub fn run_wsl_vllm_health(
    distro: &str,
    user: &str,
    models_endpoint: &str,
    served_model_name: &str,
) -> Result<String, ConfigError> {
    let command = vllm_health_command(distro, user, models_endpoint, served_model_name);
    let output = Command::new(command.program.executable())
        .args(&command.args)
        .output()?;

    if !output.status.success() {
        return Err(ConfigError::CommandFailed {
            program: FixedProgram::WslExe.executable(),
            status: output.status.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }

    Ok(decode_command_output(&output.stdout))
}

pub fn models_response_has_model(body: &str, served_model_name: &str) -> Result<bool, ConfigError> {
    let value = serde_json::from_str::<Value>(body)?;
    Ok(value
        .get("data")
        .and_then(Value::as_array)
        .is_some_and(|models| {
            models.iter().any(|model| {
                model
                    .get("id")
                    .and_then(Value::as_str)
                    .is_some_and(|id| id == served_model_name)
            })
        }))
}

pub fn build_model_readiness_from_models_response(
    models_endpoint: &str,
    status_code: u16,
    body: &str,
    served_model_names: &[&str],
) -> Result<Vec<(EndpointStatus, bool)>, ConfigError> {
    let value = serde_json::from_str::<Value>(body)?;
    let served = value
        .get("data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|model| model.get("id").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();

    Ok(served_model_names
        .iter()
        .map(|name| {
            (
                EndpointStatus::ok(models_endpoint, status_code),
                served.contains(name),
            )
        })
        .collect())
}

pub fn tail_file_lines(
    path: impl AsRef<Path>,
    line_count: usize,
) -> Result<Vec<String>, ConfigError> {
    let content = fs::read_to_string(path)?;
    let mut lines = content
        .lines()
        .rev()
        .take(line_count)
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    lines.reverse();
    Ok(lines)
}

pub async fn check_endpoint(url: &str) -> EndpointStatus {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    {
        Ok(client) => client,
        Err(err) => return EndpointStatus::unavailable(url, err.to_string()),
    };

    match client.get(url).send().await {
        Ok(response) => {
            let status = response.status();
            if status.is_success() {
                EndpointStatus::ok(url, status.as_u16())
            } else {
                EndpointStatus {
                    url: url.to_owned(),
                    reachable: true,
                    status_code: Some(status.as_u16()),
                    message: format!("http {status}"),
                }
            }
        }
        Err(err) => EndpointStatus::unavailable(url, err.to_string()),
    }
}

async fn check_models_endpoint_for_names(
    models_endpoint: &str,
    served_model_names: &[&str],
) -> Vec<(EndpointStatus, bool)> {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    {
        Ok(client) => client,
        Err(err) => {
            return served_model_names
                .iter()
                .map(|_| {
                    (
                        EndpointStatus::unavailable(models_endpoint, err.to_string()),
                        false,
                    )
                })
                .collect();
        }
    };

    match client.get(models_endpoint).send().await {
        Ok(response) => {
            let status = response.status();
            if !status.is_success() {
                return served_model_names
                    .iter()
                    .map(|_| {
                        (
                            EndpointStatus {
                                url: models_endpoint.to_owned(),
                                reachable: true,
                                status_code: Some(status.as_u16()),
                                message: format!("http {status}"),
                            },
                            false,
                        )
                    })
                    .collect();
            }

            match response.text().await {
                Ok(body) => build_model_readiness_from_models_response(
                    models_endpoint,
                    status.as_u16(),
                    &body,
                    served_model_names,
                )
                .unwrap_or_else(|err| {
                    served_model_names
                        .iter()
                        .map(|_| {
                            (
                                EndpointStatus {
                                    url: models_endpoint.to_owned(),
                                    reachable: true,
                                    status_code: Some(status.as_u16()),
                                    message: err.to_string(),
                                },
                                false,
                            )
                        })
                        .collect()
                }),
                Err(err) => served_model_names
                    .iter()
                    .map(|_| {
                        (
                            EndpointStatus {
                                url: models_endpoint.to_owned(),
                                reachable: true,
                                status_code: Some(status.as_u16()),
                                message: err.to_string(),
                            },
                            false,
                        )
                    })
                    .collect(),
            }
        }
        Err(err) => served_model_names
            .iter()
            .map(|_| {
                (
                    EndpointStatus::unavailable(models_endpoint, err.to_string()),
                    false,
                )
            })
            .collect(),
    }
}

async fn check_model_endpoint_candidates_for_names(
    models_endpoints: &[String],
    served_model_names: &[&str],
) -> Vec<(EndpointStatus, bool)> {
    let mut results = served_model_names
        .iter()
        .map(|_| EndpointStatus::unavailable("unknown", "no model endpoint configured"))
        .map(|status| (status, false))
        .collect::<Vec<_>>();

    for endpoint in models_endpoints {
        let checked = check_models_endpoint_for_names(endpoint, served_model_names).await;
        for (index, candidate) in checked.into_iter().enumerate() {
            if !results[index].1 {
                results[index] = candidate;
            }
        }
        if results.iter().all(|(_, ready)| *ready) {
            break;
        }
    }

    results
}

fn check_model_with_wsl_helper(
    distro: &str,
    user: &str,
    models_endpoint: &str,
    served_model_name: &str,
) -> Option<(EndpointStatus, bool)> {
    let body = run_wsl_vllm_health(distro, user, models_endpoint, served_model_name).ok()?;
    let value = serde_json::from_str::<Value>(&body).ok()?;
    let endpoint = value
        .get("models_endpoint")
        .and_then(Value::as_str)
        .unwrap_or("wsl:vllm-health");
    let ready = vllm_helper_response_ready(&body, served_model_name).unwrap_or(false);
    if ready {
        Some((EndpointStatus::ok(endpoint, 200), true))
    } else {
        Some((
            EndpointStatus::unavailable(endpoint, "WSL vLLM health helper reports not ready"),
            false,
        ))
    }
}

pub fn should_fallback_to_wsl_vllm_helper(endpoint_reachable: bool, ready: bool) -> bool {
    endpoint_reachable && !ready
}

pub async fn collect_read_only_status(
    config_dir: impl AsRef<Path>,
) -> Result<ReadOnlyStatus, ConfigError> {
    let config_dir = config_dir.as_ref();
    let config = load_config_dir(config_dir)?;
    let project_root = config_dir
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    let wsl = run_wsl_list_verbose().ok().and_then(|distros| {
        distros
            .into_iter()
            .find(|distro| distro.name == config.control.wsl.distro)
    });
    let wsl_ips =
        run_wsl_hostname_ips(&config.control.wsl.distro, &config.control.wsl.default_user)
            .unwrap_or_default();
    let hermes = check_endpoint(&config.control.hermes.health_url).await;

    let mut models = Vec::new();
    for runtime in &config.model_runtimes.runtimes {
        let mut endpoints = vec![runtime.models_endpoint.clone()];
        endpoints.extend(
            wsl_ips
                .iter()
                .filter_map(|ip| build_wsl_models_endpoint(&runtime.models_endpoint, ip)),
        );
        endpoints.dedup();
        let served_model_names = runtime
            .variants
            .iter()
            .map(|variant| variant.served_model_name.as_str())
            .collect::<Vec<_>>();
        let readiness =
            check_model_endpoint_candidates_for_names(&endpoints, &served_model_names).await;

        for (variant, (mut endpoint, mut ready)) in runtime.variants.iter().zip(readiness) {
            if should_fallback_to_wsl_vllm_helper(endpoint.reachable, ready) {
                for candidate in &endpoints {
                    if let Some((helper_endpoint, helper_ready)) = check_model_with_wsl_helper(
                        &runtime.wsl_distro,
                        &config.control.wsl.default_user,
                        candidate,
                        &variant.served_model_name,
                    ) {
                        endpoint = helper_endpoint;
                        ready = helper_ready;
                        if ready {
                            break;
                        }
                    }
                }
            }
            models.push(ModelRuntimeSummary {
                runtime_id: runtime.id.clone(),
                variant_id: variant.id.clone(),
                served_model_name: variant.served_model_name.clone(),
                model_root: runtime.model_root.clone(),
                endpoint,
                ready,
            });
        }
    }

    let state = StateSummary {
        state_db_exists: resolve_project_path(&project_root, &config.control.daemon.state_db)
            .exists(),
        audit_db_exists: resolve_project_path(&project_root, &config.control.daemon.audit_db)
            .exists(),
    };

    let overall = summarize_health(wsl.as_ref(), &hermes, &models);

    Ok(ReadOnlyStatus {
        wsl,
        hermes,
        models,
        state,
        overall,
    })
}

fn resolve_project_path(project_root: &Path, configured_path: &str) -> PathBuf {
    let path = PathBuf::from(configured_path);
    if path.is_absolute() {
        path
    } else {
        project_root.join(path)
    }
}

fn summarize_health(
    wsl: Option<&WslDistroStatus>,
    hermes: &EndpointStatus,
    models: &[ModelRuntimeSummary],
) -> HealthStatus {
    let wsl_running = wsl.is_some_and(|status| status.state.eq_ignore_ascii_case("running"));
    let any_model_ready = models.iter().any(|model| model.ready);

    if wsl_running && hermes.reachable && any_model_ready {
        HealthStatus::Ok
    } else if wsl.is_some() || hermes.reachable || any_model_ready {
        HealthStatus::Degraded
    } else {
        HealthStatus::Down
    }
}
