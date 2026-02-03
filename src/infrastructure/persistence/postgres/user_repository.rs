use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::auth::{
  entities::User,
  errors::{AuthError, RepositoryError},
  ports::UserRepository,
  value_objects::Email,
};

/// PostgreSQL implementation of the UserRepository trait
pub struct PostgresUserRepository {
  pool: PgPool,
}

impl PostgresUserRepository {
  /// Creates a new instance of PostgresUserRepository
  pub fn new(pool: PgPool) -> Self {
    Self { pool }
  }
}

/// Database row structure for users table
#[derive(Debug, sqlx::FromRow)]
struct UserRow {
  id: Uuid,
  email: String,
  password_hash: String,
  full_name: String,
  is_email_verified: bool,
  email_verification_token: Option<String>,
  email_verification_token_expires_at: Option<DateTime<Utc>>,
  password_reset_token: Option<String>,
  password_reset_token_expires_at: Option<DateTime<Utc>>,
  created_at: DateTime<Utc>,
  updated_at: DateTime<Utc>,
  deleted_at: Option<DateTime<Utc>>,
}

impl From<UserRow> for User {
  fn from(row: UserRow) -> Self {
    User::from_db(
      row.id,
      row.email,
      row.password_hash,
      row.full_name,
      row.is_email_verified,
      row.email_verification_token,
      row.email_verification_token_expires_at,
      row.password_reset_token,
      row.password_reset_token_expires_at,
      row.created_at,
      row.updated_at,
      row.deleted_at,
    )
  }
}

#[async_trait]
impl UserRepository for PostgresUserRepository {
  async fn create(&self, user: User) -> Result<User, AuthError> {
    let result = sqlx::query_as::<_, UserRow>(
      r#"
            INSERT INTO users (
                id,
                email,
                password_hash,
                full_name,
                is_email_verified,
                email_verification_token,
                email_verification_token_expires_at,
                password_reset_token,
                password_reset_token_expires_at,
                created_at,
                updated_at,
                deleted_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING
                id,
                email,
                password_hash,
                full_name,
                is_email_verified,
                email_verification_token,
                email_verification_token_expires_at,
                password_reset_token,
                password_reset_token_expires_at,
                created_at,
                updated_at,
                deleted_at
            "#,
    )
    .bind(user.id)
    .bind(&user.email)
    .bind(&user.password_hash)
    .bind(&user.full_name)
    .bind(user.is_email_verified)
    .bind(&user.email_verification_token)
    .bind(user.email_verification_token_expires_at)
    .bind(&user.password_reset_token)
    .bind(user.password_reset_token_expires_at)
    .bind(user.created_at)
    .bind(user.updated_at)
    .bind(user.deleted_at)
    .fetch_one(&self.pool)
    .await?;

    Ok(result.into())
  }

  async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, AuthError> {
    let result = sqlx::query_as::<_, UserRow>(
      r#"
            SELECT
                id,
                email,
                password_hash,
                full_name,
                is_email_verified,
                email_verification_token,
                email_verification_token_expires_at,
                password_reset_token,
                password_reset_token_expires_at,
                created_at,
                updated_at,
                deleted_at
            FROM users
            WHERE id = $1 AND deleted_at IS NULL
            "#,
    )
    .bind(id)
    .fetch_optional(&self.pool)
    .await;

    match result {
      Ok(Some(row)) => Ok(Some(row.into())),
      Ok(None) => Ok(None),
      Err(e) => Err(e.into()),
    }
  }

  async fn find_by_email(&self, email: &Email) -> Result<Option<User>, AuthError> {
    let result = sqlx::query_as::<_, UserRow>(
      r#"
            SELECT
                id,
                email,
                password_hash,
                full_name,
                is_email_verified,
                email_verification_token,
                email_verification_token_expires_at,
                password_reset_token,
                password_reset_token_expires_at,
                created_at,
                updated_at,
                deleted_at
            FROM users
            WHERE email = $1 AND deleted_at IS NULL
            "#,
    )
    .bind(email.as_str())
    .fetch_optional(&self.pool)
    .await;

    match result {
      Ok(Some(row)) => Ok(Some(row.into())),
      Ok(None) => Ok(None),
      Err(e) => Err(e.into()),
    }
  }

  async fn update(&self, user: User) -> Result<User, AuthError> {
    let result = sqlx::query_as::<_, UserRow>(
      r#"
            UPDATE users
            SET
                email = $2,
                password_hash = $3,
                full_name = $4,
                is_email_verified = $5,
                email_verification_token = $6,
                email_verification_token_expires_at = $7,
                password_reset_token = $8,
                password_reset_token_expires_at = $9,
                updated_at = $10
            WHERE id = $1 AND deleted_at IS NULL
            RETURNING
                id,
                email,
                password_hash,
                full_name,
                is_email_verified,
                email_verification_token,
                email_verification_token_expires_at,
                password_reset_token,
                password_reset_token_expires_at,
                created_at,
                updated_at,
                deleted_at
            "#,
    )
    .bind(user.id)
    .bind(&user.email)
    .bind(&user.password_hash)
    .bind(&user.full_name)
    .bind(user.is_email_verified)
    .bind(&user.email_verification_token)
    .bind(user.email_verification_token_expires_at)
    .bind(&user.password_reset_token)
    .bind(user.password_reset_token_expires_at)
    .bind(user.updated_at)
    .fetch_one(&self.pool)
    .await;

    match result {
      Ok(row) => Ok(row.into()),
      Err(sqlx::Error::RowNotFound) => Err(AuthError::Repository(RepositoryError::NotFound)),
      Err(sqlx::Error::Database(db_err)) => {
        // Check for unique constraint violation on email
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
    let result = sqlx::query(
      r#"
            UPDATE users
            SET
                deleted_at = NOW(),
                updated_at = NOW()
            WHERE id = $1 AND deleted_at IS NULL
            "#,
    )
    .bind(id)
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

#[cfg(test)]
mod tests {
  use super::*;
  use sqlx::postgres::PgPoolOptions;
  use testcontainers::ImageExt;
  use testcontainers_modules::postgres::Postgres;
  use testcontainers_modules::testcontainers::{ContainerAsync, runners::AsyncRunner};

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
    let database_url = format!("postgres://postgres:postgres@{}:{}/postgres", host, port);

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
  async fn test_create_user() {
    let (pool, _container) = setup_test_db().await;
    let repo = PostgresUserRepository::new(pool);

    let user = User::new(
      "test@example.com".to_string(),
      "hashed_password".to_string(),
      "Test User".to_string(),
    );

    let result = repo.create(user.clone()).await;
    assert!(result.is_ok());

    let created_user = result.unwrap();
    assert_eq!(created_user.email, user.email);
    assert_eq!(created_user.full_name, user.full_name);
  }

  #[tokio::test]
  async fn test_find_by_email() {
    let (pool, _container) = setup_test_db().await;
    let repo = PostgresUserRepository::new(pool);

    let user = User::new(
      "find@example.com".to_string(),
      "hashed_password".to_string(),
      "Find User".to_string(),
    );

    repo.create(user.clone()).await.unwrap();

    let email = Email::new("find@example.com".to_string()).unwrap();
    let result = repo.find_by_email(&email).await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_some());
  }

  #[tokio::test]
  async fn test_duplicate_email() {
    let (pool, _container) = setup_test_db().await;
    let repo = PostgresUserRepository::new(pool);

    let user1 = User::new(
      "duplicate@example.com".to_string(),
      "hashed_password".to_string(),
      "User One".to_string(),
    );

    let user2 = User::new(
      "duplicate@example.com".to_string(),
      "hashed_password2".to_string(),
      "User Two".to_string(),
    );

    repo.create(user1).await.unwrap();
    let result = repo.create(user2).await;

    assert!(result.is_err());
    match result.unwrap_err() {
      AuthError::Repository(RepositoryError::DuplicateKey(_)) => {}
      _ => panic!("Expected Repository(DuplicateKey) error"),
    }
  }

  #[tokio::test]
  async fn test_update_user() {
    let (pool, _container) = setup_test_db().await;
    let repo = PostgresUserRepository::new(pool);

    let user = User::new(
      "update@example.com".to_string(),
      "hashed_password".to_string(),
      "Update User".to_string(),
    );

    let created_user = repo.create(user).await.unwrap();
    let mut updated_user = created_user.clone();
    updated_user.update_full_name("Updated Name".to_string());

    let result = repo.update(updated_user).await;
    assert!(result.is_ok());

    let final_user = result.unwrap();
    assert_eq!(final_user.full_name, "Updated Name");
  }

  #[tokio::test]
  async fn test_soft_delete() {
    let (pool, _container) = setup_test_db().await;
    let repo = PostgresUserRepository::new(pool.clone());

    let user = User::new(
      "delete@example.com".to_string(),
      "hashed_password".to_string(),
      "Delete User".to_string(),
    );

    let created_user = repo.create(user).await.unwrap();
    let result = repo.soft_delete(created_user.id).await;

    assert!(result.is_ok());

    // Verify the user is no longer found by normal queries (filtered by deleted_at IS NULL)
    let found_user = repo.find_by_id(created_user.id).await.unwrap();
    assert!(found_user.is_none());

    // Verify the user still physically exists in the database with deleted_at set
    let raw_user = sqlx::query_as::<_, UserRow>(
      "SELECT id, email, password_hash, full_name, is_email_verified,
       email_verification_token, email_verification_token_expires_at,
       password_reset_token, password_reset_token_expires_at,
       created_at, updated_at, deleted_at
       FROM users WHERE id = $1",
    )
    .bind(created_user.id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(raw_user.deleted_at.is_some());
  }
}
