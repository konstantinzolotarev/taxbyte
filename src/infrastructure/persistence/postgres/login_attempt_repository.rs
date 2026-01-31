use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use std::net::IpAddr;
use uuid::Uuid;

use crate::domain::auth::{
  entities::LoginAttempt,
  errors::{AuthError, RepositoryError},
  ports::LoginAttemptRepository,
};

/// Database row structure for login_attempts table
#[derive(Debug, FromRow)]
struct LoginAttemptRow {
  id: Uuid,
  email: String,
  ip_address: String,
  success: bool,
  attempted_at: DateTime<Utc>,
}

/// Database row structure for scalar email query
#[derive(Debug, FromRow)]
struct EmailRow {
  email: String,
}

/// Database row structure for scalar count query
#[derive(Debug, FromRow)]
struct CountRow {
  count: Option<i64>,
}

/// PostgreSQL implementation of the LoginAttemptRepository trait
pub struct PostgresLoginAttemptRepository {
  pool: PgPool,
}

impl PostgresLoginAttemptRepository {
  /// Creates a new PostgresLoginAttemptRepository instance
  pub fn new(pool: PgPool) -> Self {
    Self { pool }
  }
}

#[async_trait]
impl LoginAttemptRepository for PostgresLoginAttemptRepository {
  /// Records a new login attempt in the database
  async fn create(&self, attempt: LoginAttempt) -> Result<LoginAttempt, AuthError> {
    // Convert IpAddr to string for PostgreSQL INET type
    let ip_string = attempt.ip_address.to_string();

    let row = sqlx::query_as::<_, LoginAttemptRow>(
      r#"
            INSERT INTO login_attempts (id, email, ip_address, success, attempted_at)
            VALUES ($1, $2, CAST($3 AS INET), $4, $5)
            RETURNING id, email, HOST(ip_address) as ip_address, success, attempted_at
            "#,
    )
    .bind(attempt.id)
    .bind(&attempt.email)
    .bind(ip_string)
    .bind(attempt.success)
    .bind(attempt.attempted_at)
    .fetch_one(&self.pool)
    .await
    .map_err(|e| {
      tracing::error!("Failed to create login attempt: {}", e);
      AuthError::Repository(RepositoryError::QueryFailed(e.to_string()))
    })?;

    // Parse IP address from database
    let ip_address: IpAddr = row.ip_address.parse().map_err(|e| {
      tracing::error!("Failed to parse IP address: {}", e);
      AuthError::Repository(RepositoryError::DatabaseError(format!(
        "Invalid IP address: {}",
        e
      )))
    })?;

    Ok(LoginAttempt::from_db(
      row.id,
      row.email,
      ip_address,
      row.success,
      row.attempted_at,
    ))
  }

  /// Counts the number of recent failed login attempts for a user
  /// within a specified time window (in seconds)
  async fn count_recent_failures(
    &self,
    user_id: Uuid,
    window_seconds: i64,
  ) -> Result<i64, AuthError> {
    // First, get the user's email from the users table
    let user_email_row = sqlx::query_as::<_, EmailRow>(
      r#"
            SELECT email
            FROM users
            WHERE id = $1
            "#,
    )
    .bind(user_id)
    .fetch_optional(&self.pool)
    .await
    .map_err(|e| {
      tracing::error!("Failed to fetch user email for user_id {}: {}", user_id, e);
      AuthError::Repository(RepositoryError::QueryFailed(e.to_string()))
    })?
    .ok_or_else(|| {
      tracing::warn!("User not found for user_id: {}", user_id);
      AuthError::Repository(RepositoryError::NotFound)
    })?;

    let user_email = user_email_row.email;

    // Count failed login attempts within the time window
    let count_row = sqlx::query_as::<_, CountRow>(
      r#"
            SELECT COUNT(*) as count
            FROM login_attempts
            WHERE email = $1
              AND success = false
              AND attempted_at >= NOW() - INTERVAL '1 second' * $2
            "#,
    )
    .bind(&user_email)
    .bind(window_seconds)
    .fetch_one(&self.pool)
    .await
    .map_err(|e| {
      tracing::error!(
        "Failed to count login failures for email {}: {}",
        user_email,
        e
      );
      AuthError::Repository(RepositoryError::QueryFailed(e.to_string()))
    })?;

    Ok(count_row.count.unwrap_or(0))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use sqlx::postgres::PgPoolOptions;
  use testcontainers::ImageExt;
  use testcontainers_modules::postgres::Postgres;
  use testcontainers_modules::testcontainers::{runners::AsyncRunner, ContainerAsync};

  async fn setup_test_db() -> (PgPool, ContainerAsync<Postgres>) {
    // Start a PostgreSQL container
    let container = Postgres::default()
      .with_tag("16-alpine")
      .start()
      .await
      .expect("Failed to start postgres container");

    // Build connection string
    let host = container.get_host().await.expect("Failed to get host");
    let port = container
      .get_host_port_ipv4(5432)
      .await
      .expect("Failed to get port");
    let database_url = format!(
      "postgres://postgres:postgres@{}:{}/postgres",
      host, port
    );

    // Connect to the database
    let pool = PgPoolOptions::new()
      .max_connections(5)
      .connect(&database_url)
      .await
      .expect("Failed to connect to test database");

    // Run migrations
    sqlx::migrate!("./migrations")
      .run(&pool)
      .await
      .expect("Failed to run migrations");

    (pool, container)
  }

  #[tokio::test]
  async fn test_create_login_attempt() {
    let (pool, _container) = setup_test_db().await;
    let repo = PostgresLoginAttemptRepository::new(pool);

    let ip: IpAddr = "192.168.1.1".parse().unwrap();
    let attempt = LoginAttempt::new("test@example.com".to_string(), ip, false);

    let result = repo.create(attempt.clone()).await;
    assert!(result.is_ok());

    let created = result.unwrap();
    assert_eq!(created.email, attempt.email);
    assert_eq!(created.ip_address, attempt.ip_address);
    assert_eq!(created.success, attempt.success);
  }

  #[tokio::test]
  async fn test_count_recent_failures() {
    let (pool, _container) = setup_test_db().await;
    let repo = PostgresLoginAttemptRepository::new(pool.clone());

    // Create a test user first
    let user_id = Uuid::new_v4();
    let email = format!("test_{}@example.com", user_id);

    sqlx::query(
            r#"
            INSERT INTO users (id, email, password_hash, full_name, is_email_verified, created_at, updated_at)
            VALUES ($1, $2, 'hash', 'Test User', false, NOW(), NOW())
            "#
        )
        .bind(user_id)
        .bind(&email)
        .execute(&pool)
        .await
        .expect("Failed to create test user");

    // Create some failed login attempts
    let ip: IpAddr = "192.168.1.1".parse().unwrap();
    for _ in 0..3 {
      let attempt = LoginAttempt::failure(email.clone(), ip);
      repo
        .create(attempt)
        .await
        .expect("Failed to create attempt");
    }

    // Count recent failures
    let count = repo.count_recent_failures(user_id, 300).await;
    assert!(count.is_ok());
    assert_eq!(count.unwrap(), 3);

    // Clean up
    sqlx::query("DELETE FROM login_attempts WHERE email = $1")
      .bind(&email)
      .execute(&pool)
      .await
      .expect("Failed to clean up login attempts");

    sqlx::query("DELETE FROM users WHERE id = $1")
      .bind(user_id)
      .execute(&pool)
      .await
      .expect("Failed to clean up user");
  }
}
