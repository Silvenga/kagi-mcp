pub fn error_response_json() -> serde_json::Value {
    serde_json::json!({
        "meta": { "trace": "error-trace" },
        "data": null,
        "error": [
            { "code": "ERR_CODE", "url": "", "message": "Error message", "location": null }
        ]
    })
}
