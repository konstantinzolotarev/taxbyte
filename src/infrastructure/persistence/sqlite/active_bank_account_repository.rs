use async_trait::async_trait;
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::domain::company::{ActiveBankAccount, ActiveBankAccountRepository, CompanyError};

#[derive(Debug, FromRow)]
#[allow(dead_code)]
struct ActiveBankAccountRow {
  company_id: String,
  bank_account_id: String,
  set_at: String,
}

pub struct SqliteActiveBankAccountRepository {
  pool: SqlitePool,
}

impl SqliteActiveBankAccountRepository {
  pub fn new(pool: SqlitePool) -> Self {
    Self { pool }
  }
}

#[async_trait]
impl ActiveBankAccountRepository for SqliteActiveBankAccountRepository {
  async fn set_active(&self, active: ActiveBankAccount) -> Result<(), CompanyError> {
    sqlx::query(
      r#"
      INSERT INTO active_bank_accounts (company_id, bank_account_id, set_at)
      VALUES (?1, ?2, ?3)
      ON CONFLICT (company_id)
      DO UPDATE SET bank_account_id = ?2, set_at = ?3
      "#,
    )
    .bind(active.company_id.to_string())
    .bind(active.bank_account_id.to_string())
    .bind(active.set_at.to_rfc3339())
    .execute(&self.pool)
    .await?;

    Ok(())
  }

  async fn get_active(&self, company_id: Uuid) -> Result<Option<Uuid>, CompanyError> {
    let row = sqlx::query_as::<_, ActiveBankAccountRow>(
      r#"
      SELECT company_id, bank_account_id, set_at
      FROM active_bank_accounts
      WHERE company_id = ?1
      "#,
    )
    .bind(company_id.to_string())
    .fetch_optional(&self.pool)
    .await?;

    match row {
      Some(r) => {
        let bank_account_id = Uuid::parse_str(&r.bank_account_id).map_err(|e| {
          CompanyError::Repository(crate::domain::auth::errors::RepositoryError::QueryFailed(
            e.to_string(),
          ))
        })?;
        Ok(Some(bank_account_id))
      }
      None => Ok(None),
    }
  }

  async fn clear_active(&self, company_id: Uuid) -> Result<(), CompanyError> {
    sqlx::query(
      r#"
      DELETE FROM active_bank_accounts
      WHERE company_id = ?1
      "#,
    )
    .bind(company_id.to_string())
    .execute(&self.pool)
    .await?;

    Ok(())
  }
}
