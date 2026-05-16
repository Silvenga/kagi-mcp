pub fn format_json<T: serde::Serialize>(response: &T) -> String {
    serde_json::to_string_pretty(response)
        .unwrap_or_else(|e| format!("JSON serialization error: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_format_is_json_then_should_serialize_correctly() {
        let data = serde_json::json!({"key": "value"});
        assert_eq!(format_json(&data), "{\n  \"key\": \"value\"\n}");
    }

    #[test]
    fn when_format_json_with_serializable_struct_then_should_serialize() {
        #[derive(serde::Serialize)]
        struct TestData {
            name: String,
            count: u32,
        }
        let data = TestData {
            name: "test".to_owned(),
            count: 42,
        };
        assert_eq!(
            format_json(&data),
            "{\n  \"name\": \"test\",\n  \"count\": 42\n}"
        );
    }
}
