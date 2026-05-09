use rmcp::schemars;
use serde::Deserialize;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ExtractParams {
    #[expect(dead_code)]
    pub pages: Vec<String>,
    #[expect(dead_code)]
    pub timeout: Option<f64>,
    #[expect(dead_code)]
    pub output_format: Option<String>,
}

pub async fn extract_handler(
    params: ExtractParams,
) -> Result<rmcp::model::CallToolResult, rmcp::ErrorData> {
    let _ = params;
    Ok(rmcp::model::CallToolResult::success(vec![
        rmcp::model::Content::text("extract stub"),
    ]))
}
