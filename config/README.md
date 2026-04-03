# TaxByte Configuration

This directory contains configuration files for the TaxByte application.

## Configuration Files

- **default.toml** - Default development values (optional, committed to git)
- **local.toml** - Local development overrides (gitignored, optional)
- **{environment}.toml** - Environment-specific configs (gitignored, optional)

## Configuration Priority

Configuration is loaded in the following order (later sources override earlier ones):

1. Struct defaults - Built-in fallback values in Rust code
2. `config/default.toml` - Default configuration (optional)
3. `config/local.toml` - Local development overrides (optional)
4. `config/{RUN_MODE}.toml` - Environment-specific configuration (optional)
5. `.env` file / Environment variables with `TAXBYTE_` prefix - **Highest priority**

The application can be fully configured using only a `.env` file.
All TOML config files are optional.

## Recommended Setup

For **development**, the provided `default.toml` has sensible defaults.
Just create a `.env` file with your encryption key:

```bash
cp .env.example .env
# Edit .env and set TAXBYTE_SECURITY__ENCRYPTION_KEY_BASE64
# Generate with: openssl rand -base64 32
```

For **production**, use only `.env` or environment variables.
Never put sensitive values (encryption keys, credentials) in TOML files.

## Environment Variables

All configuration values can be overridden using environment variables with the `TAXBYTE_` prefix.
Use double underscores (`__`) to separate nested configuration keys.

### Server Configuration

```bash
TAXBYTE_SERVER__HOST=0.0.0.0
TAXBYTE_SERVER__PORT=8080
TAXBYTE_SERVER__BASE_URL=https://example.com
```

### Database Configuration

```bash
TAXBYTE_DATABASE__URL=postgres://user:pass@localhost:5432/taxbyte
TAXBYTE_DATABASE__MAX_CONNECTIONS=30
TAXBYTE_DATABASE__CONNECT_TIMEOUT_SECONDS=5
TAXBYTE_DATABASE__ACQUIRE_TIMEOUT_SECONDS=30
```

### Redis Configuration

```bash
TAXBYTE_REDIS__URL=redis://localhost:6379
TAXBYTE_REDIS__CONNECT_TIMEOUT_SECONDS=5
```

### Security Configuration

```bash
TAXBYTE_SECURITY__PASSWORD_MIN_LENGTH=12
TAXBYTE_SECURITY__SESSION_TTL_SECONDS=3600
TAXBYTE_SECURITY__REMEMBER_ME_TTL_SECONDS=2592000
# Required - generate with: openssl rand -base64 32
TAXBYTE_SECURITY__ENCRYPTION_KEY_BASE64=your-base64-key-here
```

### Rate Limiting Configuration

```bash
TAXBYTE_RATE_LIMIT__LOGIN_MAX_ATTEMPTS=5
TAXBYTE_RATE_LIMIT__LOGIN_WINDOW_SECONDS=300
```

### Google Drive Configuration

```bash
TAXBYTE_GOOGLE_DRIVE__OAUTH_CLIENT_ID=your-client-id.apps.googleusercontent.com
TAXBYTE_GOOGLE_DRIVE__OAUTH_CLIENT_SECRET=your-client-secret
TAXBYTE_GOOGLE_DRIVE__OAUTH_REDIRECT_URL=http://localhost:8080/oauth/google/callback
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
