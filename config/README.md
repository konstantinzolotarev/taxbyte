# TaxByte Configuration

This directory contains configuration files for the TaxByte application.

## Configuration Files

- **default.toml** - Default configuration values (committed to git)
- **local.toml** - Local development overrides (gitignored, optional)
- **{environment}.toml** - Environment-specific configs (gitignored, optional)

## Configuration Priority

Configuration is loaded in the following order (later sources override earlier ones):

1. `config/default.toml` - Default configuration (always loaded)
2. `config/local.toml` - Local development overrides (optional)
3. `config/{RUN_MODE}.toml` - Environment-specific configuration (optional)
4. Environment variables with `TAXBYTE_` prefix - Runtime overrides

## Environment Variables

All configuration values can be overridden using environment variables with the `TAXBYTE_` prefix.
Use double underscores (`__`) to separate nested configuration keys.

### Server Configuration

```bash
TAXBYTE_SERVER__HOST=0.0.0.0
TAXBYTE_SERVER__PORT=8080
```

### Database Configuration

```bash
TAXBYTE_DATABASE__URL=postgres://user:pass@localhost:5432/taxbyte
TAXBYTE_DATABASE__MAX_CONNECTIONS=10
```

### Redis Configuration

```bash
TAXBYTE_REDIS__URL=redis://localhost:6379
```

### Security Configuration

```bash
TAXBYTE_SECURITY__PASSWORD_MIN_LENGTH=12
TAXBYTE_SECURITY__SESSION_TTL_SECONDS=3600
TAXBYTE_SECURITY__REMEMBER_ME_TTL_SECONDS=2592000
```

### Rate Limiting Configuration

```bash
TAXBYTE_RATE_LIMIT__LOGIN_MAX_ATTEMPTS=5
TAXBYTE_RATE_LIMIT__LOGIN_WINDOW_SECONDS=300
```

## Running in Different Environments

Set the `RUN_MODE` environment variable to load environment-specific configuration:

```bash
RUN_MODE=production cargo run
```

This will attempt to load `config/production.toml` if it exists.

## Local Development

For local development overrides:

1. Copy `local.toml.example` to `local.toml`
2. Customize values as needed
3. The file is gitignored and won't be committed

## Example Usage

```rust
use taxbyte::infrastructure::config::Config;

fn main() {
    let config = Config::load().expect("Failed to load configuration");
    println!("Server running on {}:{}", config.server.host, config.server.port);
}
```

See `examples/config_usage.rs` for a complete example.
