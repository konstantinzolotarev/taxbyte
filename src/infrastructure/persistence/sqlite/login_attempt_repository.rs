use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use sqlx::{FromRow, SqlitePool};
use std::net::IpAddr;
use uuid::Uuid;

use crate::domain::auth::{
  entities::LoginAttempt,
  errors::{AuthError, RepositoryError},
  ports::LoginAttemptRepository,
};

#[derive(Debug, FromRow)]
struct LoginAttemptRow {
  id: String,
  email: String,
  ip_address: String,
  success: bool,
  attempted_at: String,
}

#[derive(Debug, FromRow)]
struct EmailRow {
  email: String,
}

#[derive(Debug, FromRow)]
struct CountRow {
  count: i32,
}

pub struct SqliteLoginAttemptRepository {
  pool: SqlitePool,
}

impl SqliteLoginAttemptRepository {
  pub fn new(pool: SqlitePool) -> Self {
    Self { pool }
  }
}

#[async_trait]
impl LoginAttemptRepository for SqliteLoginAttemptRepository {
  async fn create(&self, attempt: LoginAttempt) -> Result<LoginAttempt, AuthError> {
    let ip_string = attempt.ip_address.to_string();

    let row = sqlx::query_as::<_, LoginAttemptRow>(
      r#"
      INSERT INTO login_attempts (id, email, ip_address, success, attempted_at)
      VALUES (?1, ?2, ?3, ?4, ?5)
      RETURNING id, email, ip_address, success, attempted_at
      "#,
    )
    .bind(attempt.id.to_string())
    .bind(&attempt.email)
    .bind(&ip_string)
    .bind(attempt.success)
    .bind(attempt.attempted_at.to_rfc3339())
    .fetch_one(&self.pool)
    .await?;

    let ip_address: IpAddr = row
      .ip_address
      .parse()
      .map_err(RepositoryError::from)
      .map_err(AuthError::from)?;

    let id = Uuid::parse_str(&row.id)
      .map_err(|e| AuthError::Repository(RepositoryError::QueryFailed(e.to_string())))?;

    let attempted_at = DateTime::parse_from_rfc3339(&row.attempted_at)
      .map(|dt| dt.with_timezone(&Utc))
      .map_err(|e| AuthError::Repository(RepositoryError::QueryFailed(e.to_string())))?;

    Ok(LoginAttempt::from_db(
      id,
      row.email,
      ip_address,
      row.success,
      attempted_at,
    ))
  }

  async fn count_recent_failures(
    &self,
    user_id: Uuid,
    window_seconds: i64,
  ) -> Result<i64, AuthError> {
    // Get the user's email
    let user_email_row = sqlx::query_as::<_, EmailRow>("SELECT email FROM users WHERE id = ?1")
      .bind(user_id.to_string())
      .fetch_optional(&self.pool)
      .await?
      .ok_or_else(|| {
        tracing::warn!("User not found for user_id: {}", user_id);
        AuthError::Repository(RepositoryError::NotFound)
      })?;

    let user_email = user_email_row.email;

    // Compute cutoff timestamp in Rust (instead of using INTERVAL)
    let cutoff = Utc::now() - Duration::seconds(window_seconds);

    let count_row = sqlx::query_as::<_, CountRow>(
      r#"
      SELECT COUNT(*) as count
      FROM login_attempts
      WHERE email = ?1
        AND success = 0
        AND attempted_at >= ?2
      "#,
    )
    .bind(&user_email)
    .bind(cutoff.to_rfc3339())
    .fetch_one(&self.pool)
    .await?;

    Ok(count_row.count as i64)
  }
}
