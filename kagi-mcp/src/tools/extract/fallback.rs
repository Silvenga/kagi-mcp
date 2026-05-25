//! Fallback rules for URL extraction.
//!
//! Provides [`FallbackRules`] for checking whether a URL should be blocked
//! or have its content replaced with an empty/message response.

use crate::config::FallbackRule;
use crate::tools::domain::extract_registrable_domain;
use kagi_api::ExtractData;

/// The result of checking a URL against fallback rules.
#[derive(Debug, Clone, PartialEq)]
pub enum FallbackMatch {
    /// The URL does not match any fallback rule.
    NoMatch,
    /// The URL matched a rule that replaces content with an empty/message response.
    EmptyContent {
        /// The message to return.
        message: String,
    },
    /// The URL matched a rule that always blocks the request.
    AlwaysBlock {
        /// The message to return.
        message: String,
    },
}

/// A set of fallback rules for URL extraction.
#[derive(Debug, Clone)]
pub struct FallbackRules {
    /// The list of fallback rules.
    pub rules: Vec<FallbackRule>,
}

impl FallbackRules {
    /// Check a URL against the fallback rules.
    ///
    /// Returns the appropriate [`FallbackMatch`] variant based on the matching rule.
    pub fn check(&self, url: &str) -> FallbackMatch {
        let Some(domain) = extract_registrable_domain(url) else {
            return FallbackMatch::NoMatch;
        };

        for rule in &self.rules {
            if rule.domain.eq_ignore_ascii_case(&domain) {
                if rule.always_block {
                    return FallbackMatch::AlwaysBlock {
                        message: rule.message.clone(),
                    };
                }
                if !rule.message.is_empty() {
                    return FallbackMatch::EmptyContent {
                        message: rule.message.clone(),
                    };
                }
                // always_block is false and message is empty — no meaningful action
                return FallbackMatch::NoMatch;
            }
        }

        FallbackMatch::NoMatch
    }
}

/// Check whether extracted content is effectively empty.
///
/// Returns `true` when the markdown field is `None`, empty, or whitespace-only.
pub fn is_empty_content(data: &ExtractData) -> bool {
    data.markdown.as_deref().is_none_or(|s| s.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rule(domain: &str, always_block: bool, message: &str) -> FallbackRule {
        FallbackRule {
            domain: domain.to_owned(),
            always_block,
            message: message.to_owned(),
        }
    }

    fn make_rules(rules: Vec<FallbackRule>) -> FallbackRules {
        FallbackRules { rules }
    }

    // --- check() tests ---

    #[test]
    fn when_url_matches_fallback_rule_then_check_returns_match() {
        let rules = make_rules(vec![make_rule("example.com", false, "blocked")]);
        let result = rules.check("https://www.example.com/page");

        assert_eq!(
            result,
            FallbackMatch::EmptyContent {
                message: "blocked".to_owned()
            }
        );
    }

    #[test]
    fn when_url_does_not_match_then_check_returns_no_match() {
        let rules = make_rules(vec![make_rule("example.com", false, "blocked")]);
        let result = rules.check("https://www.other.com/page");

        assert_eq!(result, FallbackMatch::NoMatch);
    }

    #[test]
    fn when_url_matches_always_block_then_check_returns_always_block() {
        let rules = make_rules(vec![make_rule("example.com", true, "blocked permanently")]);
        let result = rules.check("https://www.example.com/page");

        assert_eq!(
            result,
            FallbackMatch::AlwaysBlock {
                message: "blocked permanently".to_owned()
            }
        );
    }

    #[test]
    fn when_url_matches_case_insensitive_then_check_returns_match() {
        let rules = make_rules(vec![make_rule("Example.COM", false, "blocked")]);
        let result = rules.check("https://www.example.com/page");

        assert_eq!(
            result,
            FallbackMatch::EmptyContent {
                message: "blocked".to_owned()
            }
        );
    }

    #[test]
    fn when_url_has_no_registrable_domain_then_check_returns_no_match() {
        let rules = make_rules(vec![make_rule("example.com", false, "blocked")]);
        let result = rules.check("not-a-valid-url");

        assert_eq!(result, FallbackMatch::NoMatch);
    }

    #[test]
    fn when_rule_has_empty_message_and_not_always_block_then_check_returns_no_match() {
        let rules = make_rules(vec![make_rule("example.com", false, "")]);
        let result = rules.check("https://example.com");

        assert_eq!(result, FallbackMatch::NoMatch);
    }

    // --- is_empty_content() tests ---

    #[test]
    fn is_empty_content_returns_true_for_none_markdown() {
        let data = ExtractData {
            url: "https://example.com".to_owned(),
            markdown: None,
        };

        assert!(is_empty_content(&data));
    }

    #[test]
    fn is_empty_content_returns_true_for_empty_string_markdown() {
        let data = ExtractData {
            url: "https://example.com".to_owned(),
            markdown: Some(String::new()),
        };

        assert!(is_empty_content(&data));
    }

    #[test]
    fn is_empty_content_returns_true_for_whitespace_only_markdown() {
        let data = ExtractData {
            url: "https://example.com".to_owned(),
            markdown: Some("   \n  \t  ".to_owned()),
        };

        assert!(is_empty_content(&data));
    }

    #[test]
    fn is_empty_content_returns_false_for_real_content() {
        let data = ExtractData {
            url: "https://example.com".to_owned(),
            markdown: Some("# Hello\n\nThis is content.".to_owned()),
        };

        assert!(!is_empty_content(&data));
    }
}
