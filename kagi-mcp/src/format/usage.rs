use crate::format::errors::FormatError;
use crate::metrics::DailyMetrics;
use askama::Template;

#[derive(Template)]
#[template(path = "usage_results.md", escape = "none")]
struct UsageResultsTemplate {
    month_str: String,
    rows: Vec<UsageRow>,
    totals: UsageTotals,
}

struct UsageRow {
    day: i64,
    extract_reqs: i64,
    search_reqs: i64,
    cached_extracts: i64,
    cached_searches: i64,
    failed_extracts: i64,
}

struct UsageTotals {
    extract_reqs: i64,
    search_reqs: i64,
    cached_extracts: i64,
    cached_searches: i64,
    failed_extracts: i64,
}

pub fn format_usage_markdown(
    month_str: &str,
    daily: &[DailyMetrics],
) -> Result<String, FormatError> {
    let days_in_month = days_in_month_from_str(month_str);
    let mut rows = Vec::new();
    let mut totals = UsageTotals {
        extract_reqs: 0,
        search_reqs: 0,
        cached_extracts: 0,
        cached_searches: 0,
        failed_extracts: 0,
    };

    for day in 1..=days_in_month {
        let metrics = daily.iter().find(|m| m.day == day as i64);
        let row = if let Some(m) = metrics {
            totals.extract_reqs += m.total_extract_requests;
            totals.search_reqs += m.total_search_requests;
            totals.cached_extracts += m.total_extract_urls_from_cache;
            totals.cached_searches += m.total_search_requests_from_cache;
            totals.failed_extracts += m.failed_extract_urls;
            UsageRow {
                day: day as i64,
                extract_reqs: m.total_extract_requests,
                search_reqs: m.total_search_requests,
                cached_extracts: m.total_extract_urls_from_cache,
                cached_searches: m.total_search_requests_from_cache,
                failed_extracts: m.failed_extract_urls,
            }
        } else {
            UsageRow {
                day: day as i64,
                extract_reqs: 0,
                search_reqs: 0,
                cached_extracts: 0,
                cached_searches: 0,
                failed_extracts: 0,
            }
        };
        rows.push(row);
    }

    let template = UsageResultsTemplate {
        month_str: month_str.to_owned(),
        rows,
        totals,
    };

    template
        .render()
        .map_err(FormatError::TemplateError)
        .map(|s| s.trim_end().to_owned())
}

fn days_in_month_from_str(month_str: &str) -> u32 {
    let parts: Vec<&str> = month_str.split('-').collect();
    let year: i32 = parts[0].parse().unwrap_or(2024);
    let month: u32 = parts[1].parse().unwrap_or(1);
    chrono::NaiveDate::from_ymd_opt(year, month, 1)
        .map(|d| {
            let next_month = if month == 12 {
                chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)
            } else {
                chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)
            };
            next_month.map(|n| (n - d).num_days() as u32).unwrap_or(30)
        })
        .unwrap_or(30)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::DailyMetrics;

    macro_rules! assert_snapshot {
        ($value:expr) => {
            insta::assert_snapshot!($value.replace("\r\n", "\n"));
        };
    }

    #[test]
    fn when_usage_no_data_then_should_show_zero_table() {
        let markdown = format_usage_markdown("2024-01", &[]).unwrap();
        assert_snapshot!(markdown);
    }

    #[test]
    fn when_usage_single_day_then_should_show_row_plus_zeros() {
        let daily = vec![DailyMetrics {
            year: 2024,
            month: 1,
            day: 15,
            total_extract_requests: 3,
            total_search_requests: 5,
            total_extract_urls_from_cache: 2,
            total_search_requests_from_cache: 1,
            failed_extract_urls: 0,
        }];
        let markdown = format_usage_markdown("2024-01", &daily).unwrap();
        assert_snapshot!(markdown);
    }

    #[test]
    fn when_usage_partial_month_then_should_show_all_days() {
        let daily = vec![
            DailyMetrics {
                year: 2024,
                month: 1,
                day: 1,
                total_extract_requests: 1,
                total_search_requests: 2,
                total_extract_urls_from_cache: 0,
                total_search_requests_from_cache: 0,
                failed_extract_urls: 0,
            },
            DailyMetrics {
                year: 2024,
                month: 1,
                day: 15,
                total_extract_requests: 5,
                total_search_requests: 10,
                total_extract_urls_from_cache: 3,
                total_search_requests_from_cache: 2,
                failed_extract_urls: 1,
            },
            DailyMetrics {
                year: 2024,
                month: 1,
                day: 31,
                total_extract_requests: 2,
                total_search_requests: 1,
                total_extract_urls_from_cache: 1,
                total_search_requests_from_cache: 0,
                failed_extract_urls: 0,
            },
        ];
        let markdown = format_usage_markdown("2024-01", &daily).unwrap();
        assert_snapshot!(markdown);
    }

    #[test]
    fn when_usage_february_leap_year_then_should_show_29_days() {
        let daily = vec![DailyMetrics {
            year: 2024,
            month: 2,
            day: 29,
            total_extract_requests: 1,
            total_search_requests: 1,
            total_extract_urls_from_cache: 0,
            total_search_requests_from_cache: 0,
            failed_extract_urls: 0,
        }];
        let markdown = format_usage_markdown("2024-02", &daily).unwrap();
        assert_snapshot!(markdown);
    }
}
