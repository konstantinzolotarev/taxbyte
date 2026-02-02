use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::domain::company::{ActiveBankAccount, ActiveBankAccountRepository, CompanyError};

#[derive(Debug, FromRow)]
struct ActiveBankAccountRow {
  company_id: Uuid,
  bank_account_id: Uuid,
  set_at: DateTime<Utc>,
}

impl From<ActiveBankAccountRow> for ActiveBankAccount {
  fn from(row: ActiveBankAccountRow) -> Self {
    ActiveBankAccount::from_db(row.company_id, row.bank_account_id, row.set_at)
  }
}

pub struct PostgresActiveBankAccountRepository {
  pool: PgPool,
}

impl PostgresActiveBankAccountRepository {
  pub fn new(pool: PgPool) -> Self {
    Self { pool }
  }
}

#[async_trait]
impl ActiveBankAccountRepository for PostgresActiveBankAccountRepository {
  async fn set_active(&self, active: ActiveBankAccount) -> Result<(), CompanyError> {
    sqlx::query(
      r#"
      INSERT INTO active_bank_accounts (company_id, bank_account_id, set_at)
      VALUES ($1, $2, $3)
      ON CONFLICT (company_id)
      DO UPDATE SET bank_account_id = $2, set_at = $3
      "#,
    )
    .bind(active.company_id)
    .bind(active.bank_account_id)
    .bind(active.set_at)
    .execute(&self.pool)
    .await?;

    Ok(())
  }

  async fn get_active(&self, company_id: Uuid) -> Result<Option<Uuid>, CompanyError> {
    let row = sqlx::query_as::<_, ActiveBankAccountRow>(
      r#"
      SELECT company_id, bank_account_id, set_at
      FROM active_bank_accounts
      WHERE company_id = $1
      "#,
    )
    .bind(company_id)
    .fetch_optional(&self.pool)
    .await?;

    Ok(row.map(|r| r.bank_account_id))
  }

  async fn clear_active(&self, company_id: Uuid) -> Result<(), CompanyError> {
    sqlx::query(
      r#"
      DELETE FROM active_bank_accounts
      WHERE company_id = $1
      "#,
    )
    .bind(company_id)
    .execute(&self.pool)
    .await?;

    Ok(())
  }
}
