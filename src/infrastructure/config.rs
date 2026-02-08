use config::{Config as ConfigBuilder, ConfigError, Environment, File};
use serde::Deserialize;
use std::env;

// Default timeout functions
fn default_db_connect_timeout() -> u64 {
  5
}

fn default_db_acquire_timeout() -> u64 {
  3
}

fn default_redis_connect_timeout() -> u64 {
  5
}

/// Main application configuration
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
  pub server: ServerConfig,
  pub database: DatabaseConfig,
  pub redis: RedisConfig,
  pub security: SecurityConfig,
  pub rate_limit: RateLimitConfig,
  #[serde(default)]
  pub google_drive: Option<GoogleDriveConfig>,
  pub pdf: PdfConfig,
}

/// Server configuration
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
  pub host: String,
  pub port: u16,
  pub base_url: String,
}

/// Database configuration
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
  pub url: String,
  pub max_connections: u32,
  #[serde(default = "default_db_connect_timeout")]
  pub connect_timeout_seconds: u64,
  #[serde(default = "default_db_acquire_timeout")]
  pub acquire_timeout_seconds: u64,
}

/// Redis configuration
#[derive(Debug, Clone, Deserialize)]
pub struct RedisConfig {
  pub url: String,
  #[serde(default = "default_redis_connect_timeout")]
  pub connect_timeout_seconds: u64,
}

/// Security configuration
#[derive(Debug, Clone, Deserialize)]
pub struct SecurityConfig {
  pub password_min_length: usize,
  pub session_ttl_seconds: u64,
  pub remember_me_ttl_seconds: u64,
  /// Base64-encoded 32-byte encryption key for OAuth tokens
  /// Generate with: openssl rand -base64 32
  pub encryption_key_base64: String,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitConfig {
  pub login_max_attempts: u32,
  pub login_window_seconds: u64,
}

/// Google Drive configuration
#[derive(Debug, Clone, Deserialize)]
pub struct GoogleDriveConfig {
  pub service_account_key_path: String,
  pub parent_folder_id: Option<String>,
  pub default_invoice_subfolder: String,
  pub enabled: bool,
  /// OAuth 2.0 client ID from Google Cloud Console
  pub oauth_client_id: Option<String>,
  /// OAuth 2.0 client secret from Google Cloud Console
  pub oauth_client_secret: Option<String>,
  /// OAuth 2.0 redirect URL (must match Google Cloud Console settings)
  pub oauth_redirect_url: Option<String>,
}

/// PDF generation configuration
#[derive(Debug, Clone, Deserialize)]
pub struct PdfConfig {
  pub output_dir: String,
  pub wkhtmltopdf_path: Option<String>,
}

impl Config {
  /// Load configuration from files and environment variables
  ///
  /// Configuration is loaded in the following order (later sources override earlier ones):
  /// 1. config/default.toml
  /// 2. config/local.toml (if exists)
  /// 3. Environment variables with TAXBYTE_ prefix
  ///
  /// # Example
  ///
  /// ```no_run
  /// use taxbyte::infrastructure::config::Config;
  ///
  /// let config = Config::load().expect("Failed to load configuration");
  /// println!("Server running on {}:{}", config.server.host, config.server.port);
  /// ```
  ///
  /// # Environment Variables
  ///
  /// Environment variables use the TAXBYTE_ prefix and are separated by double underscores:
  /// - `TAXBYTE_SERVER__HOST=0.0.0.0`
  /// - `TAXBYTE_SERVER__PORT=8080`
  /// - `TAXBYTE_DATABASE__URL=postgres://user:pass@localhost/db`
  /// - `TAXBYTE_DATABASE__MAX_CONNECTIONS=10`
  /// - `TAXBYTE_REDIS__URL=redis://localhost`
  /// - `TAXBYTE_SECURITY__PASSWORD_MIN_LENGTH=8`
  /// - `TAXBYTE_SECURITY__SESSION_TTL_SECONDS=3600`
  /// - `TAXBYTE_SECURITY__REMEMBER_ME_TTL_SECONDS=2592000`
  /// - `TAXBYTE_RATE_LIMIT__LOGIN_MAX_ATTEMPTS=5`
  /// - `TAXBYTE_RATE_LIMIT__LOGIN_WINDOW_SECONDS=300`
  ///
  /// # Errors
  ///
  /// Returns a `ConfigError` if:
  /// - Required configuration files are missing
  /// - Configuration files contain invalid TOML
  /// - Required configuration values are missing
  /// - Configuration values have invalid types
  pub fn load() -> Result<Self, ConfigError> {
    let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "development".into());

    let config = ConfigBuilder::builder()
      // Start with default configuration
      .add_source(File::with_name("config/default").required(true))
      // Add optional local configuration (for local development overrides)
      .add_source(File::with_name("config/local").required(false))
      // Add optional environment-specific configuration
      .add_source(File::with_name(&format!("config/{}", run_mode)).required(false))
      // Add environment variables with TAXBYTE_ prefix
      // Use double underscore as separator: TAXBYTE_SERVER__PORT=8080
      .add_source(
        Environment::with_prefix("TAXBYTE")
          .prefix_separator("_")
          .separator("__")
          .try_parsing(true),
      )
      .build()?;

    config.try_deserialize()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_config_structure() {
    // This test verifies that the Config structure can be deserialized
    let toml = r#"
            [server]
            host = "127.0.0.1"
            port = 8080

            [database]
            url = "postgres://localhost/taxbyte"
            max_connections = 5

            [redis]
            url = "redis://localhost"

            [security]
            password_min_length = 8
            session_ttl_seconds = 3600
            remember_me_ttl_seconds = 2592000

            [rate_limit]
            login_max_attempts = 5
            login_window_seconds = 300

            [pdf]
            output_dir = "./data/invoices/pdfs"
        "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse config");

    assert_eq!(config.server.host, "127.0.0.1");
    assert_eq!(config.server.port, 8080);
    assert_eq!(config.database.url, "postgres://localhost/taxbyte");
    assert_eq!(config.database.max_connections, 5);
    assert_eq!(config.database.connect_timeout_seconds, 5); // default
    assert_eq!(config.database.acquire_timeout_seconds, 3); // default
    assert_eq!(config.redis.url, "redis://localhost");
    assert_eq!(config.redis.connect_timeout_seconds, 5); // default
    assert_eq!(config.security.password_min_length, 8);
    assert_eq!(config.security.session_ttl_seconds, 3600);
    assert_eq!(config.security.remember_me_ttl_seconds, 2592000);
    assert_eq!(config.rate_limit.login_max_attempts, 5);
    assert_eq!(config.rate_limit.login_window_seconds, 300);
  }
}
