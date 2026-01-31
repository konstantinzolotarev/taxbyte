# TaxByte

A modern, secure, and scalable tax management platform built with Rust and Actix-Web.

## Project Overview

TaxByte is a comprehensive tax management solution designed to help businesses and individuals manage their tax obligations efficiently. The platform provides secure user authentication, company management, and will include features for tax calculations, filing, and compliance tracking.

## Architecture Overview

This project follows **Hexagonal Architecture** (also known as Ports and Adapters Architecture) to ensure clean separation of concerns and maintainability:

```
src/
├── domain/          # Core business logic and entities
│   └── auth/        # Authentication domain
│       ├── entities.rs        # Domain entities (User, Session)
│       ├── value_objects.rs   # Value objects (Email, Password)
│       ├── ports.rs           # Port interfaces (repositories)
│       ├── services.rs        # Domain services
│       └── errors.rs          # Domain errors
│
├── application/     # Use cases and application logic
│   └── auth/        # Authentication use cases
│       ├── register_user.rs
│       ├── login_user.rs
│       ├── logout_user.rs
│       ├── logout_all_devices.rs
│       └── get_current_user.rs
│
├── adapters/        # External interfaces
│   └── http/        # HTTP adapter (REST API)
│       ├── handlers/          # Request handlers
│       ├── middleware/        # HTTP middleware
│       ├── routes.rs          # Route configuration
│       ├── dtos.rs            # Data transfer objects
│       └── errors.rs          # HTTP error handling
│
└── infrastructure/  # Technical implementations
    ├── config.rs              # Configuration management
    ├── persistence/           # Database implementations
    │   └── postgres/          # PostgreSQL repositories
    │       ├── user_repository.rs
    │       ├── session_repository.rs
    │       └── login_attempt_repository.rs
    └── security/              # Security implementations
        ├── argon2_hasher.rs
        └── token_generator.rs
```

### Key Architectural Principles

1. **Domain-Driven Design**: Business logic is isolated in the domain layer
2. **Dependency Inversion**: Core domain depends on abstractions, not implementations
3. **Testability**: Each layer can be tested independently
4. **Flexibility**: Easy to swap implementations (e.g., database, web framework)

## Prerequisites

Before you begin, ensure you have the following installed:

- **Rust** (1.75 or later): [Install Rust](https://www.rust-lang.org/tools/install)
- **Docker** and **Docker Compose**: [Install Docker](https://docs.docker.com/get-docker/)
- **PostgreSQL Client Tools** (optional, for manual database access): `brew install postgresql` (macOS)

## Setup Instructions

### 1. Clone the Repository

```bash
git clone <repository-url>
cd taxbyte
```

### 2. Start Docker Services

Start PostgreSQL and Redis using Docker Compose:

```bash
docker-compose up -d
```

Verify services are running:

```bash
docker-compose ps
```

You should see both `taxbyte-postgres` and `taxbyte-redis` containers running.

### 3. Configure Environment Variables

Copy the example environment file:

```bash
cp .env .env.local
```

The default `.env` file contains sensible defaults for local development. Modify `.env.local` if you need custom settings.

### 4. Run Database Migrations

The application automatically runs migrations on startup, but you can also run them manually using `sqlx-cli`:

```bash
# Install sqlx-cli if you haven't already
cargo install sqlx-cli --no-default-features --features postgres

# Run migrations
sqlx migrate run --database-url "postgres://postgres:postgres@localhost:5432/taxbyte"
```

### 5. Build and Run the Application

```bash
# Build the project
cargo build

# Run the application
cargo run
```

The server will start on `http://127.0.0.1:8080`

### 6. Verify Installation

Check the health endpoint:

```bash
curl http://localhost:8080/health
```

You should receive an `OK` response.

## API Endpoints Documentation

### Health Check

#### GET /health

Check if the service is running.

**Response:**

- `200 OK`: Service is healthy

```bash
curl http://localhost:8080/health
```

### Authentication Endpoints

All authentication endpoints are prefixed with `/api/v1/auth`.

#### POST /api/v1/auth/register

Register a new user account.

**Request Body:**

```json
{
  "email": "user@example.com",
  "password": "SecurePassword123!",
  "full_name": "John Doe"
}
```

**Validation Rules:**

- Email: Valid email format
- Password: Minimum 12 characters (configurable)
- Full Name: Not empty, max 255 characters

**Response:**

- `201 Created`: User successfully registered

```json
{
  "id": "uuid",
  "email": "user@example.com",
  "full_name": "John Doe",
  "created_at": "2026-01-31T12:00:00Z"
}
```

- `400 Bad Request`: Validation error
- `409 Conflict`: Email already exists

**Example:**

```bash
curl -X POST http://localhost:8080/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "SecurePassword123!",
    "full_name": "John Doe"
  }'
```

#### POST /api/v1/auth/login

Authenticate and create a session.

**Request Body:**

```json
{
  "email": "user@example.com",
  "password": "SecurePassword123!",
  "remember_me": false
}
```

**Response:**

- `200 OK`: Login successful

```json
{
  "session_token": "generated-session-token",
  "user": {
    "id": "uuid",
    "email": "user@example.com",
    "full_name": "John Doe",
    "created_at": "2026-01-31T12:00:00Z"
  },
  "expires_at": "2026-01-31T13:00:00Z"
}
```

- `401 Unauthorized`: Invalid credentials
- `429 Too Many Requests`: Rate limit exceeded (5 attempts per 5 minutes)

**Example:**

```bash
curl -X POST http://localhost:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "SecurePassword123!",
    "remember_me": false
  }'
```

#### POST /api/v1/auth/logout

Logout and invalidate current session.

**Headers:**

- `Authorization: Bearer <session-token>`

**Response:**

- `200 OK`: Logout successful

```json
{
  "message": "Logged out successfully"
}
```

- `401 Unauthorized`: Invalid or expired session token

**Example:**

```bash
curl -X POST http://localhost:8080/api/v1/auth/logout \
  -H "Authorization: Bearer <session-token>"
```

#### POST /api/v1/auth/logout-all

Logout from all devices (invalidate all sessions).

**Headers:**

- `Authorization: Bearer <session-token>`

**Response:**

- `200 OK`: All sessions invalidated

```json
{
  "message": "Logged out from all devices"
}
```

- `401 Unauthorized`: Invalid or expired session token

**Example:**

```bash
curl -X POST http://localhost:8080/api/v1/auth/logout-all \
  -H "Authorization: Bearer <session-token>"
```

#### GET /api/v1/auth/me

Get current authenticated user information.

**Headers:**

- `Authorization: Bearer <session-token>`

**Response:**

- `200 OK`: User information retrieved

```json
{
  "id": "uuid",
  "email": "user@example.com",
  "full_name": "John Doe",
  "created_at": "2026-01-31T12:00:00Z"
}
```

- `401 Unauthorized`: Invalid or expired session token

**Example:**

```bash
curl http://localhost:8080/api/v1/auth/me \
  -H "Authorization: Bearer <session-token>"
```

## Testing Instructions

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run tests in a specific module
cargo test domain::auth
```

### Integration Tests

Integration tests use testcontainers to spin up real PostgreSQL and Redis instances. Make sure Docker is running before executing integration tests.

```bash
# Run integration tests only
cargo test --test '*'
```

### Code Coverage

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --out Html --output-dir coverage
```

### Linting and Formatting

```bash
# Check formatting
cargo fmt -- --check

# Format code
cargo fmt

# Run clippy
cargo clippy -- -D warnings
```

## Configuration Options

Configuration is managed through a combination of:

1. **config/default.toml**: Default configuration values
2. **Environment variables**: Override specific values
3. **.env file**: Local environment variables (not committed to git)

### Configuration Files

- `config/default.toml`: Default configuration (committed to git)
- `config/local.toml`: Local overrides (gitignored)
- `.env`: Environment variables for local development

### Environment Variable Mapping

Environment variables use the `TAXBYTE_` prefix (optional, can also use plain variable names):

| Environment Variable      | Config Path                        | Default Value                                       |
| ------------------------- | ---------------------------------- | --------------------------------------------------- |
| `SERVER_HOST`             | `server.host`                      | `127.0.0.1`                                         |
| `SERVER_PORT`             | `server.port`                      | `8080`                                              |
| `DATABASE_URL`            | `database.url`                     | `postgres://taxbyte:taxbyte@localhost:5432/taxbyte` |
| `REDIS_URL`               | `redis.url`                        | `redis://localhost:6379`                            |
| `PASSWORD_MIN_LENGTH`     | `security.password_min_length`     | `12`                                                |
| `SESSION_TTL_SECONDS`     | `security.session_ttl_seconds`     | `3600` (1 hour)                                     |
| `REMEMBER_ME_TTL_SECONDS` | `security.remember_me_ttl_seconds` | `2592000` (30 days)                                 |
| `LOGIN_MAX_ATTEMPTS`      | `rate_limit.login_max_attempts`    | `5`                                                 |
| `LOGIN_WINDOW_SECONDS`    | `rate_limit.login_window_seconds`  | `300` (5 minutes)                                   |

### Logging Configuration

Set the `RUST_LOG` environment variable to control logging levels:

```bash
# Default
RUST_LOG=taxbyte=debug,actix_web=info

# More verbose
RUST_LOG=taxbyte=trace,actix_web=debug

# Production
RUST_LOG=taxbyte=info,actix_web=warn
```

## Development Workflow

### Making Changes

1. Create a feature branch
2. Make your changes
3. Run tests: `cargo test`
4. Run linter: `cargo clippy`
5. Format code: `cargo fmt`
6. Commit and push

### Database Migrations

Create a new migration:

```bash
sqlx migrate add <migration_name>
```

This creates a new SQL file in the `migrations/` directory. Edit the file to add your schema changes.

### Adding New Dependencies

```bash
cargo add <package-name>

# For dev dependencies
cargo add --dev <package-name>
```

## Docker Commands

### Start Services

```bash
docker-compose up -d
```

### Stop Services

```bash
docker-compose down
```

### View Logs

```bash
# All services
docker-compose logs -f

# Specific service
docker-compose logs -f postgres
docker-compose logs -f redis
```

### Reset Database

```bash
# Stop and remove containers, volumes
docker-compose down -v

# Start fresh
docker-compose up -d
```

### Access PostgreSQL

```bash
# Using docker exec
docker exec -it taxbyte-postgres psql -U taxbyte -d taxbyte

# Using local psql client
psql postgres://postgres:postgres@localhost:5432/taxbyte
```

### Access Redis

```bash
# Using docker exec
docker exec -it taxbyte-redis redis-cli

# Test connection
docker exec -it taxbyte-redis redis-cli ping
```

## Troubleshooting

### Port Already in Use

If ports 5432 or 6379 are already in use, modify the port mappings in `docker-compose.yml` and update `.env` accordingly.

### Database Connection Failed

1. Ensure Docker containers are running: `docker-compose ps`
2. Check container logs: `docker-compose logs postgres`
3. Verify DATABASE_URL in `.env` matches docker-compose.yml

### Redis Connection Failed

1. Ensure Redis container is running: `docker-compose ps`
2. Check container logs: `docker-compose logs redis`
3. Verify REDIS_URL in `.env` matches docker-compose.yml

### Migration Errors

If migrations fail, you may need to reset the database:

```bash
docker-compose down -v
docker-compose up -d
cargo run
```

## Project Status

### Completed Features

- User registration with email validation
- Secure password hashing using Argon2
- User authentication and session management
- Session storage in Redis for performance
- Rate limiting for login attempts
- Logout and logout-all-devices functionality
- Health check endpoint
- Request ID middleware for tracing

### Upcoming Features

- Company management
- Tax form management
- Tax calculation engine
- Document upload and storage
- Reporting and analytics
- Email notifications
- Two-factor authentication
- OAuth integration

## Contributing

1. Fork the repository
2. Create your feature branch: `git checkout -b feature/my-new-feature`
3. Commit your changes: `git commit -am 'Add some feature'`
4. Push to the branch: `git push origin feature/my-new-feature`
5. Submit a pull request

## License

This project is proprietary and confidential.

## Support

For questions or issues, please contact the development team or open an issue on the repository.
