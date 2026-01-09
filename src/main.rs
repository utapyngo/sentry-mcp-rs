mod api_client;
mod tools;

use rmcp::{ServiceExt, transport::stdio};
use tools::SentryTools;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::DEBUG.into()),
        )
        .with_writer(std::io::stderr)
        .init();
    info!("Starting sentry-rs MCP server");
    let tools = SentryTools::new();
    let service = tools.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
