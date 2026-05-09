use std::sync::Arc;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;
use rmcp::{tool, tool_router, ErrorData as McpError};

use kagi_api::client::KagiClient;

use crate::tools::extract::{extract_handler, ExtractParams};
use crate::tools::search::{search_handler, SearchParams};

#[derive(Clone)]
pub struct KagiMcpServer {
    #[expect(dead_code)]
    pub client: Arc<KagiClient>,
}

impl KagiMcpServer {
    pub fn new(client: KagiClient) -> Self {
        Self {
            client: Arc::new(client),
        }
    }
}

#[tool_router(server_handler)]
impl KagiMcpServer {
    #[tool(description = "Search the web using Kagi")]
    async fn search(
        &self,
        Parameters(params): Parameters<SearchParams>,
    ) -> Result<CallToolResult, McpError> {
        search_handler(params).await
    }

    #[tool(description = "Extract clean Markdown from URLs")]
    async fn extract(
        &self,
        Parameters(params): Parameters<ExtractParams>,
    ) -> Result<CallToolResult, McpError> {
        extract_handler(params).await
    }
}

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

        let server = KagiMcpServer::new(client);

        let info = server.get_info();
        assert!(
            info.capabilities.tools.is_some(),
            "server should have tools capability"
        );
    }
}
