mod config;
pub(crate) mod domain;
pub mod format;
mod guard;
pub mod server;
mod tools;
pub(crate) mod validation;

use clap::Parser;
use config::Config;
use kagi_api::client::KagiClientBuilder;
use rmcp::ServiceExt;
use server::KagiMcpServer;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    let config = Config::parse();

    let client = KagiClientBuilder::new()
        .api_key(&config.api_key)
        .base_url(&config.base_url)
        .timeout(config.client_timeout)
        .retries(config.retries)
        .build()
        .map_err(|e| anyhow::anyhow!("failed to create Kagi client: {e}"))?;

    let server = KagiMcpServer::new(
        client,
        config.kagi_timeout,
        config.limit,
        config.safe_search,
        config.region,
        config.overfetch_multiplier,
        config.overfetch_max,
    );

    let transport = (tokio::io::stdin(), tokio::io::stdout());
    let service = server.serve(transport).await?;
    service.waiting().await?;

    Ok(())
}
