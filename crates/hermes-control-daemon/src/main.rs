#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init()
        .ok();

    let _router = hermes_control_daemon::build_router();
    tracing::info!("hermes-control-daemon skeleton initialized; serving starts in Phase 3");
    Ok(())
}
