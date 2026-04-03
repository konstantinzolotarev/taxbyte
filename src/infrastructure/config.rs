use config::{Config as ConfigBuilder, ConfigError, Environment, File};
use serde::Deserialize;
use std::env;
use std::fmt;

/// Database backend selection
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseBackend {
  Postgres,
  #[default]
  Sqlite,
}

impl fmt::Display for DatabaseBackend {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Postgres => write!(f, "postgres"),
      Self::Sqlite => write!(f, "sqlite"),
    }
  }
}

// Default value functions for serde
fn default_server_host() -> String {
  "127.0.0.1".to_string()
}

fn default_server_port() -> u16 {
  8080
}

fn default_server_base_url() -> String {
  "http://127.0.0.1:8080".to_string()
}

fn default_database_backend() -> DatabaseBackend {
  DatabaseBackend::Sqlite
}

fn default_database_url() -> String {
  "sqlite://./data/taxbyte.db?mode=rwc".to_string()
}

fn default_db_max_connections() -> u32 {
  30
}

fn default_db_connect_timeout() -> u64 {
  5
}

fn default_db_acquire_timeout() -> u64 {
  30
}

fn default_redis_url() -> String {
  "redis://localhost:6379".to_string()
}

fn default_redis_connect_timeout() -> u64 {
  5
}

fn default_password_min_length() -> usize {
  12
}

fn default_session_ttl() -> u64 {
  3600
}

fn default_remember_me_ttl() -> u64 {
  2592000
}

fn default_login_max_attempts() -> u32 {
  5
}

fn default_login_window_seconds() -> u64 {
  300
}

fn default_pdf_output_dir() -> String {
  "./data/invoices/pdfs".to_string()
}

/// Main application configuration
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
  #[serde(default)]
  pub server: ServerConfig,
  #[serde(default)]
  pub database: DatabaseConfig,
  #[serde(default)]
  pub redis: RedisConfig,
  #[serde(default)]
  pub security: SecurityConfig,
  #[serde(default)]
  pub rate_limit: RateLimitConfig,
  #[serde(default)]
  pub google_drive: Option<GoogleDriveConfig>,
  #[serde(default)]
  pub pdf: PdfConfig,
}

/// Server configuration
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
  #[serde(default = "default_server_host")]
  pub host: String,
  #[serde(default = "default_server_port")]
  pub port: u16,
  #[serde(default = "default_server_base_url")]
  pub base_url: String,
}

impl Default for ServerConfig {
  fn default() -> Self {
    Self {
      host: default_server_host(),
      port: default_server_port(),
      base_url: default_server_base_url(),
    }
  }
}

/// Database configuration
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
  #[serde(default = "default_database_backend")]
  pub backend: DatabaseBackend,
  #[serde(default = "default_database_url")]
  pub url: String,
  #[serde(default = "default_db_max_connections")]
  pub max_connections: u32,
  #[serde(default = "default_db_connect_timeout")]
  pub connect_timeout_seconds: u64,
  #[serde(default = "default_db_acquire_timeout")]
  pub acquire_timeout_seconds: u64,
}

impl Default for DatabaseConfig {
  fn default() -> Self {
    Self {
      backend: default_database_backend(),
      url: default_database_url(),
      max_connections: default_db_max_connections(),
      connect_timeout_seconds: default_db_connect_timeout(),
      acquire_timeout_seconds: default_db_acquire_timeout(),
    }
  }
}

/// Redis configuration
#[derive(Debug, Clone, Deserialize)]
pub struct RedisConfig {
  #[serde(default = "default_redis_url")]
  pub url: String,
  #[serde(default = "default_redis_connect_timeout")]
  pub connect_timeout_seconds: u64,
}

impl Default for RedisConfig {
  fn default() -> Self {
    Self {
      url: default_redis_url(),
      connect_timeout_seconds: default_redis_connect_timeout(),
    }
  }
}

/// Security configuration
#[derive(Debug, Clone, Deserialize)]
pub struct SecurityConfig {
  #[serde(default = "default_password_min_length")]
  pub password_min_length: usize,
  #[serde(default = "default_session_ttl")]
  pub session_ttl_seconds: u64,
  #[serde(default = "default_remember_me_ttl")]
  pub remember_me_ttl_seconds: u64,
  /// Base64-encoded 32-byte encryption key for OAuth tokens
  /// Generate with: openssl rand -base64 32
  /// MUST be set via environment variable (TAXBYTE_SECURITY__ENCRYPTION_KEY_BASE64)
  pub encryption_key_base64: String,
}

impl Default for SecurityConfig {
  fn default() -> Self {
    Self {
      password_min_length: default_password_min_length(),
      session_ttl_seconds: default_session_ttl(),
      remember_me_ttl_seconds: default_remember_me_ttl(),
      encryption_key_base64: String::new(),
    }
  }
}

/// Rate limiting configuration
#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitConfig {
  #[serde(default = "default_login_max_attempts")]
  pub login_max_attempts: u32,
  #[serde(default = "default_login_window_seconds")]
  pub login_window_seconds: u64,
}

impl Default for RateLimitConfig {
  fn default() -> Self {
    Self {
      login_max_attempts: default_login_max_attempts(),
      login_window_seconds: default_login_window_seconds(),
    }
  }
}

/// Google Drive configuration
#[derive(Debug, Clone, Deserialize)]
pub struct GoogleDriveConfig {
  pub service_account_key_path: String,
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
  #[serde(default = "default_pdf_output_dir")]
  pub output_dir: String,
  pub wkhtmltopdf_path: Option<String>,
}

impl Default for PdfConfig {
  fn default() -> Self {
    Self {
      output_dir: default_pdf_output_dir(),
      wkhtmltopdf_path: None,
    }
  }
}

impl Config {
  /// Load configuration from files and environment variables
  ///
  /// Configuration is loaded in the following order (later sources override earlier ones):
  /// 1. config/default.toml (optional - provides development defaults)
  /// 2. config/local.toml (optional - local overrides)
  /// 3. config/{RUN_MODE}.toml (optional - environment-specific)
  /// 4. Environment variables with TAXBYTE_ prefix (highest priority, loaded from .env)
  ///
  /// The application can be fully configured using only a .env file.
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
  /// - `TAXBYTE_GOOGLE_DRIVE__OAUTH_CLIENT_ID=your-client-id.apps.googleusercontent.com`
  /// - `TAXBYTE_GOOGLE_DRIVE__OAUTH_CLIENT_SECRET=your-client-secret`
  /// - `TAXBYTE_GOOGLE_DRIVE__OAUTH_REDIRECT_URL=http://localhost:8080/oauth/google/callback`
  ///
  /// Note: Use double underscores (__) to separate the section name from the field name.
  /// For nested config like `google_drive.oauth_client_id`, use `GOOGLE_DRIVE__OAUTH_CLIENT_ID`.
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
      // Start with default configuration (optional - can be fully configured via .env)
      .add_source(File::with_name("config/default").required(false))
      // Add optional local configuration (for local development overrides)
      .add_source(File::with_name("config/local").required(false))
      // Add optional environment-specific configuration
      .add_source(File::with_name(&format!("config/{}", run_mode)).required(false))
      // Add environment variables with TAXBYTE_ prefix (highest priority)
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
            base_url = "http://127.0.0.1:8080"

            [database]
            url = "postgres://localhost/taxbyte"
            max_connections = 5

            [redis]
            url = "redis://localhost"

            [security]
            password_min_length = 8
            session_ttl_seconds = 3600
            remember_me_ttl_seconds = 2592000
            encryption_key_base64 = "2e26WueyLmI1t+XuJIu/o74VCrjf8yebwywMqEE8g5k="

            [rate_limit]
            login_max_attempts = 5
            login_window_seconds = 300

            [pdf]
            output_dir = "./data/invoices/pdfs"
        "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse config");

    assert_eq!(config.server.host, "127.0.0.1");
    assert_eq!(config.server.port, 8080);
    assert_eq!(config.server.base_url, "http://127.0.0.1:8080");
    assert_eq!(config.database.url, "postgres://localhost/taxbyte");
    assert_eq!(config.database.max_connections, 5);
    assert_eq!(config.database.connect_timeout_seconds, 5); // default
    assert_eq!(config.database.acquire_timeout_seconds, 30); // default
    assert_eq!(config.redis.url, "redis://localhost");
    assert_eq!(config.redis.connect_timeout_seconds, 5); // default
    assert_eq!(config.security.password_min_length, 8);
    assert_eq!(config.security.session_ttl_seconds, 3600);
    assert_eq!(config.security.remember_me_ttl_seconds, 2592000);
    assert_eq!(config.rate_limit.login_max_attempts, 5);
    assert_eq!(config.rate_limit.login_window_seconds, 300);
  }

  #[test]
  fn test_config_defaults_without_toml() {
    // Config can be deserialized from an empty TOML (all sections have defaults)
    let config: Config = toml::from_str("").expect("Failed to parse empty config");

    // Server defaults
    assert_eq!(config.server.host, "127.0.0.1");
    assert_eq!(config.server.port, 8080);
    assert_eq!(config.server.base_url, "http://127.0.0.1:8080");

    // Database defaults
    assert_eq!(config.database.backend, DatabaseBackend::Sqlite);
    assert_eq!(config.database.url, "sqlite://./data/taxbyte.db?mode=rwc");
    assert_eq!(config.database.max_connections, 30);
    assert_eq!(config.database.connect_timeout_seconds, 5);
    assert_eq!(config.database.acquire_timeout_seconds, 30);

    // Redis defaults
    assert_eq!(config.redis.url, "redis://localhost:6379");
    assert_eq!(config.redis.connect_timeout_seconds, 5);

    // Security defaults
    assert_eq!(config.security.password_min_length, 12);
    assert_eq!(config.security.session_ttl_seconds, 3600);
    assert_eq!(config.security.remember_me_ttl_seconds, 2592000);
    assert!(config.security.encryption_key_base64.is_empty());

    // Rate limit defaults
    assert_eq!(config.rate_limit.login_max_attempts, 5);
    assert_eq!(config.rate_limit.login_window_seconds, 300);

    // Google Drive defaults to None
    assert!(config.google_drive.is_none());

    // PDF defaults
    assert_eq!(config.pdf.output_dir, "./data/invoices/pdfs");
    assert!(config.pdf.wkhtmltopdf_path.is_none());
  }

  #[test]
  fn test_config_partial_toml_with_defaults() {
    // Only specify security section, rest should use defaults
    let toml = r#"
            [security]
            encryption_key_base64 = "test-key-base64"
        "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse partial config");

    // Explicitly set value
    assert_eq!(config.security.encryption_key_base64, "test-key-base64");

    // Security fields with defaults still applied
    assert_eq!(config.security.password_min_length, 12);
    assert_eq!(config.security.session_ttl_seconds, 3600);

    // Other sections use defaults
    assert_eq!(config.server.port, 8080);
    assert_eq!(config.database.max_connections, 30);
    assert_eq!(config.redis.url, "redis://localhost:6379");
  }

  #[test]
  fn test_config_toml_overrides_defaults() {
    // Values from TOML should override struct defaults
    let toml = r#"
            [server]
            host = "0.0.0.0"
            port = 3000
            base_url = "https://example.com"

            [database]
            url = "postgres://prod:secret@db.example.com/taxbyte"
            max_connections = 50
            connect_timeout_seconds = 10
            acquire_timeout_seconds = 60

            [redis]
            url = "redis://cache.example.com:6380"
            connect_timeout_seconds = 10

            [security]
            password_min_length = 16
            session_ttl_seconds = 7200
            remember_me_ttl_seconds = 5184000
            encryption_key_base64 = "production-key"

            [rate_limit]
            login_max_attempts = 3
            login_window_seconds = 600

            [pdf]
            output_dir = "/var/data/pdfs"
            wkhtmltopdf_path = "/usr/local/bin/wkhtmltopdf"
        "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse config");

    assert_eq!(config.server.host, "0.0.0.0");
    assert_eq!(config.server.port, 3000);
    assert_eq!(config.server.base_url, "https://example.com");
    assert_eq!(
      config.database.url,
      "postgres://prod:secret@db.example.com/taxbyte"
    );
    assert_eq!(config.database.max_connections, 50);
    assert_eq!(config.database.connect_timeout_seconds, 10);
    assert_eq!(config.database.acquire_timeout_seconds, 60);
    assert_eq!(config.redis.url, "redis://cache.example.com:6380");
    assert_eq!(config.redis.connect_timeout_seconds, 10);
    assert_eq!(config.security.password_min_length, 16);
    assert_eq!(config.security.session_ttl_seconds, 7200);
    assert_eq!(config.security.remember_me_ttl_seconds, 5184000);
    assert_eq!(config.rate_limit.login_max_attempts, 3);
    assert_eq!(config.rate_limit.login_window_seconds, 600);
    assert_eq!(config.pdf.output_dir, "/var/data/pdfs");
    assert_eq!(
      config.pdf.wkhtmltopdf_path,
      Some("/usr/local/bin/wkhtmltopdf".to_string())
    );
  }

  #[test]
  fn test_config_with_google_drive() {
    let toml = r#"
            [security]
            encryption_key_base64 = "key"

            [google_drive]
            enabled = true
            service_account_key_path = "./sa.json"
            oauth_client_id = "client-id"
            oauth_client_secret = "client-secret"
            oauth_redirect_url = "http://localhost:8080/oauth/callback"
        "#;

    let config: Config = toml::from_str(toml).expect("Failed to parse config");

    let gd = config.google_drive.expect("google_drive should be Some");
    assert!(gd.enabled);
    assert_eq!(gd.service_account_key_path, "./sa.json");
    assert_eq!(gd.oauth_client_id, Some("client-id".to_string()));
    assert_eq!(gd.oauth_client_secret, Some("client-secret".to_string()));
    assert_eq!(
      gd.oauth_redirect_url,
      Some("http://localhost:8080/oauth/callback".to_string())
    );
  }
}
