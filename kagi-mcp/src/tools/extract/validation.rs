use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum ValidationError {
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
    #[error("SSRF prevention: URL scheme must be HTTPS")]
    NotHttps,
    #[error("SSRF prevention: private IP addresses are not allowed ({0})")]
    PrivateIp(String),
    #[error("SSRF prevention: localhost is not allowed")]
    Localhost,
    #[error("SSRF prevention: link-local addresses are not allowed ({0})")]
    LinkLocal(String),
    #[error("Pages must be between 1 and 10, got {count}")]
    InvalidPageCount { count: usize },
}

pub fn validate_extract_urls(urls: &[String]) -> Result<Vec<url::Url>, ValidationError> {
    urls.iter().map(|u| validate_url(u)).collect()
}

pub fn validate_extract_pages_count(pages: &[String]) -> Result<(), ValidationError> {
    let count = pages.len();
    if !(1..=10).contains(&count) {
        return Err(ValidationError::InvalidPageCount { count });
    }
    Ok(())
}

fn validate_url(url_str: &str) -> Result<url::Url, ValidationError> {
    let url = url::Url::parse(url_str).map_err(|e| ValidationError::InvalidUrl(e.to_string()))?;

    if url.scheme() != "https" {
        return Err(ValidationError::NotHttps);
    }

    let host = url
        .host()
        .ok_or_else(|| ValidationError::InvalidUrl("missing host".to_owned()))?;

    match host {
        url::Host::Domain(domain) => {
            if domain.eq_ignore_ascii_case("localhost") {
                return Err(ValidationError::Localhost);
            }
            // Hostnames are validated for literal patterns only - no DNS resolution per spec.
        }
        url::Host::Ipv4(v4) => {
            if v4.is_loopback() || v4.is_private() {
                return Err(ValidationError::PrivateIp(v4.to_string()));
            }
            // 169.254.x.x is link-local.
            if v4.is_link_local() {
                return Err(ValidationError::LinkLocal(v4.to_string()));
            }
        }
        url::Host::Ipv6(v6) => {
            if v6.is_loopback() {
                return Err(ValidationError::PrivateIp(v6.to_string()));
            }
            if v6.is_unicast_link_local() {
                return Err(ValidationError::LinkLocal(v6.to_string()));
            }
        }
    }

    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_https_url_is_valid_should_return_parsed_url() {
        let urls = vec![
            "https://example.com".to_owned(),
            "https://kagi.com/api".to_owned(),
        ];

        let result = validate_extract_urls(&urls);

        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.len(), 2);
        // URL parser normalizes by adding trailing slash
        assert_eq!(parsed[0].as_str(), "https://example.com/");
        assert_eq!(parsed[1].as_str(), "https://kagi.com/api");
    }

    #[test]
    fn when_url_has_http_scheme_should_return_not_https_error() {
        let urls = vec!["http://example.com".to_owned()];

        let result = validate_extract_urls(&urls);

        assert_eq!(result, Err(ValidationError::NotHttps));
    }

    #[test]
    fn when_url_has_private_ipv4_10_should_return_private_ip_error() {
        let urls = vec!["https://10.0.0.1/".to_owned()];

        let result = validate_extract_urls(&urls);

        assert_eq!(
            result,
            Err(ValidationError::PrivateIp("10.0.0.1".to_owned()))
        );
    }

    #[test]
    fn when_url_has_private_ipv4_172_16_should_return_private_ip_error() {
        let urls = vec!["https://172.16.0.1/".to_owned()];

        let result = validate_extract_urls(&urls);

        assert_eq!(
            result,
            Err(ValidationError::PrivateIp("172.16.0.1".to_owned()))
        );
    }

    #[test]
    fn when_url_has_private_ipv4_192_168_should_return_private_ip_error() {
        let urls = vec!["https://192.168.1.1/".to_owned()];

        let result = validate_extract_urls(&urls);

        assert_eq!(
            result,
            Err(ValidationError::PrivateIp("192.168.1.1".to_owned()))
        );
    }

    #[test]
    fn when_url_has_loopback_ipv4_should_return_private_ip_error() {
        let urls = vec!["https://127.0.0.1/".to_owned()];

        let result = validate_extract_urls(&urls);

        assert_eq!(
            result,
            Err(ValidationError::PrivateIp("127.0.0.1".to_owned()))
        );
    }

    #[test]
    fn when_url_has_localhost_hostname_should_return_localhost_error() {
        let urls = vec!["https://localhost/".to_owned()];

        let result = validate_extract_urls(&urls);

        assert_eq!(result, Err(ValidationError::Localhost));
    }

    #[test]
    fn when_url_has_link_local_ipv4_should_return_link_local_error() {
        let urls = vec!["https://169.254.0.1/".to_owned()];

        let result = validate_extract_urls(&urls);

        assert_eq!(
            result,
            Err(ValidationError::LinkLocal("169.254.0.1".to_owned()))
        );
    }

    #[test]
    fn when_url_has_link_local_ipv6_should_return_link_local_error() {
        let urls = vec!["https://[fe80::1]/".to_owned()];

        let result = validate_extract_urls(&urls);

        assert_eq!(
            result,
            Err(ValidationError::LinkLocal("fe80::1".to_owned()))
        );
    }

    #[test]
    fn when_multiple_urls_with_first_invalid_should_return_error() {
        let urls = vec![
            "http://example.com".to_owned(),
            "https://kagi.com".to_owned(),
        ];

        let result = validate_extract_urls(&urls);

        assert_eq!(result, Err(ValidationError::NotHttps));
    }

    #[test]
    fn when_multiple_urls_with_second_invalid_should_return_error() {
        let urls = vec![
            "https://kagi.com".to_owned(),
            "http://example.com".to_owned(),
        ];

        let result = validate_extract_urls(&urls);

        assert_eq!(result, Err(ValidationError::NotHttps));
    }

    #[test]
    fn when_url_has_invalid_syntax_should_return_invalid_url_error() {
        let urls = vec!["not-a-url".to_owned()];

        let result = validate_extract_urls(&urls);

        assert!(matches!(result, Err(ValidationError::InvalidUrl(_))));
    }

    #[test]
    fn when_url_missing_host_should_return_invalid_url_error() {
        let urls = vec!["https://".to_owned()];

        let result = validate_extract_urls(&urls);

        assert!(matches!(result, Err(ValidationError::InvalidUrl(_))));
    }

    #[test]
    fn when_extract_pages_count_is_zero_should_return_invalid_page_count_error() {
        let pages: Vec<String> = vec![];

        let result = validate_extract_pages_count(&pages);

        assert_eq!(result, Err(ValidationError::InvalidPageCount { count: 0 }));
    }

    #[test]
    fn when_extract_pages_count_is_one_should_return_ok() {
        let pages = vec!["https://example.com".to_owned()];

        let result = validate_extract_pages_count(&pages);

        assert!(result.is_ok());
    }

    #[test]
    fn when_extract_pages_count_is_ten_should_return_ok() {
        let pages = (1..=10)
            .map(|i| format!("https://example{i}.com"))
            .collect::<Vec<_>>();

        let result = validate_extract_pages_count(&pages);

        assert!(result.is_ok());
    }

    #[test]
    fn when_extract_pages_count_is_eleven_should_return_invalid_page_count_error() {
        let pages = (1..=11)
            .map(|i| format!("https://example{i}.com"))
            .collect::<Vec<_>>();

        let result = validate_extract_pages_count(&pages);

        assert_eq!(result, Err(ValidationError::InvalidPageCount { count: 11 }));
    }
}
