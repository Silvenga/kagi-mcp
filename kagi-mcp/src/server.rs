use std::sync::Arc;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::service::RequestContext;
use rmcp::RoleServer;
use rmcp::{tool, tool_handler, tool_router, ErrorData as McpError};
use kagi_api::client::KagiClient;
use kagi_api::KagiApi;
use crate::tools::extract::{extract_handler, ExtractParams};
use crate::tools::search::{search_handler, SearchConfig, SearchParams};

#[derive(Clone)]
pub struct KagiMcpServer {
    pub client: Arc<dyn KagiApi>,
    pub kagi_timeout: f64,
    pub limit: u32,
    pub safe_search: bool,
    pub region: Option<String>,
}

impl KagiMcpServer {
    pub fn new(client: KagiClient, kagi_timeout: f64, limit: u32, safe_search: bool, region: Option<String>) -> Self {
        Self {
            client: Arc::new(client),
            kagi_timeout,
            limit,
            safe_search,
            region,
        }
    }

    #[cfg(test)]
    pub fn with_client(client: Arc<dyn KagiApi>) -> Self {
        Self {
            client,
            kagi_timeout: 4.0,
            limit: 10,
            safe_search: true,
            region: None,
        }
    }
}

#[tool_router(vis = "pub")]
impl KagiMcpServer {
    #[tool(description = "Search the web using Kagi")]
    async fn search(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(params): Parameters<SearchParams>,
    ) -> Result<CallToolResult, McpError> {
        let config = SearchConfig {
            kagi_timeout: self.kagi_timeout,
            limit: self.limit,
            safe_search: self.safe_search,
            region: self.region.clone(),
        };
        search_handler(&*self.client, params, &ctx, &config).await
    }

    #[tool(description = "Extract clean Markdown from URLs")]
    async fn extract(
        &self,
        ctx: RequestContext<RoleServer>,
        Parameters(params): Parameters<ExtractParams>,
    ) -> Result<CallToolResult, McpError> {
        extract_handler(&*self.client, params, &ctx, self.kagi_timeout).await
    }
}

#[tool_handler(name = "Kagi", router = Self::tool_router())]
impl rmcp::ServerHandler for KagiMcpServer {}

#[cfg(test)]
mod tests {
    use super::*;
    use kagi_api::client::KagiClientBuilder;
    use rmcp::ServerHandler;

    #[test]
    fn when_server_created_then_tools_should_be_registered() {
        let client = KagiClientBuilder::new()
            .api_key("test-key")
            .build()
            .unwrap();

        let server = KagiMcpServer::new(client, 4.0, 10, true, None);

        let info = server.get_info();
        assert!(
            info.capabilities.tools.is_some(),
            "server should have tools capability"
        );
    }

    #[test]
    fn when_mock_client_provided_then_server_should_accept_it() {
        let mock = kagi_api::MockKagiApi::new();
        let server = KagiMcpServer::with_client(Arc::new(mock));

        let info = server.get_info();
        assert!(
            info.capabilities.tools.is_some(),
            "server should have tools capability"
        );
    }
}
