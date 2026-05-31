mod cache;
mod config;
mod format;
mod server;
mod tools;

use crate::cache::CacheStore;
use crate::config::{Config, TransportMode};
use crate::tools::extract::FallbackRules;
use axum::Router;
use clap::Parser;
use kagi_api::KagiClientBuilder;
use kagi_mcp::logging::build_subscriber;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::tower::StreamableHttpService;
use rmcp::transport::streamable_http_server::StreamableHttpServerConfig;
use rmcp::ServiceExt;
use server::KagiMcpServer;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{stdin, stdout};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::parse();

    let cache_dir = config
        .resolved_cache_dir()
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let _layers = build_subscriber(
        matches!(config.transport, TransportMode::StreamableHttp),
        &cache_dir,
    )?;

    log_panics::init();

    let mut rules = Vec::new();

    for rule in &config.fallback_always {
        rules.push(rule.clone());
    }

    for rule in &config.fallback_messages {
        if let Some(existing) = rules.iter_mut().find(|r| r.domain == rule.domain) {
            existing.message = rule.message.clone();
        } else {
            rules.push(rule.clone());
        }
    }
    let fallback_rules = if rules.is_empty() {
        None
    } else {
        Some(FallbackRules { rules })
    };

    let client = KagiClientBuilder::new()
        .with_api_key(&config.api_key)
        .with_base_url(&config.base_url)
        .with_timeout_seconds(config.client_timeout)
        .with_retries(config.retries)
        .build()
        .map_err(|e| anyhow::anyhow!("failed to create Kagi client: {e}"))?;

    let cache_store = Arc::new(
        CacheStore::new(&cache_dir, config.cache_size_gb, config.cache_ttl_days)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "failed to initialize cache store");
                anyhow::anyhow!("failed to initialize cache: {e}")
            })?,
    );

    let server = KagiMcpServer::new(client)
        .with_search_timeout(config.search_timeout)
        .with_extract_timeout(config.extract_timeout)
        .with_limit(config.limit)
        .with_safe_search(config.safe_search)
        .with_region(config.region)
        .with_cache_store(Some(cache_store))
        .with_fallback_rules(fallback_rules);

    match config.transport {
        TransportMode::Stdio => {
            let transport = (stdin(), stdout());
            let service = server.serve(transport).await?;
            service.waiting().await?;
        }
        TransportMode::StreamableHttp => {
            let addr: SocketAddr = config.bind.parse()?;
            let listener = TcpListener::bind(addr).await?;
            let session_manager = Arc::new(LocalSessionManager::default());
            let http_config = StreamableHttpServerConfig::default().disable_allowed_hosts();
            let service = StreamableHttpService::new(
                move || -> Result<_, io::Error> { Ok(server.clone()) },
                session_manager,
                http_config,
            );
            let app = Router::new().route_service("/mcp", service);
            axum::serve(listener, app).await?;
        }
    }

    Ok(())
}
