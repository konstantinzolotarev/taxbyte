use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::domain::company::{
  BankAccount, BankAccountName, BankAccountRepository, BankDetails, CompanyError, Iban,
};

#[derive(Debug, FromRow)]
struct BankAccountRow {
  id: Uuid,
  company_id: Uuid,
  name: String,
  iban: String,
  bank_details: Option<String>,
  created_at: DateTime<Utc>,
  updated_at: DateTime<Utc>,
  archived_at: Option<DateTime<Utc>>,
}

impl TryFrom<BankAccountRow> for BankAccount {
  type Error = CompanyError;

  fn try_from(row: BankAccountRow) -> Result<Self, Self::Error> {
    let name = BankAccountName::new(row.name).map_err(CompanyError::Validation)?;
    let iban = Iban::new(row.iban).map_err(CompanyError::Validation)?;
    let bank_details = row
      .bank_details
      .map(BankDetails::new)
      .transpose()
      .map_err(CompanyError::Validation)?;

    Ok(BankAccount::from_db(
      row.id,
      row.company_id,
      name,
      iban,
      bank_details,
      row.created_at,
      row.updated_at,
      row.archived_at,
    ))
  }
}

pub struct PostgresBankAccountRepository {
  pool: PgPool,
}

impl PostgresBankAccountRepository {
  pub fn new(pool: PgPool) -> Self {
    Self { pool }
  }
}

#[async_trait]
impl BankAccountRepository for PostgresBankAccountRepository {
  async fn create(&self, account: BankAccount) -> Result<BankAccount, CompanyError> {
    let bank_details = account.bank_details.as_ref().map(|d| d.as_str());

    sqlx::query_as::<_, BankAccountRow>(
      r#"
      INSERT INTO bank_accounts (id, company_id, name, iban, bank_details, created_at, updated_at, archived_at)
      VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
      RETURNING id, company_id, name, iban, bank_details, created_at, updated_at, archived_at
      "#,
    )
    .bind(account.id)
    .bind(account.company_id)
    .bind(account.name.as_str())
    .bind(account.iban.as_str())
    .bind(bank_details)
    .bind(account.created_at)
    .bind(account.updated_at)
    .bind(account.archived_at)
    .fetch_one(&self.pool)
    .await?
    .try_into()
  }

  async fn find_by_id(&self, id: Uuid) -> Result<Option<BankAccount>, CompanyError> {
    let row = sqlx::query_as::<_, BankAccountRow>(
      r#"
      SELECT id, company_id, name, iban, bank_details, created_at, updated_at, archived_at
      FROM bank_accounts
      WHERE id = $1
      "#,
    )
    .bind(id)
    .fetch_optional(&self.pool)
    .await?;

    row.map(|r| r.try_into()).transpose()
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
        WHERE company_id = $1
        ORDER BY created_at DESC
        "#,
      )
      .bind(company_id)
      .fetch_all(&self.pool)
      .await?
    } else {
      sqlx::query_as::<_, BankAccountRow>(
        r#"
        SELECT id, company_id, name, iban, bank_details, created_at, updated_at, archived_at
        FROM bank_accounts
        WHERE company_id = $1 AND archived_at IS NULL
        ORDER BY created_at DESC
        "#,
      )
      .bind(company_id)
      .fetch_all(&self.pool)
      .await?
    };

    rows.into_iter().map(|r| r.try_into()).collect()
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
      WHERE company_id = $1 AND iban = $2 AND archived_at IS NULL
      "#,
    )
    .bind(company_id)
    .bind(iban)
    .fetch_optional(&self.pool)
    .await?;

    row.map(|r| r.try_into()).transpose()
  }

  async fn update(&self, account: BankAccount) -> Result<BankAccount, CompanyError> {
    let bank_details = account.bank_details.as_ref().map(|d| d.as_str());

    sqlx::query_as::<_, BankAccountRow>(
      r#"
      UPDATE bank_accounts
      SET name = $2, iban = $3, bank_details = $4, updated_at = $5, archived_at = $6
      WHERE id = $1
      RETURNING id, company_id, name, iban, bank_details, created_at, updated_at, archived_at
      "#,
    )
    .bind(account.id)
    .bind(account.name.as_str())
    .bind(account.iban.as_str())
    .bind(bank_details)
    .bind(account.updated_at)
    .bind(account.archived_at)
    .fetch_one(&self.pool)
    .await?
    .try_into()
  }

  async fn archive(&self, id: Uuid) -> Result<(), CompanyError> {
    let now = Utc::now();

    sqlx::query(
      r#"
      UPDATE bank_accounts
      SET archived_at = $2
      WHERE id = $1
      "#,
    )
    .bind(id)
    .bind(now)
    .execute(&self.pool)
    .await?;

    Ok(())
  }
}
