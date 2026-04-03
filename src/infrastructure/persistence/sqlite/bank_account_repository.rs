use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::domain::auth::errors::RepositoryError;
use crate::domain::company::{
  BankAccount, BankAccountName, BankAccountRepository, BankDetails, CompanyError, Iban,
};

#[derive(Debug, FromRow)]
struct BankAccountRow {
  id: String,
  company_id: String,
  name: String,
  iban: String,
  bank_details: Option<String>,
  created_at: String,
  updated_at: String,
  archived_at: Option<String>,
}

fn parse_bank_account_row(row: BankAccountRow) -> Result<BankAccount, CompanyError> {
  let id = Uuid::parse_str(&row.id)
    .map_err(|e| CompanyError::Repository(RepositoryError::QueryFailed(e.to_string())))?;
  let company_id = Uuid::parse_str(&row.company_id)
    .map_err(|e| CompanyError::Repository(RepositoryError::QueryFailed(e.to_string())))?;
  let name = BankAccountName::new(row.name).map_err(CompanyError::Validation)?;
  let iban = Iban::new(row.iban).map_err(CompanyError::Validation)?;
  let bank_details = row
    .bank_details
    .map(BankDetails::new)
    .transpose()
    .map_err(CompanyError::Validation)?;
  let created_at = DateTime::parse_from_rfc3339(&row.created_at)
    .map(|dt| dt.with_timezone(&Utc))
    .map_err(|e| CompanyError::Repository(RepositoryError::QueryFailed(e.to_string())))?;
  let updated_at = DateTime::parse_from_rfc3339(&row.updated_at)
    .map(|dt| dt.with_timezone(&Utc))
    .map_err(|e| CompanyError::Repository(RepositoryError::QueryFailed(e.to_string())))?;
  let archived_at = row
    .archived_at
    .map(|s| {
      DateTime::parse_from_rfc3339(&s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| CompanyError::Repository(RepositoryError::QueryFailed(e.to_string())))
    })
    .transpose()?;

  Ok(BankAccount::from_db(
    id,
    company_id,
    name,
    iban,
    bank_details,
    created_at,
    updated_at,
    archived_at,
  ))
}

pub struct SqliteBankAccountRepository {
  pool: SqlitePool,
}

impl SqliteBankAccountRepository {
  pub fn new(pool: SqlitePool) -> Self {
    Self { pool }
  }
}

#[async_trait]
impl BankAccountRepository for SqliteBankAccountRepository {
  async fn create(&self, account: BankAccount) -> Result<BankAccount, CompanyError> {
    let bank_details = account
      .bank_details
      .as_ref()
      .map(|d| d.as_str().to_string());

    let row = sqlx::query_as::<_, BankAccountRow>(
      r#"
      INSERT INTO bank_accounts (id, company_id, name, iban, bank_details, created_at, updated_at, archived_at)
      VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
      RETURNING id, company_id, name, iban, bank_details, created_at, updated_at, archived_at
      "#,
    )
    .bind(account.id.to_string())
    .bind(account.company_id.to_string())
    .bind(account.name.as_str())
    .bind(account.iban.as_str())
    .bind(bank_details.as_deref())
    .bind(account.created_at.to_rfc3339())
    .bind(account.updated_at.to_rfc3339())
    .bind(account.archived_at.map(|dt| dt.to_rfc3339()))
    .fetch_one(&self.pool)
    .await?;

    parse_bank_account_row(row)
  }

  async fn find_by_id(&self, id: Uuid) -> Result<Option<BankAccount>, CompanyError> {
    let row = sqlx::query_as::<_, BankAccountRow>(
      r#"
      SELECT id, company_id, name, iban, bank_details, created_at, updated_at, archived_at
      FROM bank_accounts
      WHERE id = ?1
      "#,
    )
    .bind(id.to_string())
    .fetch_optional(&self.pool)
    .await?;

    row.map(parse_bank_account_row).transpose()
  }

  async fn find_by_company_id(
    &self,
    company_id: Uuid,
    include_archived: bool,
  ) -> Result<Vec<BankAccount>, CompanyError> {
    let rows = if include_archived {
      sqlx::query_as::<_, BankAccountRow>(
        r#"
        SELECT id, company_id, name, iban, bank_details, created_at, updated_at, archived_at
        FROM bank_accounts
        WHERE company_id = ?1
        ORDER BY created_at DESC
        "#,
      )
      .bind(company_id.to_string())
      .fetch_all(&self.pool)
      .await?
    } else {
      sqlx::query_as::<_, BankAccountRow>(
        r#"
        SELECT id, company_id, name, iban, bank_details, created_at, updated_at, archived_at
        FROM bank_accounts
        WHERE company_id = ?1 AND archived_at IS NULL
        ORDER BY created_at DESC
        "#,
      )
      .bind(company_id.to_string())
      .fetch_all(&self.pool)
      .await?
    };

    rows.into_iter().map(parse_bank_account_row).collect()
  }

  async fn find_by_iban(
    &self,
    company_id: Uuid,
    iban: &str,
  ) -> Result<Option<BankAccount>, CompanyError> {
    let row = sqlx::query_as::<_, BankAccountRow>(
      r#"
      SELECT id, company_id, name, iban, bank_details, created_at, updated_at, archived_at
      FROM bank_accounts
      WHERE company_id = ?1 AND iban = ?2 AND archived_at IS NULL
      "#,
    )
    .bind(company_id.to_string())
    .bind(iban)
    .fetch_optional(&self.pool)
    .await?;

    row.map(parse_bank_account_row).transpose()
  }

  async fn update(&self, account: BankAccount) -> Result<BankAccount, CompanyError> {
    let bank_details = account
      .bank_details
      .as_ref()
      .map(|d| d.as_str().to_string());

    let row = sqlx::query_as::<_, BankAccountRow>(
      r#"
      UPDATE bank_accounts
      SET name = ?2, iban = ?3, bank_details = ?4, updated_at = ?5, archived_at = ?6
      WHERE id = ?1
      RETURNING id, company_id, name, iban, bank_details, created_at, updated_at, archived_at
      "#,
    )
    .bind(account.id.to_string())
    .bind(account.name.as_str())
    .bind(account.iban.as_str())
    .bind(bank_details.as_deref())
    .bind(account.updated_at.to_rfc3339())
    .bind(account.archived_at.map(|dt| dt.to_rfc3339()))
    .fetch_one(&self.pool)
    .await?;

    parse_bank_account_row(row)
  }

  async fn archive(&self, id: Uuid) -> Result<(), CompanyError> {
    let now = Utc::now();

    sqlx::query(
      r#"
      UPDATE bank_accounts
      SET archived_at = ?2
      WHERE id = ?1
      "#,
    )
    .bind(id.to_string())
    .bind(now.to_rfc3339())
    .execute(&self.pool)
    .await?;

    Ok(())
  }
}
