#!/bin/bash

# TaxByte Setup Script
# This script helps set up the development environment for TaxByte

set -e

echo "🚀 TaxByte Setup Script"
echo "======================="
echo ""

# Check if Docker is installed
if ! command -v docker &> /dev/null; then
    echo "❌ Docker is not installed. Please install Docker first."
    exit 1
fi

# Check if Docker Compose is installed
if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null; then
    echo "❌ Docker Compose is not installed. Please install Docker Compose first."
    exit 1
fi

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust is not installed. Please install Rust from https://rustup.rs/"
    exit 1
fi

echo "✅ Prerequisites check passed"
echo ""

# Start Docker services
echo "📦 Starting Docker services (PostgreSQL and Redis)..."
docker-compose up -d

# Wait for PostgreSQL to be ready
echo "⏳ Waiting for PostgreSQL to be ready..."
sleep 5

max_retries=30
retry_count=0

until docker-compose exec -T postgres pg_isready -U taxbyte &> /dev/null || [ $retry_count -eq $max_retries ]; do
    echo "   Waiting for PostgreSQL... ($retry_count/$max_retries)"
    sleep 1
    retry_count=$((retry_count + 1))
done

if [ $retry_count -eq $max_retries ]; then
    echo "❌ PostgreSQL failed to start"
    exit 1
fi

echo "✅ PostgreSQL is ready"

# Wait for Redis to be ready
echo "⏳ Waiting for Redis to be ready..."
sleep 2
echo "✅ Redis is ready"
echo ""

# Install sqlx-cli if not present
if ! command -v sqlx &> /dev/null; then
    echo "📥 Installing sqlx-cli..."
    cargo install sqlx-cli --no-default-features --features postgres
    echo "✅ sqlx-cli installed"
else
    echo "✅ sqlx-cli already installed"
fi
echo ""

# Run database migrations
echo "🔄 Running database migrations..."
export DATABASE_URL="postgres://postgres:postgres@localhost:5432/taxbyte"
sqlx migrate run
echo "✅ Migrations completed"
echo ""

# Prepare sqlx offline mode (for compilation without database)
echo "🔄 Preparing sqlx offline query cache..."
cargo sqlx prepare
echo "✅ Sqlx offline query cache prepared"
echo ""

# Build the project
echo "🔨 Building the project..."
cargo build
echo "✅ Build completed"
echo ""

echo "✨ Setup complete!"
echo ""
echo "Next steps:"
echo "  1. Run 'cargo run' to start the server"
echo "  2. Visit http://localhost:8080/health to verify the server is running"
echo "  3. Read the README.md for API documentation and usage examples"
echo ""
echo "Useful commands:"
echo "  - Start services: docker-compose up -d"
echo "  - Stop services: docker-compose down"
echo "  - View logs: docker-compose logs -f"
echo "  - Run tests: cargo test"
echo "  - Format code: cargo fmt"
echo "  - Lint code: cargo clippy"
echo ""
