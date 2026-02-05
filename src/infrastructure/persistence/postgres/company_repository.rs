use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::domain::auth::errors::RepositoryError;
use crate::domain::auth::value_objects::Email;
use crate::domain::company::{
  Company, CompanyAddress, CompanyError, CompanyRepository, PhoneNumber, RegistryCode, VatNumber,
};

#[derive(Debug, FromRow)]
struct CompanyRow {
  id: Uuid,
  name: String,
  email: Option<String>,
  phone: Option<String>,
  address: Option<String>, // JSON string
  tax_id: Option<String>,  // Maps to registry_code
  vat_number: Option<String>,
  invoice_folder_path: Option<String>,
  google_drive_folder_id: Option<String>,
  storage_provider: Option<String>,
  storage_config: Option<String>,
  created_at: DateTime<Utc>,
  updated_at: DateTime<Utc>,
}

impl TryFrom<CompanyRow> for Company {
  type Error = CompanyError;

  fn try_from(row: CompanyRow) -> Result<Self, Self::Error> {
    // Parse email
    let email = row
      .email
      .map(Email::new)
      .transpose()
      .map_err(CompanyError::from)?;

    // Parse phone
    let phone = row
      .phone
      .map(PhoneNumber::new)
      .transpose()
      .map_err(CompanyError::Validation)?;

    // Parse address from JSON
    let address = row
      .address
      .map(|a| {
        serde_json::from_str::<CompanyAddress>(&a)
          .map_err(|e| CompanyError::Repository(RepositoryError::QueryFailed(e.to_string())))
      })
      .transpose()?;

    // Parse registry_code (tax_id)
    let registry_code = row
      .tax_id
      .map(RegistryCode::new)
      .transpose()
      .map_err(CompanyError::Validation)?;

    // Parse vat_number
    let vat_number = row
      .vat_number
      .map(VatNumber::new)
      .transpose()
      .map_err(CompanyError::Validation)?;

    Ok(Company {
      id: row.id,
      name: row.name,
      email,
      phone,
      address,
      registry_code,
      vat_number,
      invoice_folder_path: row.invoice_folder_path,
      google_drive_folder_id: row.google_drive_folder_id,
      storage_provider: row.storage_provider,
      storage_config: row.storage_config,
      created_at: row.created_at,
      updated_at: row.updated_at,
    })
  }
}

pub struct PostgresCompanyRepository {
  pool: PgPool,
}

impl PostgresCompanyRepository {
  pub fn new(pool: PgPool) -> Self {
    Self { pool }
  }
}

#[async_trait]
impl CompanyRepository for PostgresCompanyRepository {
  async fn create(&self, company: Company) -> Result<Company, CompanyError> {
    // Serialize address to JSON if present
    let address_json = company
      .address
      .as_ref()
      .map(|a| a.as_json())
      .transpose()
      .map_err(|e| {
        CompanyError::Repository(crate::domain::auth::errors::RepositoryError::QueryFailed(
          e.to_string(),
        ))
      })?;

    let row = sqlx::query_as::<_, CompanyRow>(
            r#"
            INSERT INTO companies (id, name, email, phone, address, tax_id, vat_number, invoice_folder_path, google_drive_folder_id, storage_provider, storage_config, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            RETURNING id, name, email, phone, address, tax_id, vat_number, invoice_folder_path, google_drive_folder_id, storage_provider, storage_config, created_at, updated_at
            "#,
        )
        .bind(company.id)
        .bind(&company.name)
        .bind(company.email.as_ref().map(|e| e.as_str()))
        .bind(company.phone.as_ref().map(|p| p.as_str()))
        .bind(address_json.as_deref())
        .bind(company.registry_code.as_ref().map(|r| r.as_str()))
        .bind(company.vat_number.as_ref().map(|v| v.as_str()))
        .bind(company.invoice_folder_path.as_deref())
        .bind(company.google_drive_folder_id.as_deref())
        .bind(company.storage_provider.as_deref())
        .bind(company.storage_config.as_deref())
        .bind(company.created_at)
        .bind(company.updated_at)
        .fetch_one(&self.pool)
        .await?;

    row.try_into()
  }

  async fn find_by_id(&self, id: Uuid) -> Result<Option<Company>, CompanyError> {
    let row = sqlx::query_as::<_, CompanyRow>(
      r#"
            SELECT id, name, email, phone, address, tax_id, vat_number, invoice_folder_path, google_drive_folder_id, storage_provider, storage_config, created_at, updated_at
            FROM companies
            WHERE id = $1
            "#,
    )
    .bind(id)
    .fetch_optional(&self.pool)
    .await?;

    row.map(|r| r.try_into()).transpose()
  }

  async fn update(&self, company: Company) -> Result<Company, CompanyError> {
    // Serialize address to JSON if present
    let address_json = company
      .address
      .as_ref()
      .map(|a| a.as_json())
      .transpose()
      .map_err(|e| {
        CompanyError::Repository(crate::domain::auth::errors::RepositoryError::QueryFailed(
          e.to_string(),
        ))
      })?;

    let row = sqlx::query_as::<_, CompanyRow>(
            r#"
            UPDATE companies
            SET name = $2, email = $3, phone = $4, address = $5, tax_id = $6, vat_number = $7, invoice_folder_path = $8, google_drive_folder_id = $9, storage_provider = $10, storage_config = $11, updated_at = $12
            WHERE id = $1
            RETURNING id, name, email, phone, address, tax_id, vat_number, invoice_folder_path, google_drive_folder_id, storage_provider, storage_config, created_at, updated_at
            "#,
        )
        .bind(company.id)
        .bind(&company.name)
        .bind(company.email.as_ref().map(|e| e.as_str()))
        .bind(company.phone.as_ref().map(|p| p.as_str()))
        .bind(address_json.as_deref())
        .bind(company.registry_code.as_ref().map(|r| r.as_str()))
        .bind(company.vat_number.as_ref().map(|v| v.as_str()))
        .bind(company.invoice_folder_path.as_deref())
        .bind(company.google_drive_folder_id.as_deref())
        .bind(company.storage_provider.as_deref())
        .bind(company.storage_config.as_deref())
        .bind(company.updated_at)
        .fetch_one(&self.pool)
        .await?;

    row.try_into()
  }

  async fn delete(&self, id: Uuid) -> Result<(), CompanyError> {
    sqlx::query("DELETE FROM companies WHERE id = $1")
      .bind(id)
      .execute(&self.pool)
      .await?;

    Ok(())
  }
}
