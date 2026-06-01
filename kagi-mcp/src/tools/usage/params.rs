use serde::Deserialize;

fn option_string_schema(_gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({
        "type": "string"
    })
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UsageParams {
    /// Month to query (format: YYYY-MM). Defaults to current month.
    #[serde(default)]
    #[schemars(schema_with = "option_string_schema")]
    pub month: Option<String>,
}
