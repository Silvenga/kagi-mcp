use clap::{Parser, ValueEnum};
use std::path::PathBuf;

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

#[derive(Debug, Clone, ValueEnum, Default)]
pub enum TransportMode {
    #[default]
    Stdio,
    StreamableHttp,
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
        default_value = "~/.cache/kagi-mcp/",
        value_parser = parse_cache_dir
    )]
    pub cache_dir: PathBuf,

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

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
        let expected_cache_dir = shellexpand::tilde("~/.cache/kagi-mcp/");
        assert_eq!(config.cache_dir, PathBuf::from(expected_cache_dir.as_ref()));
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
        assert_eq!(config.cache_dir, PathBuf::from("/custom/cache/dir"));
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
    fn when_default_args_then_transport_should_default_to_stdio() {
        // Clear env var to avoid interference from parallel env tests
        env::remove_var("KAGI_TRANSPORT");

        let config =
            Config::try_parse_from(["kagi-mcp", "--api-key", "test-key"]).unwrap();

        assert_eq!(config.bind, "127.0.0.1:3000");
        assert!(matches!(config.transport, TransportMode::Stdio));
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
    fn when_env_transport_set_then_should_parse() {
        let prior = env::var_os("KAGI_TRANSPORT");
        env::set_var("KAGI_TRANSPORT", "streamable-http");
        let config =
            Config::try_parse_from(["kagi-mcp", "--api-key", "test-key"]).unwrap();
        match prior {
            Some(v) => env::set_var("KAGI_TRANSPORT", v),
            None => env::remove_var("KAGI_TRANSPORT"),
        }

        assert!(matches!(config.transport, TransportMode::StreamableHttp));
    }
}
