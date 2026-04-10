# syntax=docker/dockerfile:1.7

# ---- Builder stage ----
FROM rust:1-bookworm AS builder

WORKDIR /app

# Copy manifests and sources. Migrations are needed at compile time because
# sqlx::migrate!() is a macro that reads the migration files.
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations

RUN cargo build --release --locked

# ---- Runtime stage ----
FROM debian:bookworm-slim AS runtime

ENV DEBIAN_FRONTEND=noninteractive

# wkhtmltopdf + runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
      wkhtmltopdf \
      ca-certificates \
      libssl3 \
      fontconfig \
      libjpeg62-turbo \
      libxrender1 \
      libxext6 \
      xfonts-base \
      xfonts-75dpi \
    && rm -rf /var/lib/apt/lists/*

# Non-root user
RUN useradd --system --create-home --home-dir /home/taxbyte --shell /usr/sbin/nologin taxbyte \
    && mkdir -p /app/data \
    && chown -R taxbyte:taxbyte /app

WORKDIR /app

# Binary
COPY --from=builder /app/target/release/taxbyte /usr/local/bin/taxbyte

# Runtime assets (binary reads relative paths from WORKDIR)
COPY --chown=taxbyte:taxbyte templates ./templates
COPY --chown=taxbyte:taxbyte static ./static
COPY --chown=taxbyte:taxbyte migrations ./migrations
COPY --chown=taxbyte:taxbyte config ./config

USER taxbyte

EXPOSE 8080

ENV TAXBYTE_SERVER__HOST=0.0.0.0 \
    TAXBYTE_SERVER__PORT=8080 \
    TAXBYTE_DATABASE__URL=sqlite:///app/data/taxbyte.db

VOLUME ["/app/data"]

CMD ["taxbyte"]
