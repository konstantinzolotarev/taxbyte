use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::domain::auth::errors::RepositoryError;
use crate::domain::auth::value_objects::Email;
use crate::domain::company::{
  Company, CompanyAddress, CompanyError, CompanyRepository, PhoneNumber, RegistryCode, VatNumber,
};

#[derive(Debug, FromRow)]
struct CompanyRow {
  id: String,
  name: String,
  email: Option<String>,
  phone: Option<String>,
  address: Option<String>,
  tax_id: Option<String>,
  vat_number: Option<String>,
  google_drive_folder_id: Option<String>,
  storage_provider: Option<String>,
  storage_config: Option<String>,
  oauth_access_token: Option<String>,
  oauth_refresh_token: Option<String>,
  oauth_token_expires_at: Option<String>,
  oauth_connected_by: Option<String>,
  oauth_connected_at: Option<String>,
  created_at: String,
  updated_at: String,
}

impl TryFrom<CompanyRow> for Company {
  type Error = CompanyError;

  fn try_from(row: CompanyRow) -> Result<Self, Self::Error> {
    let id = Uuid::parse_str(&row.id)
      .map_err(|e| CompanyError::Repository(RepositoryError::QueryFailed(e.to_string())))?;

    let email = row
      .email
      .map(Email::new)
      .transpose()
      .map_err(CompanyError::from)?;

    let phone = row
      .phone
      .map(PhoneNumber::new)
      .transpose()
      .map_err(CompanyError::Validation)?;

    let address = row
      .address
      .map(|a| {
        serde_json::from_str::<CompanyAddress>(&a)
          .map_err(|e| CompanyError::Repository(RepositoryError::QueryFailed(e.to_string())))
      })
      .transpose()?;

    let registry_code = row
      .tax_id
      .map(RegistryCode::new)
      .transpose()
      .map_err(CompanyError::Validation)?;

    let vat_number = row
      .vat_number
      .map(VatNumber::new)
      .transpose()
      .map_err(CompanyError::Validation)?;

    let created_at = DateTime::parse_from_rfc3339(&row.created_at)
      .map(|dt| dt.with_timezone(&Utc))
      .map_err(|e| CompanyError::Repository(RepositoryError::QueryFailed(e.to_string())))?;
    let updated_at = DateTime::parse_from_rfc3339(&row.updated_at)
      .map(|dt| dt.with_timezone(&Utc))
      .map_err(|e| CompanyError::Repository(RepositoryError::QueryFailed(e.to_string())))?;

    let oauth_token_expires_at = row
      .oauth_token_expires_at
      .map(|s| {
        DateTime::parse_from_rfc3339(&s)
          .map(|dt| dt.with_timezone(&Utc))
          .map_err(|e| CompanyError::Repository(RepositoryError::QueryFailed(e.to_string())))
      })
      .transpose()?;

    let oauth_connected_by = row
      .oauth_connected_by
      .map(|s| {
        Uuid::parse_str(&s)
          .map_err(|e| CompanyError::Repository(RepositoryError::QueryFailed(e.to_string())))
      })
      .transpose()?;

    let oauth_connected_at = row
      .oauth_connected_at
      .map(|s| {
        DateTime::parse_from_rfc3339(&s)
          .map(|dt| dt.with_timezone(&Utc))
          .map_err(|e| CompanyError::Repository(RepositoryError::QueryFailed(e.to_string())))
      })
      .transpose()?;

    Ok(Company {
      id,
      name: row.name,
      email,
      phone,
      address,
      registry_code,
      vat_number,
      google_drive_folder_id: row.google_drive_folder_id,
      storage_provider: row.storage_provider,
      storage_config: row.storage_config,
      oauth_access_token: row.oauth_access_token,
      oauth_refresh_token: row.oauth_refresh_token,
      oauth_token_expires_at,
      oauth_connected_by,
      oauth_connected_at,
      created_at,
      updated_at,
    })
  }
}

pub struct SqliteCompanyRepository {
  pool: SqlitePool,
}

impl SqliteCompanyRepository {
  pub fn new(pool: SqlitePool) -> Self {
    Self { pool }
  }
}

#[async_trait]
impl CompanyRepository for SqliteCompanyRepository {
  async fn create(&self, company: Company) -> Result<Company, CompanyError> {
    let address_json = company
      .address
      .as_ref()
      .map(|a| a.as_json())
      .transpose()
      .map_err(|e| CompanyError::Repository(RepositoryError::QueryFailed(e.to_string())))?;

    let row = sqlx::query_as::<_, CompanyRow>(
      r#"
      INSERT INTO companies (id, name, email, phone, address, tax_id, vat_number, google_drive_folder_id, storage_provider, storage_config, oauth_access_token, oauth_refresh_token, oauth_token_expires_at, oauth_connected_by, oauth_connected_at, created_at, updated_at)
      VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
      RETURNING id, name, email, phone, address, tax_id, vat_number, google_drive_folder_id, storage_provider, storage_config, oauth_access_token, oauth_refresh_token, oauth_token_expires_at, oauth_connected_by, oauth_connected_at, created_at, updated_at
      "#,
    )
    .bind(company.id.to_string())
    .bind(&company.name)
    .bind(company.email.as_ref().map(|e| e.as_str().to_string()))
    .bind(company.phone.as_ref().map(|p| p.as_str().to_string()))
    .bind(address_json.as_deref())
    .bind(company.registry_code.as_ref().map(|r| r.as_str().to_string()))
    .bind(company.vat_number.as_ref().map(|v| v.as_str().to_string()))
    .bind(company.google_drive_folder_id.as_deref())
    .bind(company.storage_provider.as_deref())
    .bind(company.storage_config.as_deref())
    .bind(company.oauth_access_token.as_deref())
    .bind(company.oauth_refresh_token.as_deref())
    .bind(company.oauth_token_expires_at.map(|dt| dt.to_rfc3339()))
    .bind(company.oauth_connected_by.map(|id| id.to_string()))
    .bind(company.oauth_connected_at.map(|dt| dt.to_rfc3339()))
    .bind(company.created_at.to_rfc3339())
    .bind(company.updated_at.to_rfc3339())
    .fetch_one(&self.pool)
    .await?;

    row.try_into()
  }

  async fn find_by_id(&self, id: Uuid) -> Result<Option<Company>, CompanyError> {
    let row = sqlx::query_as::<_, CompanyRow>(
      r#"
      SELECT id, name, email, phone, address, tax_id, vat_number, google_drive_folder_id, storage_provider, storage_config, oauth_access_token, oauth_refresh_token, oauth_token_expires_at, oauth_connected_by, oauth_connected_at, created_at, updated_at
      FROM companies
      WHERE id = ?1
      "#,
    )
    .bind(id.to_string())
    .fetch_optional(&self.pool)
    .await?;

    row.map(|r| r.try_into()).transpose()
  }

  async fn update(&self, company: Company) -> Result<Company, CompanyError> {
    let address_json = company
      .address
      .as_ref()
      .map(|a| a.as_json())
      .transpose()
      .map_err(|e| CompanyError::Repository(RepositoryError::QueryFailed(e.to_string())))?;

    let row = sqlx::query_as::<_, CompanyRow>(
      r#"
      UPDATE companies
      SET name = ?2, email = ?3, phone = ?4, address = ?5, tax_id = ?6, vat_number = ?7, google_drive_folder_id = ?8, storage_provider = ?9, storage_config = ?10, oauth_access_token = ?11, oauth_refresh_token = ?12, oauth_token_expires_at = ?13, oauth_connected_by = ?14, oauth_connected_at = ?15, updated_at = ?16
      WHERE id = ?1
      RETURNING id, name, email, phone, address, tax_id, vat_number, google_drive_folder_id, storage_provider, storage_config, oauth_access_token, oauth_refresh_token, oauth_token_expires_at, oauth_connected_by, oauth_connected_at, created_at, updated_at
      "#,
    )
    .bind(company.id.to_string())
    .bind(&company.name)
    .bind(company.email.as_ref().map(|e| e.as_str().to_string()))
    .bind(company.phone.as_ref().map(|p| p.as_str().to_string()))
    .bind(address_json.as_deref())
    .bind(company.registry_code.as_ref().map(|r| r.as_str().to_string()))
    .bind(company.vat_number.as_ref().map(|v| v.as_str().to_string()))
    .bind(company.google_drive_folder_id.as_deref())
    .bind(company.storage_provider.as_deref())
    .bind(company.storage_config.as_deref())
    .bind(company.oauth_access_token.as_deref())
    .bind(company.oauth_refresh_token.as_deref())
    .bind(company.oauth_token_expires_at.map(|dt| dt.to_rfc3339()))
    .bind(company.oauth_connected_by.map(|id| id.to_string()))
    .bind(company.oauth_connected_at.map(|dt| dt.to_rfc3339()))
    .bind(company.updated_at.to_rfc3339())
    .fetch_one(&self.pool)
    .await?;

    row.try_into()
  }

  async fn delete(&self, id: Uuid) -> Result<(), CompanyError> {
    sqlx::query("DELETE FROM companies WHERE id = ?1")
      .bind(id.to_string())
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
      SET oauth_access_token = ?2,
          oauth_refresh_token = ?3,
          oauth_token_expires_at = ?4,
          oauth_connected_by = ?5,
          oauth_connected_at = COALESCE(oauth_connected_at, ?6),
          updated_at = ?6
      WHERE id = ?1
      "#,
    )
    .bind(company_id.to_string())
    .bind(encrypted_access_token)
    .bind(encrypted_refresh_token)
    .bind(expires_at.to_rfc3339())
    .bind(connected_by.to_string())
    .bind(now.to_rfc3339())
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
          updated_at = ?2
      WHERE id = ?1
      "#,
    )
    .bind(company_id.to_string())
    .bind(now.to_rfc3339())
    .execute(&self.pool)
    .await?;

    Ok(())
  }
}
