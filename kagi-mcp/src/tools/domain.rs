use publicsuffix::{List, Psl};
use std::str::from_utf8;
use std::sync::LazyLock;
use url::Url;

/// Minimal public suffix data for eTLD+1 extraction.
///
/// Covers common TLDs and well-known second-level public suffixes so that
/// registrable domain extraction works correctly for URLs from popular
/// registries (`.uk`, `.au`, `.jp`, etc.) without shipping the full PSL.
const PSL_DATA: &[u8] = b"\
// BEGIN ICANN DOMAINS\n\
com\norg\nnet\ngov\nedu\nmil\n\
uk\nde\njp\nfr\nau\nbr\n\
co.uk\norg.uk\nac.uk\ngov.uk\nme.uk\nnet.uk\nsch.uk\n\
com.au\nnet.au\norg.au\n\
co.jp\nne.jp\nor.jp\n\
co.nz\nnet.nz\norg.nz\n";

static PSL_LIST: LazyLock<List> =
    LazyLock::new(|| List::from_bytes(PSL_DATA).expect("embedded PSL data is valid"));

/// Extract the registrable domain (eTLD+1) from a URL string.
///
/// Parses the URL, extracts the host, and returns the registrable domain
/// using the embedded Public Suffix List. For IP addresses, returns the
/// address string. Returns `None` when the URL is malformed or has no host.
pub fn extract_registrable_domain(url: &str) -> Option<String> {
    let parsed = Url::parse(url).ok()?;
    let host = parsed.host()?;

    match host {
        url::Host::Ipv4(addr) => Some(addr.to_string()),
        url::Host::Ipv6(addr) => Some(addr.to_string()),
        url::Host::Domain(domain) => {
            let list = &*PSL_LIST;
            match list.domain(domain.as_bytes()) {
                Some(parsed) => {
                    let registrable = from_utf8(parsed.as_bytes())
                        .expect("domain from URL parsing is valid UTF-8");
                    Some(registrable.to_owned())
                }
                None => {
                    let labels: Vec<&str> = domain.split('.').collect();
                    Some(if labels.len() <= 2 {
                        domain.to_owned()
                    } else {
                        labels[labels.len() - 2..].join(".")
                    })
                }
            }
        }
    }
}

/// Compare two registrable domain strings case-insensitively.
///
/// Performs an exact match after lowercasing both inputs. This is a
/// simple string comparison — no wildcard, regex, or subdomain matching.
#[expect(
    clippy::allow_attributes,
    reason = "dead_code is target-dependent; expect would be unfulfilled on lib target"
)]
#[allow(dead_code, reason = "only fires on binary target, not lib")]
pub fn fallback_match(domain: &str, pattern: &str) -> bool {
    domain.to_lowercase() == pattern.to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    mod extract_registrable_domain {
        use super::*;

        #[test]
        fn when_bare_domain_then_return_domain() {
            assert_eq!(
                extract_registrable_domain("https://github.com"),
                Some("github.com".into())
            );
        }

        #[test]
        fn when_www_subdomain_then_return_etld1() {
            assert_eq!(
                extract_registrable_domain("https://www.github.com/page"),
                Some("github.com".into())
            );
        }

        #[test]
        fn when_deep_subdomain_then_return_etld1() {
            assert_eq!(
                extract_registrable_domain("https://docs.github.com/en/rest"),
                Some("github.com".into())
            );
        }

        #[test]
        fn when_public_suffix_like_co_uk_then_handle_correctly() {
            assert_eq!(
                extract_registrable_domain("https://sub.github.co.uk/page"),
                Some("github.co.uk".into())
            );
        }

        #[test]
        fn when_different_domain_then_not_equal() {
            assert_eq!(
                extract_registrable_domain("https://notgithub.com"),
                Some("notgithub.com".into())
            );
        }

        #[test]
        fn when_ipv4_address_then_return_address() {
            assert_eq!(
                extract_registrable_domain("http://192.0.2.1/index.html"),
                Some("192.0.2.1".into())
            );
        }

        #[test]
        fn when_ipv6_address_then_return_address() {
            assert_eq!(
                extract_registrable_domain("http://[::1]/index.html"),
                Some("::1".into())
            );
        }

        #[test]
        fn when_localhost_then_return_localhost() {
            assert_eq!(
                extract_registrable_domain("http://localhost:8080/path"),
                Some("localhost".into())
            );
        }

        #[test]
        fn when_malformed_url_then_return_none() {
            assert_eq!(extract_registrable_domain("not-a-valid-url"), None);
        }

        #[test]
        fn when_no_host_then_return_none() {
            assert_eq!(extract_registrable_domain("mailto:user@example.com"), None);
        }

        #[test]
        fn when_case_insensitive_domain_then_return_lowercase() {
            let result = extract_registrable_domain("https://WWW.GITHUB.COM/repo");
            assert_eq!(result, Some("github.com".into()));
        }
    }

    mod fallback_match {
        use super::*;

        #[test]
        fn when_exact_match_then_return_true() {
            assert!(fallback_match("github.com", "github.com"));
        }

        #[test]
        fn when_subdomain_vs_bare_then_return_false() {
            assert!(!fallback_match("www.github.com", "github.com"));
        }

        #[test]
        fn when_different_domains_then_return_false() {
            assert!(!fallback_match("github.com", "example.com"));
        }

        #[test]
        fn when_case_differs_then_return_true() {
            assert!(fallback_match("GitHub.com", "github.com"));
        }

        #[test]
        fn when_both_uppercase_then_return_true() {
            assert!(fallback_match("GITHUB.COM", "GITHUB.COM"));
        }

        #[test]
        fn when_ip_address_patterns_then_exact_match() {
            assert!(fallback_match("192.0.2.1", "192.0.2.1"));
            assert!(!fallback_match("192.0.2.1", "192.0.2.2"));
        }
    }
}
