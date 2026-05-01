#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init()
        .ok();

    let config_dir =
        std::env::var("HERMES_CONTROL_CONFIG_DIR").unwrap_or_else(|_| "config".to_owned());
    let api_token = std::env::var("HERMES_CONTROL_API_TOKEN")
        .map_err(|_| anyhow::anyhow!("missing HERMES_CONTROL_API_TOKEN"))?;
    let config = hermes_control_core::load_config_dir(&config_dir)?;
    let bind = config.control.daemon.bind.clone();
    let router = hermes_control_daemon::build_router(&config_dir, api_token)?;

    let listener = tokio::net::TcpListener::bind(&bind).await?;
    tracing::info!(bind = %bind, "hermes-control-daemon listening");
    axum::serve(listener, router).await?;
    Ok(())
}
