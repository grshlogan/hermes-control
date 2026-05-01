use hermes_control_bot::{BotConfig, run_bot};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init()
        .ok();

    let config = BotConfig::from_env()?;
    run_bot(config).await
}
