use std::{env, path::PathBuf};

use clap::{Args, CommandFactory, Parser, Subcommand};
use hermes_control_core::{collect_read_only_status, load_config_dir};
use hermes_control_types::{
    ActionRequest, CancelRequest, ConfirmRequest, ConfirmationLifecycleResponse, EndpointStatus,
    HermesAction, ModelAction, ModelRuntimeSummary, OperationResponse, ProviderConfig,
    ReadOnlyStatus, Requester, RequesterChannel, WslAction,
};
use serde::{Serialize, de::DeserializeOwned};

#[derive(Debug, Parser)]
#[command(name = "hermes-control")]
#[command(about = "Thin CLI client for the Hermes Control daemon")]
pub struct Cli {
    #[arg(long, global = true, help = "Emit machine-readable JSON output")]
    pub json: bool,

    #[arg(
        long,
        global = true,
        default_value = "config",
        help = "Directory containing Hermes Control TOML config files"
    )]
    pub config_dir: PathBuf,

    #[arg(
        long,
        global = true,
        default_value = "http://127.0.0.1:18787",
        help = "Hermes Control daemon base URL"
    )]
    pub daemon_url: String,

    #[arg(
        long,
        global = true,
        help = "Bearer token for mutating daemon API calls; can also use HERMES_CONTROL_API_TOKEN"
    )]
    pub api_token: Option<String>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(about = "Show global daemon status")]
    Status,
    #[command(about = "Show aggregated health")]
    Health,
    #[command(subcommand, about = "Operate the Hermes runtime through the daemon")]
    Hermes(HermesCommand),
    #[command(about = "List configured AI providers")]
    Providers,
    #[command(subcommand, about = "Inspect or switch the active AI route")]
    Route(RouteCommand),
    #[command(about = "List local model runtimes")]
    Models,
    #[command(subcommand, about = "Inspect or operate a local model runtime")]
    Model(ModelCommand),
    #[command(subcommand, about = "Inspect or operate the WSL subsystem")]
    Wsl(WslCommand),
    #[command(about = "Confirm a pending destructive operation")]
    Confirm { code: String },
    #[command(about = "Cancel the current pending operation")]
    Cancel,
}

#[derive(Debug, Subcommand)]
pub enum RouteCommand {
    #[command(about = "Show the active route")]
    Active,
}

#[derive(Debug, Subcommand)]
pub enum ModelCommand {
    #[command(about = "Show model runtime status")]
    Status { model_id: String },
    #[command(about = "Tail model runtime logs")]
    Logs { model_id: String },
    #[command(about = "Install or repair the project-owned vLLM runtime")]
    Install(ModelActionArgs),
    #[command(about = "Start a local model runtime through the daemon")]
    Start(ModelActionArgs),
    #[command(about = "Stop a local model runtime through the daemon")]
    Stop(ModelActionArgs),
    #[command(about = "Restart a local model runtime through the daemon")]
    Restart(ModelActionArgs),
    #[command(about = "Check model readiness through the daemon")]
    Health(ModelActionArgs),
    #[command(about = "Run a controlled model benchmark through the daemon")]
    Benchmark(ModelActionArgs),
}

#[derive(Debug, Subcommand)]
pub enum HermesCommand {
    #[command(about = "Wake Hermes through the daemon")]
    Wake(ActionOptions),
    #[command(about = "Stop Hermes through the daemon")]
    Stop(ActionOptions),
    #[command(about = "Restart Hermes through the daemon")]
    Restart(ActionOptions),
    #[command(about = "Kill Hermes through the daemon")]
    Kill(ActionOptions),
}

#[derive(Debug, Subcommand)]
pub enum WslCommand {
    #[command(about = "Show WSL status")]
    Status,
    #[command(about = "Wake the configured WSL distro through the daemon")]
    Wake(ActionOptions),
    #[command(about = "Stop the configured WSL distro through the daemon")]
    Stop(ActionOptions),
    #[command(about = "Restart the configured WSL distro through the daemon")]
    Restart(ActionOptions),
    #[command(about = "Shutdown all WSL distros through the daemon")]
    ShutdownAll(ActionOptions),
}

#[derive(Debug, Clone, Args)]
pub struct ActionOptions {
    #[arg(long, help = "Ask the daemon for a dry-run preview only")]
    pub dry_run: bool,

    #[arg(
        long,
        default_value = "CLI request",
        help = "Operator reason for audit"
    )]
    pub reason: String,
}

#[derive(Debug, Clone, Args)]
pub struct ModelActionArgs {
    pub model_id: String,

    #[command(flatten)]
    pub options: ActionOptions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliOutputFormat {
    Text,
    Json,
}

impl From<bool> for CliOutputFormat {
    fn from(json: bool) -> Self {
        if json { Self::Json } else { Self::Text }
    }
}

pub async fn run_cli(cli: Cli) -> anyhow::Result<String> {
    let format = CliOutputFormat::from(cli.json);
    let daemon = DaemonClientConfig {
        base_url: cli.daemon_url.clone(),
        api_token: cli.api_token.clone(),
    };
    match cli.command {
        Some(Command::Status) | Some(Command::Health) => {
            let status = collect_read_only_status(&cli.config_dir).await?;
            render_status(&status, format)
        }
        Some(Command::Providers) => {
            let config = load_config_dir(&cli.config_dir)?;
            render_providers(&config.providers.providers, format)
        }
        Some(Command::Models) => {
            let status = collect_read_only_status(&cli.config_dir).await?;
            render_models(&status.models, format)
        }
        Some(Command::Route(RouteCommand::Active)) => Ok(match format {
            CliOutputFormat::Json => "{\"active_route\":null}".to_owned(),
            CliOutputFormat::Text => "Active route: unavailable until Phase 3 state DB".to_owned(),
        }),
        Some(Command::Model(ModelCommand::Status { model_id })) => {
            let status = collect_read_only_status(&cli.config_dir).await?;
            let models = status
                .models
                .into_iter()
                .filter(|model| model.variant_id == model_id || model.served_model_name == model_id)
                .collect::<Vec<_>>();
            render_models(&models, format)
        }
        Some(Command::Model(ModelCommand::Logs { model_id })) => Ok(format!(
            "Model logs for {model_id}: log tailing by model id lands in Phase 5"
        )),
        Some(Command::Model(command)) => {
            let (model_id, action, options) = model_action(command);
            let response = daemon
                .post_json::<_, OperationResponse>(
                    &format!("/v1/models/{model_id}/action"),
                    &ActionRequest {
                        requester: cli_requester(),
                        action,
                        reason: options.reason,
                        dry_run: options.dry_run,
                    },
                )
                .await?;
            render_operation_response(&response, format)
        }
        Some(Command::Hermes(command)) => {
            let (action, options) = hermes_action(command);
            let response = daemon
                .post_json::<_, OperationResponse>(
                    "/v1/hermes/action",
                    &ActionRequest {
                        requester: cli_requester(),
                        action,
                        reason: options.reason,
                        dry_run: options.dry_run,
                    },
                )
                .await?;
            render_operation_response(&response, format)
        }
        Some(Command::Wsl(WslCommand::Status)) => {
            let status = collect_read_only_status(&cli.config_dir).await?;
            render_status(
                &ReadOnlyStatus {
                    models: Vec::new(),
                    ..status
                },
                format,
            )
        }
        Some(Command::Wsl(command)) => {
            let (action, options) = wsl_action(command);
            let response = daemon
                .post_json::<_, OperationResponse>(
                    "/v1/wsl/action",
                    &ActionRequest {
                        requester: cli_requester(),
                        action,
                        reason: options.reason,
                        dry_run: options.dry_run,
                    },
                )
                .await?;
            render_operation_response(&response, format)
        }
        Some(Command::Confirm { code }) => {
            let response = daemon
                .post_json::<_, ConfirmationLifecycleResponse>(
                    "/v1/confirm",
                    &ConfirmRequest {
                        requester: cli_requester(),
                        code,
                    },
                )
                .await?;
            render_confirmation_response(&response, format)
        }
        Some(Command::Cancel) => {
            let response = daemon
                .post_json::<_, ConfirmationLifecycleResponse>(
                    "/v1/cancel",
                    &CancelRequest {
                        requester: cli_requester(),
                    },
                )
                .await?;
            render_confirmation_response(&response, format)
        }
        None => {
            let mut command = Cli::command();
            Ok(command.render_long_help().to_string())
        }
    }
}

pub fn render_operation_response(
    response: &OperationResponse,
    format: CliOutputFormat,
) -> anyhow::Result<String> {
    if format == CliOutputFormat::Json {
        return Ok(serde_json::to_string_pretty(response)?);
    }

    let mut lines = vec![
        format!("Status: {}", response.status),
        format!("Risk: {:?}", response.risk),
        format!("Dry run: {}", response.dry_run),
        format!("Summary: {}", response.summary),
    ];

    if let Some(code_hint) = &response.code_hint {
        lines.push(format!("Confirmation code: {code_hint}"));
    }
    if let Some(expires_at) = &response.expires_at {
        lines.push(format!("Expires at: {expires_at}"));
    }
    if !response.commands.is_empty() {
        lines.push("Commands:".to_owned());
        lines.extend(
            response
                .commands
                .iter()
                .map(|command| format!("  {} {}", command.program, command.args.join(" "))),
        );
    }

    Ok(lines.join("\n"))
}

pub fn render_confirmation_response(
    response: &ConfirmationLifecycleResponse,
    format: CliOutputFormat,
) -> anyhow::Result<String> {
    if format == CliOutputFormat::Json {
        return Ok(serde_json::to_string_pretty(response)?);
    }

    let execution = response
        .execution_status
        .as_ref()
        .map(|status| format!("\nExecution: {status}"))
        .unwrap_or_default();
    Ok(format!(
        "Status: {}\nConfirmation: {}\nSummary: {}{}",
        response.status, response.confirmation_id, response.summary, execution
    ))
}

pub fn render_status(status: &ReadOnlyStatus, format: CliOutputFormat) -> anyhow::Result<String> {
    if format == CliOutputFormat::Json {
        return Ok(serde_json::to_string_pretty(status)?);
    }

    let wsl = status
        .wsl
        .as_ref()
        .map(|wsl| {
            format!(
                "WSL: {} {} (v{})",
                wsl.name,
                wsl.state,
                wsl.version.unwrap_or(0)
            )
        })
        .unwrap_or_else(|| "WSL: unavailable".to_owned());
    let hermes = format_endpoint("Hermes", &status.hermes);
    let models = if status.models.is_empty() {
        "Models: none configured".to_owned()
    } else {
        let rendered = status
            .models
            .iter()
            .map(|model| {
                format!(
                    "Model: {}/{}: {}",
                    model.runtime_id,
                    model.variant_id,
                    if model.ready { "ready" } else { "not ready" }
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!("Models:\n{rendered}")
    };

    Ok(format!(
        "Overall: {:?}\n{wsl}\n{hermes}\n{models}\nState DB: {}\nAudit DB: {}",
        status.overall,
        if status.state.state_db_exists {
            "present"
        } else {
            "missing"
        },
        if status.state.audit_db_exists {
            "present"
        } else {
            "missing"
        }
    ))
}

pub fn render_providers(
    providers: &[ProviderConfig],
    format: CliOutputFormat,
) -> anyhow::Result<String> {
    if format == CliOutputFormat::Json {
        return Ok(serde_json::to_string_pretty(providers)?);
    }

    Ok(providers
        .iter()
        .map(|provider| {
            let secret = provider
                .api_key_ref
                .as_ref()
                .map(|secret_ref| format!(" secret_ref={secret_ref}"))
                .unwrap_or_default();
            format!(
                "{} {:?} {}{}",
                provider.id, provider.kind, provider.display_name, secret
            )
        })
        .collect::<Vec<_>>()
        .join("\n"))
}

pub fn render_models(
    models: &[ModelRuntimeSummary],
    format: CliOutputFormat,
) -> anyhow::Result<String> {
    if format == CliOutputFormat::Json {
        return Ok(serde_json::to_string_pretty(models)?);
    }

    if models.is_empty() {
        return Ok("No matching models.".to_owned());
    }

    Ok(models
        .iter()
        .map(|model| {
            format!(
                "{}/{} {} {}",
                model.runtime_id,
                model.variant_id,
                model.served_model_name,
                if model.ready { "ready" } else { "not ready" }
            )
        })
        .collect::<Vec<_>>()
        .join("\n"))
}

fn format_endpoint(label: &str, endpoint: &EndpointStatus) -> String {
    if endpoint.reachable {
        format!(
            "{label}: {} ({})",
            endpoint.message,
            endpoint
                .status_code
                .map(|code| code.to_string())
                .unwrap_or_else(|| "no status".to_owned())
        )
    } else {
        format!("{label}: unavailable ({})", endpoint.message)
    }
}

#[derive(Debug, Clone)]
struct DaemonClientConfig {
    base_url: String,
    api_token: Option<String>,
}

impl DaemonClientConfig {
    async fn post_json<T, R>(&self, path: &str, body: &T) -> anyhow::Result<R>
    where
        T: Serialize + ?Sized,
        R: DeserializeOwned,
    {
        let token = self
            .api_token
            .clone()
            .or_else(|| env::var("HERMES_CONTROL_API_TOKEN").ok())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "mutating CLI calls require --api-token or HERMES_CONTROL_API_TOKEN"
                )
            })?;
        let url = format!("{}{}", self.base_url.trim_end_matches('/'), path);
        let response = reqwest::Client::new()
            .post(url)
            .bearer_auth(token)
            .json(body)
            .send()
            .await?
            .error_for_status()?;

        Ok(response.json::<R>().await?)
    }
}

fn cli_requester() -> Requester {
    Requester {
        channel: RequesterChannel::Cli,
        user_id: env::var("USERNAME")
            .or_else(|_| env::var("USER"))
            .unwrap_or_else(|_| "local-cli".to_owned()),
        chat_id: None,
    }
}

fn hermes_action(command: HermesCommand) -> (HermesAction, ActionOptions) {
    match command {
        HermesCommand::Wake(options) => (HermesAction::Wake, options),
        HermesCommand::Stop(options) => (HermesAction::Stop, options),
        HermesCommand::Restart(options) => (HermesAction::Restart, options),
        HermesCommand::Kill(options) => (HermesAction::Kill, options),
    }
}

fn model_action(command: ModelCommand) -> (String, ModelAction, ActionOptions) {
    match command {
        ModelCommand::Install(args) => (args.model_id, ModelAction::Install, args.options),
        ModelCommand::Start(args) => (args.model_id, ModelAction::Start, args.options),
        ModelCommand::Stop(args) => (args.model_id, ModelAction::Stop, args.options),
        ModelCommand::Restart(args) => (args.model_id, ModelAction::Restart, args.options),
        ModelCommand::Health(args) => (args.model_id, ModelAction::Health, args.options),
        ModelCommand::Benchmark(args) => (args.model_id, ModelAction::Benchmark, args.options),
        ModelCommand::Status { .. } | ModelCommand::Logs { .. } => {
            unreachable!("read-only model commands are handled before action mapping")
        }
    }
}

fn wsl_action(command: WslCommand) -> (WslAction, ActionOptions) {
    match command {
        WslCommand::Wake(options) => (WslAction::Wake, options),
        WslCommand::Stop(options) => (WslAction::StopDistro, options),
        WslCommand::Restart(options) => (WslAction::RestartDistro, options),
        WslCommand::ShutdownAll(options) => (WslAction::ShutdownAll, options),
        WslCommand::Status => unreachable!("status is handled before action mapping"),
    }
}
