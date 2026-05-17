pub(crate) mod cache;
mod config;
pub(crate) mod domain;
pub mod format;
mod guard;
pub mod server;
mod tools;
pub(crate) mod validation;

use crate::cache::store::CacheStore;
use clap::Parser;
use config::Config;
use kagi_api::KagiClientBuilder;
use rmcp::ServiceExt;
use server::KagiMcpServer;
use std::io::stderr;
use std::sync::Arc;
use tokio::io::{stdin, stdout};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(stderr)
        .init();

    let config = Config::parse();

    let client = KagiClientBuilder::new()
        .with_api_key(&config.api_key)
        .with_base_url(&config.base_url)
        .with_timeout_seconds(config.client_timeout)
        .with_retries(config.retries)
        .build()
        .map_err(|e| anyhow::anyhow!("failed to create Kagi client: {e}"))?;

    let cache_store = Arc::new(
        CacheStore::new(
            &config.cache_dir,
            config.cache_size_gb,
            config.cache_ttl_days,
        )
        .await
        .map_err(|e| anyhow::anyhow!("failed to initialize cache: {e}"))?,
    );

    let server = KagiMcpServer::new(
        client,
        config.search_timeout,
        config.extract_timeout,
        config.limit,
        config.safe_search,
        config.region,
        config.split_extract_requests,
        Some(cache_store),
    );

    let transport = (stdin(), stdout());
    let service = server.serve(transport).await?;
    service.waiting().await?;

    Ok(())
}
