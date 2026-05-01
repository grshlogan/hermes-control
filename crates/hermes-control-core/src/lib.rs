use std::{
    fs, io,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

use hermes_control_types::{
    CommandPreview, ControlConfig, EndpointStatus, HealthStatus, HermesAction, ModelRuntimeSummary,
    ModelRuntimesConfig, ProvidersConfig, ReadOnlyStatus, RiskLevel, StateSummary, WslAction,
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
            default_user: "hermes".to_owned(),
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
}

impl HermesRuntimeController {
    pub fn new(agent_root: impl Into<String>, health_url: impl Into<String>) -> Self {
        Self {
            agent_root: agent_root.into(),
            health_url: health_url.into(),
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
                commands: Vec::new(),
                requires_confirmation: false,
            },
            HermesAction::Stop => OperationPlan {
                risk: RiskLevel::Destructive,
                summary: format!("Stop Hermes runtime at {}.", self.agent_root),
                commands: Vec::new(),
                requires_confirmation: true,
            },
            HermesAction::Restart => OperationPlan {
                risk: RiskLevel::Destructive,
                summary: format!(
                    "Restart Hermes runtime at {} and verify health at {}.",
                    self.agent_root, self.health_url
                ),
                commands: Vec::new(),
                requires_confirmation: true,
            },
            HermesAction::Kill => OperationPlan {
                risk: RiskLevel::Destructive,
                summary: format!(
                    "Kill Hermes runtime at {}. Daemon and last-known-good route stay outside the runtime.",
                    self.agent_root
                ),
                commands: Vec::new(),
                requires_confirmation: true,
            },
        }
    }
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
