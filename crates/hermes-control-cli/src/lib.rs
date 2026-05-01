use std::path::PathBuf;

use clap::{CommandFactory, Parser, Subcommand};
use hermes_control_core::{collect_read_only_status, load_config_dir};
use hermes_control_types::{EndpointStatus, ModelRuntimeSummary, ProviderConfig, ReadOnlyStatus};

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

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(about = "Show global daemon status")]
    Status,
    #[command(about = "Show aggregated health")]
    Health,
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
}

#[derive(Debug, Subcommand)]
pub enum WslCommand {
    #[command(about = "Show WSL status")]
    Status,
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
        None => {
            let mut command = Cli::command();
            Ok(command.render_long_help().to_string())
        }
    }
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
