use crate::tools::output_format::OutputFormat;
use serde::Deserialize;

/// Parameters for the extract tool.
#[warn(missing_docs)]
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ExtractParams {
    /// HTTPS URLs to extract content from. 1-10 URLs per call.
    pub pages: Vec<String>,
    /// Prefer 'markdown' for human-readable results optimized for LLM consumption.
    /// Use 'json' only when the caller explicitly requests raw structured data.
    #[serde(default)]
    pub output_format: OutputFormat,
    /// Whether to use cached results. Set to false only if freshness is critical.
    #[serde(default = "default_true")]
    #[schemars(default = "default_true")]
    pub cache: bool,
}

pub fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_extract_params_deserialized_without_output_format_then_should_default_to_markdown() {
        let json = r#"{"pages": ["https://example.com"]}"#;
        let params: ExtractParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.output_format, OutputFormat::Markdown);
    }

    #[test]
    fn when_extract_params_deserialized_without_cache_should_default_to_true() {
        let json = r#"{"pages": ["https://example.com"]}"#;
        let params: ExtractParams = serde_json::from_str(json).unwrap();

        assert!(params.cache);
    }

    #[test]
    fn when_extract_params_deserialized_with_cache_false_should_be_false() {
        let json = r#"{"pages": ["https://example.com"], "cache": false}"#;
        let params: ExtractParams = serde_json::from_str(json).unwrap();

        assert!(!params.cache);
    }

    #[test]
    fn when_extract_params_deserialized_with_cache_true_should_be_true() {
        let json = r#"{"pages": ["https://example.com"], "cache": true}"#;
        let params: ExtractParams = serde_json::from_str(json).unwrap();

        assert!(params.cache);
    }
}
