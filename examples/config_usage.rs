//! Example demonstrating how to load and use configuration
//!
//! Run with: cargo run --example config_usage
//!
//! To override configuration with environment variables:
//! ```bash
//! TAXBYTE_SERVER__PORT=3000 \
//! TAXBYTE_DATABASE__MAX_CONNECTIONS=25 \
//! cargo run --example config_usage
//! ```

use taxbyte::infrastructure::config::Config;

fn main() {
  // Load configuration from files and environment variables
  match Config::load() {
    Ok(config) => {
      println!("Configuration loaded successfully!");
      println!();
      println!("Server:");
      println!("  Host: {}", config.server.host);
      println!("  Port: {}", config.server.port);
      println!();
      println!("Database:");
      println!("  URL: {}", config.database.url);
      println!("  Max Connections: {}", config.database.max_connections);
      println!();
      println!("Redis:");
      println!("  URL: {}", config.redis.url);
      println!();
      println!("Security:");
      println!(
        "  Password Min Length: {}",
        config.security.password_min_length
      );
      println!(
        "  Session TTL: {} seconds",
        config.security.session_ttl_seconds
      );
      println!(
        "  Remember Me TTL: {} seconds",
        config.security.remember_me_ttl_seconds
      );
      println!();
      println!("Rate Limit:");
      println!(
        "  Login Max Attempts: {}",
        config.rate_limit.login_max_attempts
      );
      println!(
        "  Login Window: {} seconds",
        config.rate_limit.login_window_seconds
      );
    }
    Err(e) => {
      eprintln!("Failed to load configuration: {}", e);
      std::process::exit(1);
    }
  }
}
