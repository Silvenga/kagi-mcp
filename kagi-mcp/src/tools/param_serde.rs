use serde::Deserialize as _;

/// JSON Schema for an optional string tool parameter.
pub fn option_string_schema(_gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({
        "type": "string"
    })
}

/// JSON Schema for an optional `u32` tool parameter with a minimum value of 1.
pub fn option_positive_u32_schema(_gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({
        "type": "integer",
        "format": "uint32",
        "minimum": 1
    })
}

/// Deserialize an `Option<String>`, coercing empty strings to `None`.
pub fn deserialize_optional_string_non_empty<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    Ok(value.filter(|s| !s.trim().is_empty()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Wrapper {
        #[serde(default, deserialize_with = "deserialize_optional_string_non_empty")]
        field: Option<String>,
    }

    #[test]
    fn when_field_is_empty_string_then_should_be_none() {
        let json = r#"{"field": ""}"#;
        let w: Wrapper = serde_json::from_str(json).unwrap();

        assert_eq!(w.field, None);
    }

    #[test]
    fn when_field_is_null_then_should_be_none() {
        let json = r#"{"field": null}"#;
        let w: Wrapper = serde_json::from_str(json).unwrap();

        assert_eq!(w.field, None);
    }

    #[test]
    fn when_field_is_omitted_then_should_be_none() {
        let json = r#"{}"#;
        let w: Wrapper = serde_json::from_str(json).unwrap();

        assert_eq!(w.field, None);
    }

    #[test]
    fn when_field_is_nonempty_then_should_keep_value() {
        let json = r#"{"field": "news"}"#;
        let w: Wrapper = serde_json::from_str(json).unwrap();

        assert_eq!(w.field, Some("news".to_owned()));
    }

    #[test]
    fn when_field_is_whitespace_then_should_be_none() {
        let json = r#"{"field": " "}"#;
        let w: Wrapper = serde_json::from_str(json).unwrap();

        assert_eq!(w.field, None);
    }
}
