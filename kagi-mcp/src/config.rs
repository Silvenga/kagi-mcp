use clap::Parser;

#[derive(Debug, Parser, Clone)]
#[command(name = "kagi-mcp", about = "Kagi MCP server")]
pub struct Config {
    #[arg(long, env = "KAGI_API_KEY")]
    pub api_key: String,

    #[arg(
        long,
        env = "KAGI_BASE_URL",
        default_value = "https://kagi.com/api"
    )]
    pub base_url: String,

    #[arg(long, env = "KAGI_TIMEOUT", default_value = "4.0")]
    pub kagi_timeout: f64,

    #[arg(long, env = "KAGI_CLIENT_TIMEOUT", default_value = "10.0")]
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

    #[arg(long, env = "KAGI_REGION")]
    pub region: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_default_args_then_default_values_should_apply() {
        let config = Config::try_parse_from(["kagi-mcp", "--api-key", "test-key"]).unwrap();

        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.base_url, "https://kagi.com/api");
        assert_eq!(config.kagi_timeout, 4.0);
        assert_eq!(config.client_timeout, 10.0);
        assert_eq!(config.retries, 3);
        assert_eq!(config.limit, 10);
        assert!(config.safe_search);
        assert_eq!(config.region, None);
    }

    #[test]
    fn when_all_args_provided_then_custom_values_should_apply() {
        let config = Config::try_parse_from([
            "kagi-mcp",
            "--api-key",
            "custom-key",
            "--base-url",
            "https://custom.example.com",
            "--kagi-timeout",
            "8.5",
            "--client-timeout",
            "30.0",
            "--retries",
            "5",
            "--limit",
            "25",
            "--safe-search",
            "false",
            "--region",
            "us-west",
        ])
        .unwrap();

        assert_eq!(config.api_key, "custom-key");
        assert_eq!(config.base_url, "https://custom.example.com");
        assert_eq!(config.kagi_timeout, 8.5);
        assert_eq!(config.client_timeout, 30.0);
        assert_eq!(config.retries, 5);
        assert_eq!(config.limit, 25);
        assert!(!config.safe_search);
        assert_eq!(config.region.as_deref(), Some("us-west"));
    }

    #[test]
    fn when_missing_api_key_then_parse_should_fail() {
        let result = Config::try_parse_from(["kagi-mcp"]);
        assert!(result.is_err());
    }
}
