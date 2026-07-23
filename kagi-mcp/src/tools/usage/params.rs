use crate::tools::param_serde::{deserialize_optional_string_non_empty, option_string_schema};
use serde::Deserialize;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UsageParams {
    /// Month to query (format: YYYY-MM). Defaults to current month.
    #[serde(default, deserialize_with = "deserialize_optional_string_non_empty")]
    #[schemars(schema_with = "option_string_schema")]
    pub month: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_usage_params_has_empty_month_then_should_be_none() {
        let json = r#"{"month": ""}"#;
        let params: UsageParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.month, None);
    }

    #[test]
    fn when_usage_params_has_null_month_then_should_be_none() {
        let json = r#"{"month": null}"#;
        let params: UsageParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.month, None);
    }

    #[test]
    fn when_usage_params_omits_month_then_should_be_none() {
        let json = r#"{}"#;
        let params: UsageParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.month, None);
    }

    #[test]
    fn when_usage_params_has_nonempty_month_then_should_keep_value() {
        let json = r#"{"month": "2025-06"}"#;
        let params: UsageParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.month, Some("2025-06".to_owned()));
    }
}
