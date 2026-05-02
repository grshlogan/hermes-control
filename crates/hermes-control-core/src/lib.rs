use std::{
    fs, io,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

use hermes_control_types::{
    CommandPreview, ControlConfig, EndpointStatus, HealthStatus, HermesAction, ModelAction,
    ModelRuntimeConfig, ModelRuntimeSummary, ModelRuntimeVariant, ModelRuntimesConfig,
    ProvidersConfig, ReadOnlyStatus, RiskLevel, StateSummary, WslAction, WslDistroStatus,
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

async fn check_model_endpoint(
    models_endpoint: &str,
    served_model_name: &str,
) -> (EndpointStatus, bool) {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    {
        Ok(client) => client,
        Err(err) => {
            return (
                EndpointStatus::unavailable(models_endpoint, err.to_string()),
                false,
            );
        }
    };

    match client.get(models_endpoint).send().await {
        Ok(response) => {
            let status = response.status();
            if !status.is_success() {
                return (
                    EndpointStatus {
                        url: models_endpoint.to_owned(),
                        reachable: true,
                        status_code: Some(status.as_u16()),
                        message: format!("http {status}"),
                    },
                    false,
                );
            }

            match response.text().await {
                Ok(body) => {
                    let ready =
                        models_response_has_model(&body, served_model_name).unwrap_or(false);
                    (EndpointStatus::ok(models_endpoint, status.as_u16()), ready)
                }
                Err(err) => (
                    EndpointStatus {
                        url: models_endpoint.to_owned(),
                        reachable: true,
                        status_code: Some(status.as_u16()),
                        message: err.to_string(),
                    },
                    false,
                ),
            }
        }
        Err(err) => (
            EndpointStatus::unavailable(models_endpoint, err.to_string()),
            false,
        ),
    }
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
    let hermes = check_endpoint(&config.control.hermes.health_url).await;

    let mut models = Vec::new();
    for runtime in &config.model_runtimes.runtimes {
        for variant in &runtime.variants {
            let (endpoint, ready) =
                check_model_endpoint(&runtime.models_endpoint, &variant.served_model_name).await;
            models.push(ModelRuntimeSummary {
                runtime_id: runtime.id.clone(),
                variant_id: variant.id.clone(),
                served_model_name: variant.served_model_name.clone(),
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
