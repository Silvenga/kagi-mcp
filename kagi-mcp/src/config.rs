use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub struct FallbackRule {
    pub domain: String,
    pub message: String,
    pub always_block: bool,
}

#[derive(Debug, Parser, Clone)]
#[command(name = "kagi-mcp", about = "Kagi MCP server")]
pub struct Config {
    #[arg(long, env = "KAGI_API_KEY")]
    pub api_key: String,

    #[arg(long, env = "KAGI_BASE_URL", default_value = "https://kagi.com/api")]
    pub base_url: String,

    #[arg(long, env = "KAGI_SEARCH_TIMEOUT", default_value = "4.0")]
    pub search_timeout: f64,

    #[arg(long, env = "KAGI_EXTRACT_TIMEOUT", default_value = "10.0")]
    pub extract_timeout: f64,

    #[arg(long, env = "KAGI_CLIENT_TIMEOUT", default_value = "12.0")]
    pub client_timeout: f64,

    #[arg(long, env = "KAGI_RETRIES", default_value = "3")]
    pub retries: u32,

    #[arg(long, env = "KAGI_LIMIT", default_value = "10")]
    pub limit: u32,

    #[arg(
        long,
        env = "KAGI_SAFE_SEARCH",
        default_missing_value = "true",
        num_args = 0..=1,
        default_value_t = true,
    )]
    pub safe_search: bool,

    #[arg(
        long,
        env = "KAGI_SPLIT_EXTRACT_REQUESTS",
        default_missing_value = "true",
        num_args = 0..=1,
        default_value_t = true,
    )]
    pub split_extract_requests: bool,

    #[arg(long, env = "KAGI_REGION", value_parser = parse_region)]
    pub region: Option<String>,

    #[arg(
        long,
        env = "KAGI_CACHE_DIR",
        value_parser = parse_cache_dir
    )]
    pub cache_dir: Option<PathBuf>,

    #[arg(
        long,
        env = "KAGI_CACHE_SIZE_GB",
        default_value = "5.0",
        value_parser = parse_cache_size_gb,
    )]
    pub cache_size_gb: f64,

    #[arg(
        long,
        env = "KAGI_CACHE_TTL_DAYS",
        default_value = "7",
        value_parser = parse_cache_ttl_days,
    )]
    pub cache_ttl_days: u64,

    #[arg(long, env = "KAGI_TRANSPORT", value_enum, default_value_t = TransportMode::Stdio)]
    pub transport: TransportMode,

    #[arg(long, env = "KAGI_BIND", default_value = "127.0.0.1:3000")]
    pub bind: String,

    #[arg(
        long,
        env = "KAGI_STATELESS_JSON",
        default_missing_value = "true",
        num_args = 0..=1,
        default_value_t = false,
    )]
    pub stateless_json: bool,

    #[arg(
        long = "fallback-message",
        env = "KAGI_FALLBACK_MESSAGE",
        value_parser = parse_fallback_message,
        value_delimiter = ',',
    )]
    pub fallback_messages: Vec<FallbackRule>,

    #[arg(
        long = "fallback-always",
        env = "KAGI_FALLBACK_ALWAYS",
        value_parser = parse_fallback_always,
        value_delimiter = ',',
    )]
    pub fallback_always: Vec<FallbackRule>,
}

impl Config {
    pub fn resolved_cache_dir(&self) -> Result<PathBuf, String> {
        match &self.cache_dir {
            Some(path) => Ok(path.clone()),
            None => {
                dirs::cache_dir()
                    .ok_or(
                        "unable to determine platform cache directory; set --cache-dir or KAGI_CACHE_DIR"
                            .to_owned(),
                    )
                    .map(|p| p.join("kagi-mcp"))
            }
        }
    }
}

#[derive(Debug, Clone, ValueEnum, Default)]
pub enum TransportMode {
    #[default]
    Stdio,
    StreamableHttp,
}

fn parse_cache_dir(s: &str) -> Result<PathBuf, String> {
    let expanded = shellexpand::tilde(s);
    Ok(PathBuf::from(expanded.as_ref()))
}

fn parse_region(s: &str) -> Result<String, String> {
    Ok(s.to_lowercase())
}

fn parse_cache_size_gb(s: &str) -> Result<f64, String> {
    let val: f64 = s
        .parse()
        .map_err(|_| format!("`{s}` is not a valid float"))?;
    if val <= 0.0 {
        Err(format!("cache size must be positive, got {val}"))
    } else {
        Ok(val)
    }
}

fn parse_fallback_message(s: &str) -> Result<FallbackRule, String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err("fallback message entry must not be empty".to_owned());
    }
    let eq_pos = trimmed.find('=').ok_or_else(|| {
        format!("invalid fallback message format `{trimmed}`, expected `domain=message`")
    })?;
    let domain = trimmed[..eq_pos].trim().to_owned();
    let message = trimmed[eq_pos + 1..].trim().to_owned();
    if domain.is_empty() {
        return Err("domain must not be empty in fallback message".to_owned());
    }
    if message.is_empty() {
        return Err("message must not be empty in fallback message".to_owned());
    }
    Ok(FallbackRule {
        domain,
        message,
        always_block: false,
    })
}

fn parse_fallback_always(s: &str) -> Result<FallbackRule, String> {
    let domain = s.trim().to_owned();
    if domain.is_empty() {
        return Err("domain must not be empty in fallback always".to_owned());
    }
    Ok(FallbackRule {
        domain,
        message: String::new(),
        always_block: true,
    })
}

fn parse_cache_ttl_days(s: &str) -> Result<u64, String> {
    let val: u64 = s
        .parse()
        .map_err(|_| format!("`{s}` is not a valid integer"))?;
    if val == 0 {
        Err(format!("cache TTL must be positive, got {val}"))
    } else {
        Ok(val)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_default_args_then_default_values_should_apply() {
        let config = Config::try_parse_from(["kagi-mcp", "--api-key", "test-key"]).unwrap();

        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.base_url, "https://kagi.com/api");
        assert_eq!(config.search_timeout, 4.0);
        assert_eq!(config.extract_timeout, 10.0);
        assert_eq!(config.client_timeout, 12.0);
        assert_eq!(config.retries, 3);
        assert_eq!(config.limit, 10);
        assert!(config.safe_search);
        assert!(config.split_extract_requests);
        assert_eq!(config.region, None);
        assert!(config.cache_dir.is_none());
        assert_eq!(config.cache_size_gb, 5.0);
        assert_eq!(config.cache_ttl_days, 7);
    }

    #[test]
    fn when_all_args_provided_then_custom_values_should_apply() {
        let config = Config::try_parse_from([
            "kagi-mcp",
            "--api-key",
            "custom-key",
            "--base-url",
            "https://custom.example.com",
            "--search-timeout",
            "8.5",
            "--extract-timeout",
            "30.0",
            "--client-timeout",
            "35.0",
            "--retries",
            "5",
            "--limit",
            "25",
            "--safe-search",
            "false",
            "--split-extract-requests",
            "false",
            "--region",
            "us-west",
            "--cache-dir",
            "/custom/cache/dir",
            "--cache-size-gb",
            "10.0",
            "--cache-ttl-days",
            "14",
        ])
        .unwrap();

        assert_eq!(config.api_key, "custom-key");
        assert_eq!(config.base_url, "https://custom.example.com");
        assert_eq!(config.search_timeout, 8.5);
        assert_eq!(config.extract_timeout, 30.0);
        assert_eq!(config.client_timeout, 35.0);
        assert_eq!(config.retries, 5);
        assert_eq!(config.limit, 25);
        assert!(!config.safe_search);
        assert!(!config.split_extract_requests);
        assert_eq!(config.region.as_deref(), Some("us-west"));
        assert_eq!(config.cache_dir, Some(PathBuf::from("/custom/cache/dir")));
        assert_eq!(config.cache_size_gb, 10.0);
        assert_eq!(config.cache_ttl_days, 14);
    }

    #[test]
    fn when_region_is_uppercase_then_should_be_lowercased() {
        let config =
            Config::try_parse_from(["kagi-mcp", "--api-key", "test-key", "--region", "US-WEST"])
                .unwrap();

        assert_eq!(config.region.as_deref(), Some("us-west"));
    }

    #[test]
    fn when_missing_api_key_then_parse_should_fail() {
        let result = Config::try_parse_from(["kagi-mcp"]);
        assert!(result.is_err());
    }

    #[test]
    fn when_negative_cache_size_gb_provided_then_parse_should_fail() {
        let result = Config::try_parse_from([
            "kagi-mcp",
            "--api-key",
            "test-key",
            "--cache-size-gb",
            "-1.0",
        ]);

        assert!(result.is_err());
    }

    #[test]
    fn when_invalid_cache_size_gb_provided_then_parse_should_fail() {
        let result = Config::try_parse_from([
            "kagi-mcp",
            "--api-key",
            "test-key",
            "--cache-size-gb",
            "not-a-float",
        ]);

        assert!(result.is_err());
    }

    #[test]
    fn when_negative_cache_ttl_days_provided_then_parse_should_fail() {
        let result = Config::try_parse_from([
            "kagi-mcp",
            "--api-key",
            "test-key",
            "--cache-ttl-days",
            "-1",
        ]);

        assert!(result.is_err());
    }

    #[test]
    fn when_invalid_cache_ttl_days_provided_then_parse_should_fail() {
        let result = Config::try_parse_from([
            "kagi-mcp",
            "--api-key",
            "test-key",
            "--cache-ttl-days",
            "not-an-integer",
        ]);

        assert!(result.is_err());
    }

    #[test]
    fn when_zero_cache_size_gb_provided_then_parse_should_fail() {
        let result =
            Config::try_parse_from(["kagi-mcp", "--api-key", "test-key", "--cache-size-gb", "0"]);

        assert!(result.is_err());
    }

    #[test]
    fn when_split_extract_requests_cli_flag_false_then_should_override_default() {
        let config = Config::try_parse_from([
            "kagi-mcp",
            "--api-key",
            "test-key",
            "--split-extract-requests",
            "false",
        ])
        .unwrap();

        assert!(!config.split_extract_requests);
    }

    #[test]
    fn when_split_extract_requests_flag_provided_without_value_then_should_default_to_true() {
        let config = Config::try_parse_from([
            "kagi-mcp",
            "--api-key",
            "test-key",
            "--split-extract-requests",
        ])
        .unwrap();

        assert!(config.split_extract_requests);
    }

    #[test]
    fn when_cache_size_gb_default_then_non_negative_should_apply() {
        let config = Config::try_parse_from([
            "kagi-mcp",
            "--api-key",
            "test-key",
            "--cache-size-gb",
            "0.5",
        ])
        .unwrap();

        assert_eq!(config.cache_size_gb, 0.5);
    }

    #[test]
    fn when_transport_streamable_http_then_should_parse() {
        let config = Config::try_parse_from([
            "kagi-mcp",
            "--api-key",
            "test-key",
            "--transport",
            "streamable-http",
        ])
        .unwrap();

        assert!(matches!(config.transport, TransportMode::StreamableHttp));
    }

    #[test]
    fn when_bind_custom_then_should_parse() {
        let config = Config::try_parse_from([
            "kagi-mcp",
            "--api-key",
            "test-key",
            "--bind",
            "0.0.0.0:8080",
        ])
        .unwrap();

        assert_eq!(config.bind, "0.0.0.0:8080");
    }

    #[test]
    fn when_no_cache_dir_then_resolved_default_should_match_dirs() {
        let config = Config::try_parse_from(["kagi-mcp", "--api-key", "test-key"]).unwrap();
        let resolved = config.resolved_cache_dir().unwrap();
        let expected = dirs::cache_dir().unwrap().join("kagi-mcp");
        assert_eq!(resolved, expected);
    }

    #[test]
    fn when_cache_dir_provided_then_resolved_should_return_override() {
        let config = Config::try_parse_from([
            "kagi-mcp",
            "--api-key",
            "test-key",
            "--cache-dir",
            "/custom/cache/dir",
        ])
        .unwrap();
        let resolved = config.resolved_cache_dir().unwrap();
        assert_eq!(resolved, PathBuf::from("/custom/cache/dir"));
    }

    #[test]
    fn when_cache_dir_has_tilde_then_should_expand() {
        let config = Config::try_parse_from([
            "kagi-mcp",
            "--api-key",
            "test-key",
            "--cache-dir",
            "~/custom/cache",
        ])
        .unwrap();
        let resolved = config.resolved_cache_dir().unwrap();
        let resolved_str = resolved.to_string_lossy();
        assert!(
            !resolved_str.contains('~'),
            "resolved path should not contain literal tilde: {resolved_str}"
        );
    }

    #[test]
    fn when_single_fallback_message_then_should_parse_correctly() {
        let config = Config::try_parse_from([
            "kagi-mcp",
            "--api-key",
            "test-key",
            "--fallback-message",
            "github.com=Use github-mcp instead",
        ])
        .unwrap();

        assert_eq!(config.fallback_messages.len(), 1);
        assert_eq!(config.fallback_messages[0].domain, "github.com");
        assert_eq!(
            config.fallback_messages[0].message,
            "Use github-mcp instead"
        );
        assert!(!config.fallback_messages[0].always_block);
    }

    #[test]
    fn when_multiple_fallback_messages_then_should_parse_all() {
        let config = Config::try_parse_from([
            "kagi-mcp",
            "--api-key",
            "test-key",
            "--fallback-message",
            "github.com=Use github-mcp instead",
            "--fallback-message",
            "gitlab.com=Use gitlab-mcp instead",
        ])
        .unwrap();

        assert_eq!(config.fallback_messages.len(), 2);
        assert_eq!(config.fallback_messages[0].domain, "github.com");
        assert_eq!(
            config.fallback_messages[0].message,
            "Use github-mcp instead"
        );
        assert_eq!(config.fallback_messages[1].domain, "gitlab.com");
        assert_eq!(
            config.fallback_messages[1].message,
            "Use gitlab-mcp instead"
        );
    }

    #[test]
    fn when_fallback_always_then_should_set_always_block_and_default_message() {
        let config = Config::try_parse_from([
            "kagi-mcp",
            "--api-key",
            "test-key",
            "--fallback-always",
            "github.com",
            "--fallback-always",
            "gitlab.com",
        ])
        .unwrap();

        assert_eq!(config.fallback_always.len(), 2);
        assert_eq!(config.fallback_always[0].domain, "github.com");
        assert!(config.fallback_always[0].always_block);
        assert!(config.fallback_always[0].message.is_empty());
        assert_eq!(config.fallback_always[1].domain, "gitlab.com");
        assert!(config.fallback_always[1].always_block);
        assert!(config.fallback_always[1].message.is_empty());
    }

    #[test]
    fn when_fallback_message_malformed_then_should_fail() {
        let result = Config::try_parse_from([
            "kagi-mcp",
            "--api-key",
            "test-key",
            "--fallback-message",
            "just-a-domain",
        ]);

        assert!(result.is_err());
    }

    #[test]
    fn when_empty_domain_then_should_fail() {
        let result = Config::try_parse_from([
            "kagi-mcp",
            "--api-key",
            "test-key",
            "--fallback-message",
            "=some message",
        ]);

        assert!(result.is_err());
    }
}
