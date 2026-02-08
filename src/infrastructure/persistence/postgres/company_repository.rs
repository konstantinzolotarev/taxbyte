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
  oauth_access_token: Option<String>,
  oauth_refresh_token: Option<String>,
  oauth_token_expires_at: Option<DateTime<Utc>>,
  oauth_connected_by: Option<Uuid>,
  oauth_connected_at: Option<DateTime<Utc>>,
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
      oauth_access_token: row.oauth_access_token,
      oauth_refresh_token: row.oauth_refresh_token,
      oauth_token_expires_at: row.oauth_token_expires_at,
      oauth_connected_by: row.oauth_connected_by,
      oauth_connected_at: row.oauth_connected_at,
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
            INSERT INTO companies (id, name, email, phone, address, tax_id, vat_number, invoice_folder_path, google_drive_folder_id, storage_provider, storage_config, oauth_access_token, oauth_refresh_token, oauth_token_expires_at, oauth_connected_by, oauth_connected_at, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
            RETURNING id, name, email, phone, address, tax_id, vat_number, invoice_folder_path, google_drive_folder_id, storage_provider, storage_config, oauth_access_token, oauth_refresh_token, oauth_token_expires_at, oauth_connected_by, oauth_connected_at, created_at, updated_at
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
        .bind(company.oauth_access_token.as_deref())
        .bind(company.oauth_refresh_token.as_deref())
        .bind(company.oauth_token_expires_at)
        .bind(company.oauth_connected_by)
        .bind(company.oauth_connected_at)
        .bind(company.created_at)
        .bind(company.updated_at)
        .fetch_one(&self.pool)
        .await?;

    row.try_into()
  }

  async fn find_by_id(&self, id: Uuid) -> Result<Option<Company>, CompanyError> {
    let row = sqlx::query_as::<_, CompanyRow>(
      r#"
            SELECT id, name, email, phone, address, tax_id, vat_number, invoice_folder_path, google_drive_folder_id, storage_provider, storage_config, oauth_access_token, oauth_refresh_token, oauth_token_expires_at, oauth_connected_by, oauth_connected_at, created_at, updated_at
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
            SET name = $2, email = $3, phone = $4, address = $5, tax_id = $6, vat_number = $7, invoice_folder_path = $8, google_drive_folder_id = $9, storage_provider = $10, storage_config = $11, oauth_access_token = $12, oauth_refresh_token = $13, oauth_token_expires_at = $14, oauth_connected_by = $15, oauth_connected_at = $16, updated_at = $17
            WHERE id = $1
            RETURNING id, name, email, phone, address, tax_id, vat_number, invoice_folder_path, google_drive_folder_id, storage_provider, storage_config, oauth_access_token, oauth_refresh_token, oauth_token_expires_at, oauth_connected_by, oauth_connected_at, created_at, updated_at
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
        .bind(company.oauth_access_token.as_deref())
        .bind(company.oauth_refresh_token.as_deref())
        .bind(company.oauth_token_expires_at)
        .bind(company.oauth_connected_by)
        .bind(company.oauth_connected_at)
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

  async fn update_oauth_tokens(
    &self,
    company_id: &Uuid,
    encrypted_access_token: String,
    encrypted_refresh_token: String,
    expires_at: DateTime<Utc>,
    connected_by: Uuid,
  ) -> Result<(), CompanyError> {
    let now = Utc::now();

    sqlx::query(
      r#"
      UPDATE companies
      SET oauth_access_token = $2,
          oauth_refresh_token = $3,
          oauth_token_expires_at = $4,
          oauth_connected_by = $5,
          oauth_connected_at = COALESCE(oauth_connected_at, $6),
          updated_at = $6
      WHERE id = $1
      "#,
    )
    .bind(company_id)
    .bind(encrypted_access_token)
    .bind(encrypted_refresh_token)
    .bind(expires_at)
    .bind(connected_by)
    .bind(now)
    .execute(&self.pool)
    .await?;

    Ok(())
  }

  async fn clear_oauth_tokens(&self, company_id: &Uuid) -> Result<(), CompanyError> {
    let now = Utc::now();

    sqlx::query(
      r#"
      UPDATE companies
      SET oauth_access_token = NULL,
          oauth_refresh_token = NULL,
          oauth_token_expires_at = NULL,
          oauth_connected_by = NULL,
          oauth_connected_at = NULL,
          updated_at = $2
      WHERE id = $1
      "#,
    )
    .bind(company_id)
    .bind(now)
    .execute(&self.pool)
    .await?;

    Ok(())
  }
}
