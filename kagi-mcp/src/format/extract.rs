use crate::format::errors::FormatError;
use askama::Template;
use kagi_api::ExtractResponse;

#[derive(Template)]
#[template(path = "extract_results.md", escape = "none")]
struct ExtractResultsTemplate {
    has_content: bool,
    data_items: Vec<ExtractDataItem>,
    error_items: Vec<ExtractErrorItem>,
}

struct ExtractDataItem {
    url: String,
    has_markdown: bool,
    markdown: String,
}

struct ExtractErrorItem {
    url: String,
    message: String,
}

pub fn format_extract_markdown(response: &ExtractResponse) -> Result<String, FormatError> {
    let mut data_items = Vec::new();
    let mut error_items = Vec::new();
    let mut has_content = false;

    if let Some(data) = &response.data {
        for item in data {
            has_content = true;
            let markdown = item.markdown.clone();
            let has_markdown = markdown.is_some();
            data_items.push(ExtractDataItem {
                url: item.url.clone(),
                has_markdown,
                markdown: markdown.unwrap_or_default(),
            });
        }
    }

    if let Some(errors) = &response.errors {
        for error in errors {
            has_content = true;
            error_items.push(ExtractErrorItem {
                url: error.url.clone(),
                message: error
                    .message
                    .clone()
                    .unwrap_or_else(|| "Unknown error".to_owned()),
            });
        }
    }

    let template = ExtractResultsTemplate {
        has_content,
        data_items,
        error_items,
    };

    template
        .render()
        .map_err(FormatError::TemplateError)
        .map(|s| s.trim_end().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kagi_api::{ExtractData, ExtractError, Meta};

    macro_rules! assert_snapshot {
        ($value:expr) => {
            insta::assert_snapshot!($value.replace("\r\n", "\n"));
        };
    }

    #[test]
    fn when_extract_is_empty_then_should_return_no_content() {
        let response = ExtractResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: None,
            errors: None,
        };
        assert_snapshot!(format_extract_markdown(&response).unwrap());
    }

    #[test]
    fn when_extract_has_empty_arrays_then_should_return_no_content() {
        let response = ExtractResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: Some(vec![]),
            errors: Some(vec![]),
        };
        assert_snapshot!(format_extract_markdown(&response).unwrap());
    }

    #[test]
    fn when_extract_succeeds_then_should_format_content() {
        let response = ExtractResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: Some(vec![ExtractData {
                url: "https://example.com".to_owned(),
                markdown: Some("# Hello\nWorld".to_owned()),
            }]),
            errors: None,
        };
        assert_snapshot!(format_extract_markdown(&response).unwrap());
    }

    #[test]
    fn when_extract_fails_then_should_format_error() {
        let response = ExtractResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: None,
            errors: Some(vec![ExtractError {
                url: "https://example.com".to_owned(),
                code: "404".to_owned(),
                message: Some("Not Found".to_owned()),
            }]),
        };
        assert_snapshot!(format_extract_markdown(&response).unwrap());
    }

    #[test]
    fn when_extract_error_without_message_then_should_use_fallback() {
        let response = ExtractResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: None,
            errors: Some(vec![ExtractError {
                url: "https://example.com/err".to_owned(),
                code: "403".to_owned(),
                message: None,
            }]),
        };
        assert_snapshot!(format_extract_markdown(&response).unwrap());
    }

    #[test]
    fn when_extract_has_data_and_errors_then_should_include_both() {
        let response = ExtractResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: Some(vec![ExtractData {
                url: "https://example.com/ok".to_owned(),
                markdown: Some("Successful content.".to_owned()),
            }]),
            errors: Some(vec![ExtractError {
                url: "https://example.com/bad".to_owned(),
                code: "500".to_owned(),
                message: Some("Server Error".to_owned()),
            }]),
        };
        assert_snapshot!(format_extract_markdown(&response).unwrap());
    }

    #[test]
    fn when_extract_has_multiple_data_items_then_should_show_all() {
        let response = ExtractResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: Some(vec![
                ExtractData {
                    url: "https://example.com/a".to_owned(),
                    markdown: Some("Content A.".to_owned()),
                },
                ExtractData {
                    url: "https://example.com/b".to_owned(),
                    markdown: Some("Content B.".to_owned()),
                },
            ]),
            errors: None,
        };
        assert_snapshot!(format_extract_markdown(&response).unwrap());
    }

    #[test]
    fn when_extract_data_has_no_markdown_then_should_skip_content() {
        let response = ExtractResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: Some(vec![ExtractData {
                url: "https://example.com/empty".to_owned(),
                markdown: None,
            }]),
            errors: None,
        };
        assert_snapshot!(format_extract_markdown(&response).unwrap());
    }

    #[test]
    fn when_extract_has_fallback_message_then_should_render_as_content() {
        let response = ExtractResponse {
            meta: Meta {
                trace: "test".to_owned(),
                node: None,
                ms: None,
            },
            data: Some(vec![ExtractData {
                url: "https://example.com/unavailable".to_owned(),
                markdown: Some(
                    "This URL could not be extracted. The content was not available.".to_owned(),
                ),
            }]),
            errors: None,
        };
        let result = format_extract_markdown(&response).unwrap();
        assert_snapshot!(result);
        assert!(result.contains("This URL could not be extracted"));
    }
}
