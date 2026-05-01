use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "hermes-control")]
#[command(about = "Thin CLI client for the Hermes Control daemon")]
pub struct Cli {
    #[arg(long, global = true, help = "Emit machine-readable JSON output")]
    pub json: bool,

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
    #[command(about = "Switch to a configured route profile")]
    Switch { profile_id: String },
}

#[derive(Debug, Subcommand)]
pub enum ModelCommand {
    #[command(about = "Show model runtime status")]
    Status { model_id: String },
    #[command(about = "Start a model runtime through the daemon")]
    Start { model_id: String },
    #[command(about = "Stop a model runtime through the daemon")]
    Stop { model_id: String },
    #[command(about = "Restart a model runtime through the daemon")]
    Restart { model_id: String },
    #[command(about = "Tail model runtime logs")]
    Logs { model_id: String },
}

#[derive(Debug, Subcommand)]
pub enum WslCommand {
    #[command(about = "Show WSL status")]
    Status,
    #[command(about = "Wake the configured WSL distro")]
    Wake,
    #[command(about = "Restart the configured WSL distro")]
    Restart,
}
