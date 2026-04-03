use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::domain::auth::{
  entities::User,
  errors::{AuthError, RepositoryError},
  ports::UserRepository,
  value_objects::Email,
};

#[derive(Debug, FromRow)]
struct UserRow {
  id: String,
  email: String,
  password_hash: String,
  full_name: String,
  is_email_verified: bool,
  email_verification_token: Option<String>,
  email_verification_token_expires_at: Option<String>,
  password_reset_token: Option<String>,
  password_reset_token_expires_at: Option<String>,
  created_at: String,
  updated_at: String,
  deleted_at: Option<String>,
}

fn parse_datetime(s: &str) -> Result<DateTime<Utc>, RepositoryError> {
  DateTime::parse_from_rfc3339(s)
    .map(|dt| dt.with_timezone(&Utc))
    .map_err(|e| RepositoryError::QueryFailed(format!("Failed to parse datetime: {}", e)))
}

fn parse_optional_datetime(s: &Option<String>) -> Result<Option<DateTime<Utc>>, RepositoryError> {
  s.as_ref().map(|s| parse_datetime(s)).transpose()
}

fn parse_user_row(row: UserRow) -> Result<User, AuthError> {
  let id = Uuid::parse_str(&row.id)
    .map_err(|e| AuthError::Repository(RepositoryError::QueryFailed(e.to_string())))?;
  let created_at = parse_datetime(&row.created_at)?;
  let updated_at = parse_datetime(&row.updated_at)?;
  let deleted_at = parse_optional_datetime(&row.deleted_at)?;
  let email_verification_token_expires_at =
    parse_optional_datetime(&row.email_verification_token_expires_at)?;
  let password_reset_token_expires_at =
    parse_optional_datetime(&row.password_reset_token_expires_at)?;

  Ok(User::from_db(
    id,
    row.email,
    row.password_hash,
    row.full_name,
    row.is_email_verified,
    row.email_verification_token,
    email_verification_token_expires_at,
    row.password_reset_token,
    password_reset_token_expires_at,
    created_at,
    updated_at,
    deleted_at,
  ))
}

pub struct SqliteUserRepository {
  pool: SqlitePool,
}

impl SqliteUserRepository {
  pub fn new(pool: SqlitePool) -> Self {
    Self { pool }
  }
}

#[async_trait]
impl UserRepository for SqliteUserRepository {
  async fn create(&self, user: User) -> Result<User, AuthError> {
    let result = sqlx::query_as::<_, UserRow>(
      r#"
      INSERT INTO users (
          id, email, password_hash, full_name, is_email_verified,
          email_verification_token, email_verification_token_expires_at,
          password_reset_token, password_reset_token_expires_at,
          created_at, updated_at, deleted_at
      )
      VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
      RETURNING
          id, email, password_hash, full_name, is_email_verified,
          email_verification_token, email_verification_token_expires_at,
          password_reset_token, password_reset_token_expires_at,
          created_at, updated_at, deleted_at
      "#,
    )
    .bind(user.id.to_string())
    .bind(&user.email)
    .bind(&user.password_hash)
    .bind(&user.full_name)
    .bind(user.is_email_verified)
    .bind(&user.email_verification_token)
    .bind(
      user
        .email_verification_token_expires_at
        .map(|dt| dt.to_rfc3339()),
    )
    .bind(&user.password_reset_token)
    .bind(
      user
        .password_reset_token_expires_at
        .map(|dt| dt.to_rfc3339()),
    )
    .bind(user.created_at.to_rfc3339())
    .bind(user.updated_at.to_rfc3339())
    .bind(user.deleted_at.map(|dt| dt.to_rfc3339()))
    .fetch_one(&self.pool)
    .await?;

    parse_user_row(result)
  }

  async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, AuthError> {
    let result = sqlx::query_as::<_, UserRow>(
      r#"
      SELECT
          id, email, password_hash, full_name, is_email_verified,
          email_verification_token, email_verification_token_expires_at,
          password_reset_token, password_reset_token_expires_at,
          created_at, updated_at, deleted_at
      FROM users
      WHERE id = ?1 AND deleted_at IS NULL
      "#,
    )
    .bind(id.to_string())
    .fetch_optional(&self.pool)
    .await;

    match result {
      Ok(Some(row)) => Ok(Some(parse_user_row(row)?)),
      Ok(None) => Ok(None),
      Err(e) => Err(e.into()),
    }
  }

  async fn find_by_email(&self, email: &Email) -> Result<Option<User>, AuthError> {
    let result = sqlx::query_as::<_, UserRow>(
      r#"
      SELECT
          id, email, password_hash, full_name, is_email_verified,
          email_verification_token, email_verification_token_expires_at,
          password_reset_token, password_reset_token_expires_at,
          created_at, updated_at, deleted_at
      FROM users
      WHERE email = ?1 AND deleted_at IS NULL
      "#,
    )
    .bind(email.as_str())
    .fetch_optional(&self.pool)
    .await;

    match result {
      Ok(Some(row)) => Ok(Some(parse_user_row(row)?)),
      Ok(None) => Ok(None),
      Err(e) => Err(e.into()),
    }
  }

  async fn update(&self, user: User) -> Result<User, AuthError> {
    let result = sqlx::query_as::<_, UserRow>(
      r#"
      UPDATE users
      SET
          email = ?2,
          password_hash = ?3,
          full_name = ?4,
          is_email_verified = ?5,
          email_verification_token = ?6,
          email_verification_token_expires_at = ?7,
          password_reset_token = ?8,
          password_reset_token_expires_at = ?9,
          updated_at = ?10
      WHERE id = ?1 AND deleted_at IS NULL
      RETURNING
          id, email, password_hash, full_name, is_email_verified,
          email_verification_token, email_verification_token_expires_at,
          password_reset_token, password_reset_token_expires_at,
          created_at, updated_at, deleted_at
      "#,
    )
    .bind(user.id.to_string())
    .bind(&user.email)
    .bind(&user.password_hash)
    .bind(&user.full_name)
    .bind(user.is_email_verified)
    .bind(&user.email_verification_token)
    .bind(
      user
        .email_verification_token_expires_at
        .map(|dt| dt.to_rfc3339()),
    )
    .bind(&user.password_reset_token)
    .bind(
      user
        .password_reset_token_expires_at
        .map(|dt| dt.to_rfc3339()),
    )
    .bind(user.updated_at.to_rfc3339())
    .fetch_one(&self.pool)
    .await;

    match result {
      Ok(row) => parse_user_row(row),
      Err(sqlx::Error::RowNotFound) => Err(AuthError::Repository(RepositoryError::NotFound)),
      Err(sqlx::Error::Database(db_err)) => {
        if db_err.is_unique_violation() {
          Err(AuthError::EmailAlreadyExists)
        } else {
          Err(AuthError::Repository(RepositoryError::DatabaseError(
            db_err.to_string(),
          )))
        }
      }
      Err(e) => Err(e.into()),
    }
  }

  async fn soft_delete(&self, id: Uuid) -> Result<(), AuthError> {
    let now = Utc::now();

    let result = sqlx::query(
      r#"
      UPDATE users
      SET
          deleted_at = ?2,
          updated_at = ?2
      WHERE id = ?1 AND deleted_at IS NULL
      "#,
    )
    .bind(id.to_string())
    .bind(now.to_rfc3339())
    .execute(&self.pool)
    .await;

    match result {
      Ok(result) => {
        if result.rows_affected() == 0 {
          Err(AuthError::Repository(RepositoryError::NotFound))
        } else {
          Ok(())
        }
      }
      Err(e) => Err(e.into()),
    }
  }
}
