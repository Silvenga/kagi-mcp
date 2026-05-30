use crate::tools::output_format::OutputFormat;
use serde::Deserialize;

fn option_string_schema(_gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({
        "type": "string"
    })
}

fn option_u32_schema(_gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({
        "type": "integer",
        "format": "uint32",
        "minimum": 1
    })
}

/// Parameters for the `search` tool.
#[warn(missing_docs)]
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchParams {
    /// Search query. Supports Kagi operators: site:, "exact phrases", -negation, inurl:.
    pub query: String,
    /// Result type filter. Use 'images', 'videos', 'news', or 'podcasts' to narrow results.
    /// Omit for general web search.
    #[serde(default)]
    #[schemars(schema_with = "option_string_schema")]
    pub workflow: Option<String>,
    /// Date filter (YYYY-MM-DD). Use when the query is time-sensitive.
    #[serde(default)]
    #[schemars(schema_with = "option_string_schema")]
    pub after: Option<String>,
    /// Date filter (YYYY-MM-DD). Use when the query is time-sensitive.
    #[serde(default)]
    #[schemars(schema_with = "option_string_schema")]
    pub before: Option<String>,
    /// Prefer 'markdown' for human-readable results optimized for LLM consumption.
    /// Use 'json' only when the caller explicitly requests raw structured data.
    #[serde(default)]
    pub output_format: OutputFormat,
    /// Max results per domain group. Use when results feel repetitive from the same site.
    /// Must be >= 1 if set.
    #[serde(default)]
    #[schemars(schema_with = "option_u32_schema")]
    pub limit_per_domain: Option<u32>,
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
    fn when_search_params_deserialized_without_cache_should_default_to_true() {
        let json = r#"{"query": "test"}"#;
        let params: SearchParams = serde_json::from_str(json).unwrap();

        assert!(params.cache);
    }

    #[test]
    fn when_search_params_deserialized_with_cache_false_should_be_false() {
        let json = r#"{"query": "test", "cache": false}"#;
        let params: SearchParams = serde_json::from_str(json).unwrap();

        assert!(!params.cache);
    }

    #[test]
    fn when_search_params_deserialized_with_cache_true_should_be_true() {
        let json = r#"{"query": "test", "cache": true}"#;
        let params: SearchParams = serde_json::from_str(json).unwrap();

        assert!(params.cache);
    }

    #[test]
    fn when_search_params_deserialized_without_output_format_then_should_default_to_markdown() {
        let json = r#"{"query": "test"}"#;
        let params: SearchParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.output_format, OutputFormat::Markdown);
    }
}
