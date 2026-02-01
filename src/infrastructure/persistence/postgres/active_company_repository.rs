use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::domain::company::{ActiveCompany, ActiveCompanyRepository, CompanyError};

#[derive(Debug, FromRow)]
struct ActiveCompanyRow {
  user_id: Uuid,
  company_id: Uuid,
  set_at: DateTime<Utc>,
}

impl From<ActiveCompanyRow> for ActiveCompany {
  fn from(row: ActiveCompanyRow) -> Self {
    ActiveCompany::from_db(row.user_id, row.company_id, row.set_at)
  }
}

pub struct PostgresActiveCompanyRepository {
  pool: PgPool,
}

impl PostgresActiveCompanyRepository {
  pub fn new(pool: PgPool) -> Self {
    Self { pool }
  }
}

#[async_trait]
impl ActiveCompanyRepository for PostgresActiveCompanyRepository {
  async fn set_active(&self, active: ActiveCompany) -> Result<(), CompanyError> {
    sqlx::query(
      r#"
            INSERT INTO active_companies (user_id, company_id, set_at)
            VALUES ($1, $2, $3)
            ON CONFLICT (user_id)
            DO UPDATE SET company_id = $2, set_at = $3
            "#,
    )
    .bind(active.user_id)
    .bind(active.company_id)
    .bind(active.set_at)
    .execute(&self.pool)
    .await?;

    Ok(())
  }

  async fn get_active(&self, user_id: Uuid) -> Result<Option<Uuid>, CompanyError> {
    let row = sqlx::query_as::<_, ActiveCompanyRow>(
      r#"
            SELECT user_id, company_id, set_at
            FROM active_companies
            WHERE user_id = $1
            "#,
    )
    .bind(user_id)
    .fetch_optional(&self.pool)
    .await?;

    Ok(row.map(|r| r.company_id))
  }

  async fn clear_active(&self, user_id: Uuid) -> Result<(), CompanyError> {
    sqlx::query("DELETE FROM active_companies WHERE user_id = $1")
      .bind(user_id)
      .execute(&self.pool)
      .await?;

    Ok(())
  }
}
