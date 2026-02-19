use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    tracing::info!("Govrix Platform v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!("scaffold: server stub — no proxy started yet");
    Ok(())
}
