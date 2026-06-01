use crate::format::format_usage_markdown;
use crate::metrics::MetricsStore;
use crate::tools::usage::UsageParams;
use chrono::Datelike;
use rmcp::model::{CallToolResult, Content, ErrorData};

pub async fn usage_handler(
    metrics_store: Option<&MetricsStore>,
    params: UsageParams,
) -> Result<CallToolResult, ErrorData> {
    let (year, month) = if let Some(month_str) = &params.month {
        if !regex::Regex::new(r"^\d{4}-\d{2}$")
            .unwrap()
            .is_match(month_str)
        {
            return Err(ErrorData::invalid_params(
                "month must be in YYYY-MM format",
                None,
            ));
        }
        let parts: Vec<&str> = month_str.split('-').collect();
        let year = parts[0]
            .parse::<u32>()
            .map_err(|_| ErrorData::invalid_params("Invalid year in month", None))?;
        let month = parts[1]
            .parse::<u32>()
            .map_err(|_| ErrorData::invalid_params("Invalid month in month", None))?;
        if !(1..=12).contains(&month) {
            return Err(ErrorData::invalid_params(
                "month must be between 01 and 12",
                None,
            ));
        }
        (year, month)
    } else {
        let now = chrono::Utc::now();
        (now.year() as u32, now.month())
    };

    let metrics_store = match metrics_store {
        Some(ms) => ms,
        None => {
            return Err(ErrorData::internal_error("Metrics not available", None));
        }
    };

    let daily_metrics = metrics_store
        .get_monthly_metrics(year, month)
        .await
        .map_err(|e| ErrorData::internal_error(format!("Failed to get metrics: {e}"), None))?;

    let month_str = format!("{:04}-{:02}", year, month);
    let markdown = format_usage_markdown(&month_str, &daily_metrics)
        .map_err(|e| ErrorData::internal_error(format!("Failed to format metrics: {e}"), None))?;

    Ok(CallToolResult::success(vec![Content::text(markdown)]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::MetricsStore;

    #[tokio::test]
    async fn when_usage_with_valid_month_then_should_return_markdown_table() {
        let store = MetricsStore::open_in_memory().await.unwrap();
        store.increment_search_request().await;

        let params = UsageParams {
            month: Some("2024-01".to_owned()),
        };
        let result = usage_handler(Some(&store), params).await.unwrap();
        let text = result.content[0].as_text().unwrap().text.clone();
        assert!(text.contains("Usage for 2024-01"));
        assert!(text.contains("| Day |"));
    }

    #[tokio::test]
    async fn when_usage_without_month_then_should_default_to_current_month() {
        let store = MetricsStore::open_in_memory().await.unwrap();
        store.increment_search_request().await;

        let params = UsageParams { month: None };
        let result = usage_handler(Some(&store), params).await.unwrap();
        let text = result.content[0].as_text().unwrap().text.clone();
        assert!(text.contains("Usage for"));
    }

    #[tokio::test]
    async fn when_usage_with_invalid_month_format_then_should_return_error() {
        let store = MetricsStore::open_in_memory().await.unwrap();

        let params = UsageParams {
            month: Some("invalid".to_owned()),
        };
        let result = usage_handler(Some(&store), params).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn when_usage_with_month_13_then_should_return_error() {
        let store = MetricsStore::open_in_memory().await.unwrap();

        let params = UsageParams {
            month: Some("2024-13".to_owned()),
        };
        let result = usage_handler(Some(&store), params).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn when_metrics_store_none_then_should_return_error() {
        let params = UsageParams { month: None };
        let result = usage_handler(None, params).await;
        assert!(result.is_err());
    }
}
