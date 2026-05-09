use rmcp::schemars;
use serde::Deserialize;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchParams {
    #[expect(dead_code)]
    pub query: String,
    #[expect(dead_code)]
    pub workflow: Option<String>,
    #[expect(dead_code)]
    pub after: Option<String>,
    #[expect(dead_code)]
    pub before: Option<String>,
    #[expect(dead_code)]
    pub output_format: Option<String>,
}

pub async fn search_handler(
    params: SearchParams,
) -> Result<rmcp::model::CallToolResult, rmcp::ErrorData> {
    let _ = params;
    Ok(rmcp::model::CallToolResult::success(vec![
        rmcp::model::Content::text("search stub"),
    ]))
}
