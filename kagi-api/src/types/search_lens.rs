use serde::Serialize;

/// A time-relative filter for search results.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TimeRelative {
    /// Filter for the past day.
    Day,
    /// Filter for the past week.
    Week,
    /// Filter for the past month.
    Month,
}

/// Inline description of a lens to apply to the search. Options supplied by
/// the lens take precedence over those supplied by the user in their search
/// terms (e.g., `site:` operators), allowing you to restrict the scope of the
/// search to return more relevant results in specific applications.
#[derive(Debug, Clone, Serialize)]
pub struct Lens {
    /// Search only these domains.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sites_included: Option<Vec<String>>,
    /// Exclude these domains from the search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sites_excluded: Option<Vec<String>>,
    /// Return only results containing these keywords.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords_included: Option<Vec<String>>,
    /// Exclude results containing these keywords.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords_excluded: Option<Vec<String>>,
    /// A specific file type to search for. (e.g., `pdf`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_type: Option<String>,
    /// Filters for web pages that have been updated or published *after* the
    /// given date.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_after: Option<String>,
    /// Filters for web pages that have been updated or published *before* the
    /// given date.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_before: Option<String>,
    /// Filters for web pages that have been updated or published in the given
    /// interval, relative to today's date.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_relative: Option<TimeRelative>,
    /// Requests results localized to a specific region. Can be any valid
    /// ISO-3166-1 Alpha-2 country code, or the special value `no_region`, that
    /// will try to get the most general results possible.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_region: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_lens_minimal_then_should_serialize_to_empty_object() {
        let lens = Lens {
            sites_included: None,
            sites_excluded: None,
            keywords_included: None,
            keywords_excluded: None,
            file_type: None,
            time_after: None,
            time_before: None,
            time_relative: None,
            search_region: None,
        };
        let json = serde_json::to_string(&lens).unwrap();
        assert_eq!(json, r#"{}"#);
    }

    #[test]
    fn when_time_relative_variants_then_should_serialize_lowercase() {
        assert_eq!(
            serde_json::to_string(&TimeRelative::Day).unwrap(),
            r#""day""#
        );
        assert_eq!(
            serde_json::to_string(&TimeRelative::Week).unwrap(),
            r#""week""#
        );
        assert_eq!(
            serde_json::to_string(&TimeRelative::Month).unwrap(),
            r#""month""#
        );
    }

    #[test]
    fn when_lens_fully_populated_then_should_serialize_correctly() {
        let lens = Lens {
            sites_included: Some(vec!["example.com".to_owned()]),
            sites_excluded: Some(vec!["spam.com".to_owned()]),
            keywords_included: Some(vec!["rust".to_owned()]),
            keywords_excluded: Some(vec!["snake".to_owned()]),
            file_type: Some("pdf".to_owned()),
            time_after: Some("2024-01-01".to_owned()),
            time_before: Some("2024-12-31".to_owned()),
            time_relative: Some(TimeRelative::Week),
            search_region: Some("us".to_owned()),
        };
        let json = serde_json::to_string(&lens).unwrap();
        assert!(json.contains(r#""sites_included":["example.com"]"#));
        assert!(json.contains(r#""time_relative":"week""#));
        assert!(json.contains(r#""search_region":"us""#));
    }
}
