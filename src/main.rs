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
                .add_directive("sentry_rs=info".parse()?)
                .add_directive("reqwest::connect=debug".parse()?)
                .add_directive("rmcp=warn".parse()?)
                .add_directive("hyper=warn".parse()?)
                .add_directive("hyper_util=warn".parse()?),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();
    info!("Starting sentry-rs MCP server");
    let tools = SentryTools::new();
    let service = tools.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
