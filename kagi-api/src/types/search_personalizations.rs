use serde::Serialize;

/// Handling mode for a domain personalization rule.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DomainKind {
    /// Block results from this domain entirely.
    Block,
    /// Lower the ranking of results from this domain.
    Lower,
    /// Boost the ranking of results from this domain.
    Raise,
    /// Pin results from this domain to the top.
    Pin,
}

/// A domain-level personalization rule. Each rule can boost or lower the
/// ranking of results from specific domains.
#[derive(Debug, Clone, Serialize)]
pub struct PersonalizationDomain {
    /// Domain pattern to personalize (e.g., "example.com"). Can also be a tld
    /// suffix like ".co.uk".
    pub domain: String,
    /// Handling mode for this domain pattern.
    pub kind: DomainKind,
}

/// A regex-based personalization rule.
#[derive(Debug, Clone, Serialize)]
pub struct PersonalizationRegex {
    /// The regex pattern to match.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regex: Option<String>,
    /// The replacement string to apply when the pattern matches. Will preserve
    /// paths and query parameters if not overwritten. You can refer to capture
    /// groups with "$1", "$2", etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement: Option<String>,
}

/// Personalization rules to customize search result ranking. Allows specifying
/// domain biases and regex-based replacements.
#[derive(Debug, Clone, Serialize)]
pub struct Personalizations {
    /// Domain-level personalization rules (maximum 1000 rules). Each rule can
    /// boost or lower the ranking of results from specific domains.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domains: Option<Vec<PersonalizationDomain>>,
    /// Regex-based personalization rules (maximum 1000 rules, max 1000 bytes
    /// per pattern).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regexes: Option<Vec<PersonalizationRegex>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_domain_kind_variants_then_should_serialize_lowercase() {
        assert_eq!(
            serde_json::to_string(&DomainKind::Block).unwrap(),
            r#""block""#
        );
        assert_eq!(
            serde_json::to_string(&DomainKind::Lower).unwrap(),
            r#""lower""#
        );
        assert_eq!(
            serde_json::to_string(&DomainKind::Raise).unwrap(),
            r#""raise""#
        );
        assert_eq!(serde_json::to_string(&DomainKind::Pin).unwrap(), r#""pin""#);
    }

    #[test]
    fn when_personalization_domain_then_should_serialize_correctly() {
        let domain = PersonalizationDomain {
            domain: "spam.com".to_owned(),
            kind: DomainKind::Block,
        };
        let json = serde_json::to_string(&domain).unwrap();
        assert_eq!(json, r#"{"domain":"spam.com","kind":"block"}"#);
    }

    #[test]
    fn when_personalization_regex_minimal_then_should_serialize_to_empty_object() {
        let regex = PersonalizationRegex {
            regex: None,
            replacement: None,
        };
        let json = serde_json::to_string(&regex).unwrap();
        assert_eq!(json, r#"{}"#);
    }

    #[test]
    fn when_personalization_regex_fully_populated_then_should_serialize_correctly() {
        let regex = PersonalizationRegex {
            regex: Some(r"^https?://(www\.)?reddit\.com.*".to_owned()),
            replacement: Some("https://old.reddit.com".to_owned()),
        };
        let json = serde_json::to_string(&regex).unwrap();
        assert!(json.contains(r#""regex":"#));
        assert!(json.contains(r#""replacement":"https://old.reddit.com""#));
    }

    #[test]
    fn when_personalizations_minimal_then_should_serialize_to_empty_object() {
        let personalizations = Personalizations {
            domains: None,
            regexes: None,
        };
        let json = serde_json::to_string(&personalizations).unwrap();
        assert_eq!(json, r#"{}"#);
    }

    #[test]
    fn when_personalizations_fully_populated_then_should_serialize_correctly() {
        let personalizations = Personalizations {
            domains: Some(vec![PersonalizationDomain {
                domain: "example.com".to_owned(),
                kind: DomainKind::Raise,
            }]),
            regexes: Some(vec![PersonalizationRegex {
                regex: Some(r"^https?://example\.com.*".to_owned()),
                replacement: None,
            }]),
        };
        let json = serde_json::to_string(&personalizations).unwrap();
        assert!(json.contains(r#""domain":"example.com""#));
        assert!(json.contains(r#""kind":"raise""#));
        assert!(json.contains(r#""regex":"#));
    }
}
