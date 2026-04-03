use async_trait::async_trait;
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::domain::company::{ActiveCompany, ActiveCompanyRepository, CompanyError};

#[derive(Debug, FromRow)]
#[allow(dead_code)]
struct ActiveCompanyRow {
  user_id: String,
  company_id: String,
  set_at: String,
}

pub struct SqliteActiveCompanyRepository {
  pool: SqlitePool,
}

impl SqliteActiveCompanyRepository {
  pub fn new(pool: SqlitePool) -> Self {
    Self { pool }
  }
}

#[async_trait]
impl ActiveCompanyRepository for SqliteActiveCompanyRepository {
  async fn set_active(&self, active: ActiveCompany) -> Result<(), CompanyError> {
    sqlx::query(
      r#"
      INSERT INTO active_companies (user_id, company_id, set_at)
      VALUES (?1, ?2, ?3)
      ON CONFLICT (user_id)
      DO UPDATE SET company_id = ?2, set_at = ?3
      "#,
    )
    .bind(active.user_id.to_string())
    .bind(active.company_id.to_string())
    .bind(active.set_at.to_rfc3339())
    .execute(&self.pool)
    .await?;

    Ok(())
  }

  async fn get_active(&self, user_id: Uuid) -> Result<Option<Uuid>, CompanyError> {
    let row = sqlx::query_as::<_, ActiveCompanyRow>(
      r#"
      SELECT user_id, company_id, set_at
      FROM active_companies
      WHERE user_id = ?1
      "#,
    )
    .bind(user_id.to_string())
    .fetch_optional(&self.pool)
    .await?;

    match row {
      Some(r) => {
        let company_id = Uuid::parse_str(&r.company_id).map_err(|e| {
          CompanyError::Repository(crate::domain::auth::errors::RepositoryError::QueryFailed(
            e.to_string(),
          ))
        })?;
        Ok(Some(company_id))
      }
      None => Ok(None),
    }
  }

  async fn clear_active(&self, user_id: Uuid) -> Result<(), CompanyError> {
    sqlx::query("DELETE FROM active_companies WHERE user_id = ?1")
      .bind(user_id.to_string())
      .execute(&self.pool)
      .await?;

    Ok(())
  }
}
