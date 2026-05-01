use clap::Parser;
use hermes_control_cli::{Cli, run_cli};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let output = run_cli(cli).await?;
    println!("{output}");
    Ok(())
}
