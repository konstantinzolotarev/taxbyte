use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use sqlx::{FromRow, SqlitePool};
use std::net::IpAddr;
use uuid::Uuid;

use crate::domain::auth::entities::Session;
use crate::domain::auth::errors::{AuthError, RepositoryError};
use crate::domain::auth::ports::SessionRepository;

#[derive(Debug, FromRow)]
struct SessionRow {
  id: String,
  user_id: String,
  session_token: String,
  ip_address: Option<String>,
  user_agent: Option<String>,
  expires_at: String,
  created_at: String,
}

fn parse_session_row(row: SessionRow) -> Result<Session, AuthError> {
  let id = Uuid::parse_str(&row.id)
    .map_err(|e| AuthError::Repository(RepositoryError::QueryFailed(e.to_string())))?;
  let user_id = Uuid::parse_str(&row.user_id)
    .map_err(|e| AuthError::Repository(RepositoryError::QueryFailed(e.to_string())))?;
  let ip_address = row
    .ip_address
    .and_then(|ip_str| ip_str.parse::<IpAddr>().ok());
  let expires_at = DateTime::parse_from_rfc3339(&row.expires_at)
    .map(|dt| dt.with_timezone(&Utc))
    .map_err(|e| AuthError::Repository(RepositoryError::QueryFailed(e.to_string())))?;
  let created_at = DateTime::parse_from_rfc3339(&row.created_at)
    .map(|dt| dt.with_timezone(&Utc))
    .map_err(|e| AuthError::Repository(RepositoryError::QueryFailed(e.to_string())))?;

  Ok(Session::from_db(
    id,
    user_id,
    row.session_token,
    ip_address,
    row.user_agent,
    expires_at,
    created_at,
  ))
}

/// SQLite implementation of the SessionRepository trait.
/// Sessions are stored directly in SQLite (no Redis caching layer needed for local dev).
pub struct SqliteSessionRepository {
  pool: SqlitePool,
}

impl SqliteSessionRepository {
  pub fn new(pool: SqlitePool) -> Self {
    Self { pool }
  }
}

#[async_trait]
impl SessionRepository for SqliteSessionRepository {
  async fn create(&self, session: Session) -> Result<Session, AuthError> {
    let ip_address = session.ip_address.map(|ip| ip.to_string());

    let row = sqlx::query_as::<_, SessionRow>(
      r#"
      INSERT INTO sessions (id, user_id, session_token, ip_address, user_agent, expires_at, created_at)
      VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
      RETURNING id, user_id, session_token, ip_address, user_agent, expires_at, created_at
      "#,
    )
    .bind(session.id.to_string())
    .bind(session.user_id.to_string())
    .bind(&session.session_token)
    .bind(ip_address.as_deref())
    .bind(session.user_agent.as_deref())
    .bind(session.expires_at.to_rfc3339())
    .bind(session.created_at.to_rfc3339())
    .fetch_one(&self.pool)
    .await?;

    parse_session_row(row)
  }

  async fn find_by_token_hash(&self, token_hash: &str) -> Result<Option<Session>, AuthError> {
    let row = sqlx::query_as::<_, SessionRow>(
      r#"
      SELECT id, user_id, session_token, ip_address, user_agent, expires_at, created_at
      FROM sessions
      WHERE session_token = ?1
      "#,
    )
    .bind(token_hash)
    .fetch_optional(&self.pool)
    .await?;

    match row {
      Some(row) => Ok(Some(parse_session_row(row)?)),
      None => Ok(None),
    }
  }

  async fn find_by_user_id(&self, user_id: Uuid) -> Result<Vec<Session>, AuthError> {
    let rows = sqlx::query_as::<_, SessionRow>(
      r#"
      SELECT id, user_id, session_token, ip_address, user_agent, expires_at, created_at
      FROM sessions
      WHERE user_id = ?1
      ORDER BY created_at DESC
      "#,
    )
    .bind(user_id.to_string())
    .fetch_all(&self.pool)
    .await?;

    rows.into_iter().map(parse_session_row).collect()
  }

  /// Updates the last activity timestamp for a session.
  /// Extends the session expiration by 30 minutes from the current expiry.
  async fn update_activity(&self, session_id: Uuid) -> Result<(), AuthError> {
    // Compute the new expiry: current expiry + 30 minutes.
    // We read the current expiry, add 30 minutes in Rust, and write it back.
    let row = sqlx::query_as::<_, (String,)>("SELECT expires_at FROM sessions WHERE id = ?1")
      .bind(session_id.to_string())
      .fetch_optional(&self.pool)
      .await?;

    let row = row.ok_or_else(|| {
      tracing::warn!("Session {} not found for activity update", session_id);
      AuthError::Repository(RepositoryError::NotFound)
    })?;

    let current_expires = DateTime::parse_from_rfc3339(&row.0)
      .map(|dt| dt.with_timezone(&Utc))
      .map_err(|e| AuthError::Repository(RepositoryError::QueryFailed(e.to_string())))?;

    let new_expires = current_expires + Duration::minutes(30);

    sqlx::query("UPDATE sessions SET expires_at = ?1 WHERE id = ?2")
      .bind(new_expires.to_rfc3339())
      .bind(session_id.to_string())
      .execute(&self.pool)
      .await?;

    Ok(())
  }

  async fn delete(&self, session_id: Uuid) -> Result<(), AuthError> {
    let result = sqlx::query("DELETE FROM sessions WHERE id = ?1")
      .bind(session_id.to_string())
      .execute(&self.pool)
      .await?;

    if result.rows_affected() == 0 {
      tracing::warn!("Session {} not found for deletion", session_id);
      return Err(AuthError::Repository(RepositoryError::NotFound));
    }

    Ok(())
  }

  async fn delete_all_for_user(&self, user_id: Uuid) -> Result<(), AuthError> {
    sqlx::query("DELETE FROM sessions WHERE user_id = ?1")
      .bind(user_id.to_string())
      .execute(&self.pool)
      .await?;

    tracing::info!("Deleted all sessions for user {}", user_id);
    Ok(())
  }
}
