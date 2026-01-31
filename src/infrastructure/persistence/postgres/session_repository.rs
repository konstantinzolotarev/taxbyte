use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use std::net::IpAddr;
use uuid::Uuid;

use crate::domain::auth::entities::Session;
use crate::domain::auth::errors::{AuthError, RepositoryError};
use crate::domain::auth::ports::SessionRepository;

/// Database row structure for sessions table
#[derive(Debug, FromRow)]
struct SessionRow {
  id: Uuid,
  user_id: Uuid,
  session_token: String,
  ip_address: Option<String>,
  user_agent: Option<String>,
  expires_at: DateTime<Utc>,
  created_at: DateTime<Utc>,
}

/// PostgreSQL implementation of the SessionRepository trait
pub struct PostgresSessionRepository {
  pool: PgPool,
  #[allow(dead_code)] // Redis connection for future caching functionality
  redis: Option<redis::aio::ConnectionManager>,
}

impl PostgresSessionRepository {
  /// Creates a new PostgresSessionRepository with the given connection pool
  pub fn new(pool: PgPool, redis: redis::aio::ConnectionManager) -> Self {
    Self {
      pool,
      redis: Some(redis),
    }
  }

  /// Creates a new PostgresSessionRepository without Redis caching
  #[allow(dead_code)]
  pub fn without_redis(pool: PgPool) -> Self {
    Self { pool, redis: None }
  }
}

#[async_trait]
impl SessionRepository for PostgresSessionRepository {
  /// Creates a new session in the database
  async fn create(&self, session: Session) -> Result<Session, AuthError> {
    let ip_address = session.ip_address.map(|ip| ip.to_string());

    let row = sqlx::query_as::<_, SessionRow>(
            r#"
            INSERT INTO sessions (id, user_id, session_token, ip_address, user_agent, expires_at, created_at)
            VALUES ($1, $2, $3, CAST($4 AS INET), $5, $6, $7)
            RETURNING id, user_id, session_token, HOST(ip_address) as ip_address, user_agent, expires_at, created_at
            "#
        )
        .bind(session.id)
        .bind(session.user_id)
        .bind(&session.session_token)
        .bind(ip_address.as_deref())
        .bind(session.user_agent.as_deref())
        .bind(session.expires_at)
        .bind(session.created_at)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create session: {}", e);
            AuthError::Repository(RepositoryError::QueryFailed(e.to_string()))
        })?;

    let ip_address = row
      .ip_address
      .and_then(|ip_str| ip_str.parse::<IpAddr>().ok());

    Ok(Session::from_db(
      row.id,
      row.user_id,
      row.session_token,
      ip_address,
      row.user_agent,
      row.expires_at,
      row.created_at,
    ))
  }

  /// Finds a session by its token hash
  async fn find_by_token_hash(&self, token_hash: &str) -> Result<Option<Session>, AuthError> {
    let row = sqlx::query_as::<_, SessionRow>(
      r#"
            SELECT id, user_id, session_token, HOST(ip_address) as ip_address, user_agent, expires_at, created_at
            FROM sessions
            WHERE session_token = $1
            "#,
    )
    .bind(token_hash)
    .fetch_optional(&self.pool)
    .await
    .map_err(|e| {
      tracing::error!("Failed to find session by token hash: {}", e);
      AuthError::Repository(RepositoryError::QueryFailed(e.to_string()))
    })?;

    match row {
      Some(row) => {
        let ip_address = row
          .ip_address
          .and_then(|ip_str| ip_str.parse::<IpAddr>().ok());

        Ok(Some(Session::from_db(
          row.id,
          row.user_id,
          row.session_token,
          ip_address,
          row.user_agent,
          row.expires_at,
          row.created_at,
        )))
      }
      None => Ok(None),
    }
  }

  /// Finds all active sessions for a specific user
  async fn find_by_user_id(&self, user_id: Uuid) -> Result<Vec<Session>, AuthError> {
    let rows = sqlx::query_as::<_, SessionRow>(
      r#"
            SELECT id, user_id, session_token, HOST(ip_address) as ip_address, user_agent, expires_at, created_at
            FROM sessions
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#,
    )
    .bind(user_id)
    .fetch_all(&self.pool)
    .await
    .map_err(|e| {
      tracing::error!("Failed to find sessions by user_id: {}", e);
      AuthError::Repository(RepositoryError::QueryFailed(e.to_string()))
    })?;

    let sessions = rows
      .into_iter()
      .map(|row| {
        let ip_address = row
          .ip_address
          .and_then(|ip_str| ip_str.parse::<IpAddr>().ok());

        Session::from_db(
          row.id,
          row.user_id,
          row.session_token,
          ip_address,
          row.user_agent,
          row.expires_at,
          row.created_at,
        )
      })
      .collect();

    Ok(sessions)
  }

  /// Updates the last activity timestamp for a session
  /// This extends the session expiration time
  async fn update_activity(&self, session_id: Uuid) -> Result<(), AuthError> {
    let result = sqlx::query(
      r#"
            UPDATE sessions
            SET expires_at = expires_at + INTERVAL '30 minutes'
            WHERE id = $1
            "#,
    )
    .bind(session_id)
    .execute(&self.pool)
    .await
    .map_err(|e| {
      tracing::error!("Failed to update session activity: {}", e);
      AuthError::Repository(RepositoryError::QueryFailed(e.to_string()))
    })?;

    if result.rows_affected() == 0 {
      tracing::warn!("Session {} not found for activity update", session_id);
      return Err(AuthError::Repository(RepositoryError::NotFound));
    }

    Ok(())
  }

  /// Deletes a specific session
  async fn delete(&self, session_id: Uuid) -> Result<(), AuthError> {
    let result = sqlx::query(
      r#"
            DELETE FROM sessions
            WHERE id = $1
            "#,
    )
    .bind(session_id)
    .execute(&self.pool)
    .await
    .map_err(|e| {
      tracing::error!("Failed to delete session: {}", e);
      AuthError::Repository(RepositoryError::QueryFailed(e.to_string()))
    })?;

    if result.rows_affected() == 0 {
      tracing::warn!("Session {} not found for deletion", session_id);
      return Err(AuthError::Repository(RepositoryError::NotFound));
    }

    Ok(())
  }

  /// Deletes all sessions for a specific user
  /// Useful for logout from all devices or when a user changes their password
  async fn delete_all_for_user(&self, user_id: Uuid) -> Result<(), AuthError> {
    sqlx::query(
      r#"
            DELETE FROM sessions
            WHERE user_id = $1
            "#,
    )
    .bind(user_id)
    .execute(&self.pool)
    .await
    .map_err(|e| {
      tracing::error!("Failed to delete all sessions for user {}: {}", user_id, e);
      AuthError::Repository(RepositoryError::QueryFailed(e.to_string()))
    })?;

    tracing::info!("Deleted all sessions for user {}", user_id);
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use chrono::Duration;
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

  async fn create_test_user(pool: &PgPool) -> Uuid {
    let user_id = Uuid::new_v4();
    let email = format!("test_{}@example.com", user_id);
    sqlx::query(
      r#"
            INSERT INTO users (id, email, password_hash, full_name, is_email_verified, created_at, updated_at)
            VALUES ($1, $2, 'hash', 'Test User', false, NOW(), NOW())
            "#,
    )
    .bind(user_id)
    .bind(&email)
    .execute(pool)
    .await
    .expect("Failed to create test user");
    user_id
  }

  #[tokio::test]
  async fn test_create_session() {
    let (pool, _container) = setup_test_db().await;
    let repo = PostgresSessionRepository::without_redis(pool.clone());

    let user_id = create_test_user(&pool).await;
    let session = Session::with_duration(
      user_id,
      "test_token_hash".to_string(),
      Duration::hours(1),
      Some("127.0.0.1".parse().unwrap()),
      Some("Mozilla/5.0".to_string()),
    );

    let created_session = repo.create(session.clone()).await.unwrap();

    assert_eq!(created_session.id, session.id);
    assert_eq!(created_session.user_id, user_id);
    assert_eq!(created_session.session_token, "test_token_hash");
  }

  #[tokio::test]
  async fn test_find_by_token_hash() {
    let (pool, _container) = setup_test_db().await;
    let repo = PostgresSessionRepository::without_redis(pool.clone());

    let user_id = create_test_user(&pool).await;
    let session = Session::with_duration(
      user_id,
      "unique_token_hash".to_string(),
      Duration::hours(1),
      None,
      None,
    );

    repo.create(session.clone()).await.unwrap();

    let found_session = repo.find_by_token_hash("unique_token_hash").await.unwrap();

    assert!(found_session.is_some());
    let found_session = found_session.unwrap();
    assert_eq!(found_session.user_id, user_id);
  }

  #[tokio::test]
  async fn test_find_by_user_id() {
    let (pool, _container) = setup_test_db().await;
    let repo = PostgresSessionRepository::without_redis(pool.clone());

    let user_id = create_test_user(&pool).await;

    // Create multiple sessions for the same user
    for i in 0..3 {
      let session = Session::with_duration(
        user_id,
        format!("token_hash_{}", i),
        Duration::hours(1),
        None,
        None,
      );
      repo.create(session).await.unwrap();
    }

    let sessions = repo.find_by_user_id(user_id).await.unwrap();

    assert_eq!(sessions.len(), 3);
    assert!(sessions.iter().all(|s| s.user_id == user_id));
  }

  #[tokio::test]
  async fn test_delete_session() {
    let (pool, _container) = setup_test_db().await;
    let repo = PostgresSessionRepository::without_redis(pool.clone());

    let user_id = create_test_user(&pool).await;
    let session = Session::with_duration(
      user_id,
      "to_delete".to_string(),
      Duration::hours(1),
      None,
      None,
    );

    let created_session = repo.create(session).await.unwrap();

    // Delete the session
    repo.delete(created_session.id).await.unwrap();

    // Verify it's gone
    let found = repo.find_by_token_hash("to_delete").await.unwrap();
    assert!(found.is_none());
  }

  #[tokio::test]
  async fn test_delete_all_for_user() {
    let (pool, _container) = setup_test_db().await;
    let repo = PostgresSessionRepository::without_redis(pool.clone());

    let user_id = create_test_user(&pool).await;

    // Create multiple sessions
    for i in 0..3 {
      let session = Session::with_duration(
        user_id,
        format!("token_{}", i),
        Duration::hours(1),
        None,
        None,
      );
      repo.create(session).await.unwrap();
    }

    // Delete all sessions for the user
    repo.delete_all_for_user(user_id).await.unwrap();

    // Verify they're all gone
    let sessions = repo.find_by_user_id(user_id).await.unwrap();
    assert_eq!(sessions.len(), 0);
  }
}
