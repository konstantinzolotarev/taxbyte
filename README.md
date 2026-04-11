# TaxByte

A modern, secure, and scalable tax & invoice management platform built with Rust and Actix-Web.

## Project Overview

TaxByte is a comprehensive tax and invoice management solution designed to help businesses manage their invoicing, customers, and tax obligations efficiently. The platform provides secure user authentication, multi-tenant company management, invoice generation with PDF export, Google Drive integration, and more.

## Quick Start

### SQLite (Default -- Zero Dependencies)

```bash
git clone <repository-url>
cd taxbyte
cargo run
```

That's it. No Docker, no PostgreSQL, no Redis. The app creates `./data/taxbyte.db` automatically and starts on `http://127.0.0.1:8080`.

### PostgreSQL (Production)

```bash
docker-compose up -d    # Start PostgreSQL + Redis

# Configure backend in .env or config/local.toml:
#   TAXBYTE_DATABASE__BACKEND=postgres
#   TAXBYTE_DATABASE__URL=postgres://postgres:postgres@localhost:5432/taxbyte

cargo run
```

## Database Backends

TaxByte supports two database backends, selectable via configuration:

| Backend | Default? | Dependencies | Best For |
|---|---|---|---|
| **SQLite** | Yes | None | Local development, small/solo deployments |
| **PostgreSQL** | No | PostgreSQL + Redis | Production, multi-user deployments |

Configure in `config/default.toml` or via environment variables:

```toml
[database]
backend = "sqlite"                              # or "postgres"
url = "sqlite://./data/taxbyte.db?mode=rwc"     # or postgres://...
```

Environment variable override: `TAXBYTE_DATABASE__BACKEND=postgres`

When using SQLite:
- Sessions are stored directly in SQLite (no Redis needed)
- Database file is created automatically with WAL mode and foreign keys enabled
- All data types (UUIDs, timestamps, decimals) stored as TEXT for portability

When using PostgreSQL:
- Redis is required for session caching
- Native PostgreSQL types (UUID, TIMESTAMPTZ, JSONB, DECIMAL, INET)
- 15 incremental migration files in `migrations/postgresql/`

## Architecture Overview

This project follows **Hexagonal Architecture** (Ports and Adapters) to ensure clean separation of concerns:

```
src/
├── domain/              # Core business logic (no external dependencies)
│   ├── auth/            # Authentication domain (User, Session, LoginAttempt)
│   ├── company/         # Company domain (Company, Member, BankAccount)
│   └── invoice/         # Invoice domain (Customer, Invoice, Template)
│
├── application/         # Use cases and orchestration
│   ├── auth/            # Auth use cases (register, login, logout)
│   ├── company/         # Company use cases (create, manage members, OAuth)
│   └── invoice/         # Invoice use cases (CRUD, PDF, templates)
│
├── adapters/            # External interfaces
│   └── http/            # HTTP adapter (REST API + server-rendered UI)
│       ├── handlers/    # Request handlers (API + Web)
│       ├── middleware/   # Auth, request ID, company context
│       └── routes.rs    # Route configuration
│
└── infrastructure/      # Technical implementations
    ├── config.rs        # Configuration management
    ├── persistence/
    │   ├── postgres/    # PostgreSQL repositories (13 files)
    │   └── sqlite/      # SQLite repositories (13 files)
    ├── security/        # Argon2 hasher, token generation, AES encryption
    ├── cloud/           # Google Drive integration, OAuth
    └── pdf/             # PDF generation (wkhtmltopdf)
```

### Key Architectural Principles

1. **Domain-Driven Design**: Business logic is isolated in the domain layer
2. **Dependency Inversion**: Core domain depends on abstractions (`Arc<dyn Trait>`), not implementations
3. **Testability**: Each layer can be tested independently
4. **Flexibility**: Database backend is swappable at startup via configuration

## Prerequisites

- **Rust** (1.85 or later): [Install Rust](https://www.rust-lang.org/tools/install)
- **Docker** (optional, only needed for PostgreSQL mode): [Install Docker](https://docs.docker.com/get-docker/)

## Setup Instructions

### 1. Clone and Run

```bash
git clone <repository-url>
cd taxbyte
cargo run
```

The server starts on `http://127.0.0.1:8080`. Visit the URL in your browser to access the web UI.

### 2. Configure Environment (Optional)

Copy and edit the environment file for custom settings:

```bash
cp .env .env.local
```

Key configuration (all optional, sensible defaults provided):

| Environment Variable | Description | Default |
|---|---|---|
| `TAXBYTE_DATABASE__BACKEND` | Database backend (`sqlite` or `postgres`) | `sqlite` |
| `TAXBYTE_DATABASE__URL` | Database connection URL | `sqlite://./data/taxbyte.db?mode=rwc` |
| `TAXBYTE_REDIS__URL` | Redis URL (PostgreSQL mode only) | `redis://localhost:6379` |
| `TAXBYTE_SERVER__PORT` | Server port | `8080` |
| `TAXBYTE_SECURITY__ENCRYPTION_KEY_BASE64` | AES encryption key for OAuth tokens | (dev default in config) |
| `RUST_LOG` | Logging level | `taxbyte=debug,actix_web=info` |

### 3. Verify Installation

```bash
curl http://localhost:8080/health
# Response: OK
```

## Features

### Implemented

**Authentication & User Management:**
- User registration and authentication
- Session management (Redis with PostgreSQL, SQLite-only otherwise)
- Password hashing with Argon2id
- Rate limiting for login attempts
- Multi-device logout support
- Cookie-based web authentication + Bearer token API auth

**Company Management:**
- Multi-tenant company system
- Company creation and switching
- Team member management (add/remove)
- Role-based access (owner/admin/member)
- Company profile with address, tax ID, VAT number
- Bank account management

**Invoice Management:**
- Customer management with addresses
- Invoice creation and editing
- Invoice templates (create from invoice, create invoice from template)
- PDF generation (wkhtmltopdf)
- Google Drive integration (OAuth 2.0, upload PDFs)
- Invoice status workflow (draft, sent, paid, cancelled)

**Infrastructure:**
- Dual database backend (SQLite default, PostgreSQL optional)
- Server-side rendered UI (Tera + HTMX + Tailwind CSS + Alpine.js)
- RESTful HTTP API with actix-web
- Structured logging with tracing
- Database migrations (separate for each backend)
- Configuration management with environment variable overrides

### Upcoming

- Email verification
- Password reset flow
- Two-factor authentication
- Reporting and analytics

## API Endpoints

### Authentication (REST API - JSON)

| Method | Path | Description | Auth |
|---|---|---|---|
| POST | `/api/v1/auth/register` | Register user | No |
| POST | `/api/v1/auth/login` | Login | No |
| POST | `/api/v1/auth/logout` | Logout current session | Bearer |
| POST | `/api/v1/auth/logout-all` | Logout all devices | Bearer |
| GET | `/api/v1/auth/me` | Get current user | Bearer |

### Companies (REST API - JSON)

| Method | Path | Description | Auth |
|---|---|---|---|
| POST | `/api/v1/companies` | Create company | Bearer |
| GET | `/api/v1/companies` | List user's companies | Bearer |
| POST | `/api/v1/companies/active` | Set active company | Bearer |
| POST | `/api/v1/companies/:id/members` | Add member | Bearer |
| DELETE | `/api/v1/companies/:id/members/:user_id` | Remove member | Bearer |

### Customers & Invoices (REST API - JSON)

| Method | Path | Description | Auth |
|---|---|---|---|
| POST | `/api/v1/customers` | Create customer | Bearer |
| GET | `/api/v1/customers` | List customers | Bearer |
| POST | `/api/v1/invoices` | Create invoice | Bearer |
| GET | `/api/v1/invoices` | List invoices | Bearer |

### Web UI (Server-Rendered HTML)

| Path | Description |
|---|---|
| `/login` | Login page |
| `/register` | Register page |
| `/dashboard` | Dashboard (authenticated) |
| `/companies` | Company management |
| `/companies/:id/members` | Team members |
| `/customers` | Customer management |
| `/invoices` | Invoice management |

## Testing

```bash
# Run all unit tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test module
cargo test domain::auth

# Integration tests (requires Docker for PostgreSQL tests)
cargo test -- --ignored
```

Integration tests use testcontainers to spin up real PostgreSQL instances automatically.

## Code Quality

```bash
cargo fmt                 # Format code
cargo clippy              # Lint
cargo check               # Quick compile check
```

## Database Migrations

All migrations live under `migrations/`, organized by backend:

```
migrations/
├── postgresql/    # 15 incremental migration files
└── sqlite/        # 1 consolidated schema file
```

### SQLite

SQLite uses a single consolidated migration file: `migrations/sqlite/20260131000001_initial_schema.sql`

When adding schema changes, update this file to reflect the final desired schema state.

### PostgreSQL

PostgreSQL uses incremental migration files in `migrations/postgresql/`:

```bash
# Create a new migration
sqlx migrate add -r --source migrations/postgresql <migration_name>

# Run migrations manually
sqlx migrate run --source migrations/postgresql --database-url "postgres://postgres:postgres@localhost:5432/taxbyte"
```

**Important:** When changing the schema, update both `migrations/postgresql/` (new incremental file) and `migrations/sqlite/` (update consolidated file).

## Docker Commands (PostgreSQL Mode)

```bash
docker-compose up -d          # Start PostgreSQL + Redis
docker-compose down           # Stop services
docker-compose down -v        # Stop and reset all data
docker-compose logs -f        # View logs

# Access PostgreSQL
docker exec -it taxbyte-postgres psql -U taxbyte -d taxbyte

# Access Redis
docker exec -it taxbyte-redis redis-cli
```

## Running via Docker (GHCR image)

Prebuilt images are published to GitHub Container Registry on every `v*` tag:

```bash
docker pull ghcr.io/konstantinzolotarev/taxbyte:latest

docker run --rm -p 8080:8080 \
  -e TAXBYTE_SECURITY__ENCRYPTION_KEY_BASE64="$(openssl rand -base64 32)" \
  -v taxbyte-data:/app/data \
  ghcr.io/konstantinzolotarev/taxbyte:latest
```

The image uses the SQLite backend by default and persists the database in the
`/app/data` volume. `wkhtmltopdf` is installed in the image for PDF generation.

**Google Drive OAuth (optional):** if credentials are not provided, the app
starts with a mock OAuth manager and Google Drive integration is disabled. To
enable it, supply:

```
TAXBYTE_GOOGLE_DRIVE__OAUTH_CLIENT_ID=...
TAXBYTE_GOOGLE_DRIVE__OAUTH_CLIENT_SECRET=...
TAXBYTE_GOOGLE_DRIVE__OAUTH_REDIRECT_URL=https://your-host/oauth/google/callback
```

Set `MOCK_OAUTH=true` to explicitly force the mock manager (useful for local dev).

## Releasing

Releases are cut from `v*` tags. See `.github/workflows/release.yml`.

```bash
# 1. Bump version in Cargo.toml
# 2. cargo update -p taxbyte   # refresh Cargo.lock
# 3. git commit -am "Release vX.Y.Z"
# 4. git tag vX.Y.Z && git push && git push --tags
```

The workflow verifies the tag matches `Cargo.toml`, builds a multi-arch
(`linux/amd64`, `linux/arm64`) Docker image, pushes to `ghcr.io`, and creates
a GitHub release with auto-generated notes.

## Configuration

Configuration is loaded in priority order (later overrides earlier):

1. `config/default.toml` -- Default values (committed)
2. `config/local.toml` -- Local overrides (gitignored)
3. `config/{RUN_MODE}.toml` -- Environment-specific (optional)
4. Environment variables with `TAXBYTE_` prefix (highest priority)

Environment variables use double underscores as separators: `TAXBYTE_DATABASE__BACKEND=postgres`

## Troubleshooting

### SQLite Mode

**"Failed to run SQLite migrations"**: Check that the `data/` directory is writable. The app creates it automatically, but permissions may vary.

**Database locked**: SQLite WAL mode handles most concurrency, but if you see locking errors under heavy load, consider switching to PostgreSQL.

### PostgreSQL Mode

**"Database connection timed out"**: Ensure Docker containers are running (`docker-compose ps`) and the `DATABASE_URL` is correct.

**"Redis connection failed"**: Redis is required in PostgreSQL mode. Check `docker-compose logs redis`.

**Migration errors**: Reset with `docker-compose down -v && docker-compose up -d && cargo run`.

## License

This project is proprietary and confidential.
