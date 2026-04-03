use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, SqlitePool};
use std::str::FromStr;
use uuid::Uuid;

use crate::domain::invoice::{
  entities::InvoiceTemplate,
  errors::InvoiceError,
  ports::InvoiceTemplateRepository,
  value_objects::{Currency, PaymentTerms, TemplateName},
};

#[derive(Debug, FromRow)]
struct TemplateRow {
  id: String,
  company_id: String,
  name: String,
  description: Option<String>,
  customer_id: String,
  bank_account_id: Option<String>,
  payment_terms: String,
  currency: String,
  created_at: String,
  updated_at: String,
  archived_at: Option<String>,
}

fn parse_template_row(row: TemplateRow) -> Result<InvoiceTemplate, InvoiceError> {
  let id = Uuid::parse_str(&row.id)
    .map_err(|e| InvoiceError::Internal(format!("Failed to parse UUID: {}", e)))?;
  let company_id = Uuid::parse_str(&row.company_id)
    .map_err(|e| InvoiceError::Internal(format!("Failed to parse UUID: {}", e)))?;
  let customer_id = Uuid::parse_str(&row.customer_id)
    .map_err(|e| InvoiceError::Internal(format!("Failed to parse UUID: {}", e)))?;
  let bank_account_id = row
    .bank_account_id
    .map(|s| Uuid::parse_str(&s))
    .transpose()
    .map_err(|e| InvoiceError::Internal(format!("Failed to parse UUID: {}", e)))?;

  let created_at = DateTime::parse_from_rfc3339(&row.created_at)
    .map(|dt| dt.with_timezone(&Utc))
    .map_err(|e| InvoiceError::Internal(format!("Failed to parse datetime: {}", e)))?;
  let updated_at = DateTime::parse_from_rfc3339(&row.updated_at)
    .map(|dt| dt.with_timezone(&Utc))
    .map_err(|e| InvoiceError::Internal(format!("Failed to parse datetime: {}", e)))?;
  let archived_at = row
    .archived_at
    .map(|s| {
      DateTime::parse_from_rfc3339(&s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| InvoiceError::Internal(format!("Failed to parse datetime: {}", e)))
    })
    .transpose()?;

  Ok(InvoiceTemplate {
    id,
    company_id,
    name: TemplateName::new(row.name)?,
    description: row.description,
    customer_id,
    bank_account_id,
    payment_terms: PaymentTerms::from_str(&row.payment_terms)?,
    currency: Currency::from_str(&row.currency)?,
    created_at,
    updated_at,
    archived_at,
  })
}

pub struct SqliteInvoiceTemplateRepository {
  pool: SqlitePool,
}

impl SqliteInvoiceTemplateRepository {
  pub fn new(pool: SqlitePool) -> Self {
    Self { pool }
  }
}

#[async_trait]
impl InvoiceTemplateRepository for SqliteInvoiceTemplateRepository {
  async fn create(&self, template: InvoiceTemplate) -> Result<InvoiceTemplate, InvoiceError> {
    let template_name = template.name.value().to_string();

    let row = sqlx::query_as::<_, TemplateRow>(
      r#"
      INSERT INTO invoice_templates (
        id, company_id, name, description, customer_id, bank_account_id,
        payment_terms, currency, created_at, updated_at, archived_at
      )
      VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
      RETURNING id, company_id, name, description, customer_id, bank_account_id,
                payment_terms, currency, created_at, updated_at, archived_at
      "#,
    )
    .bind(template.id.to_string())
    .bind(template.company_id.to_string())
    .bind(template.name.value())
    .bind(&template.description)
    .bind(template.customer_id.to_string())
    .bind(template.bank_account_id.map(|id| id.to_string()))
    .bind(template.payment_terms.as_str())
    .bind(template.currency.as_str())
    .bind(template.created_at.to_rfc3339())
    .bind(template.updated_at.to_rfc3339())
    .bind(template.archived_at.map(|dt| dt.to_rfc3339()))
    .fetch_one(&self.pool)
    .await
    .map_err(|e| {
      if let sqlx::Error::Database(db_err) = &e {
        if db_err.is_unique_violation() {
          return InvoiceError::TemplateNameAlreadyExists(template_name);
        }
      }
      InvoiceError::Database(e)
    })?;

    parse_template_row(row)
  }

  async fn update(&self, template: InvoiceTemplate) -> Result<InvoiceTemplate, InvoiceError> {
    let template_name = template.name.value().to_string();

    let row = sqlx::query_as::<_, TemplateRow>(
      r#"
      UPDATE invoice_templates
      SET name = ?2, description = ?3, customer_id = ?4, bank_account_id = ?5,
          payment_terms = ?6, currency = ?7, updated_at = ?8, archived_at = ?9
      WHERE id = ?1
      RETURNING id, company_id, name, description, customer_id, bank_account_id,
                payment_terms, currency, created_at, updated_at, archived_at
      "#,
    )
    .bind(template.id.to_string())
    .bind(template.name.value())
    .bind(&template.description)
    .bind(template.customer_id.to_string())
    .bind(template.bank_account_id.map(|id| id.to_string()))
    .bind(template.payment_terms.as_str())
    .bind(template.currency.as_str())
    .bind(template.updated_at.to_rfc3339())
    .bind(template.archived_at.map(|dt| dt.to_rfc3339()))
    .fetch_one(&self.pool)
    .await
    .map_err(|e| {
      if let sqlx::Error::Database(db_err) = &e {
        if db_err.is_unique_violation() {
          return InvoiceError::TemplateNameAlreadyExists(template_name);
        }
      }
      InvoiceError::Database(e)
    })?;

    parse_template_row(row)
  }

  async fn find_by_id(&self, id: Uuid) -> Result<Option<InvoiceTemplate>, InvoiceError> {
    let row = sqlx::query_as::<_, TemplateRow>(
      r#"
      SELECT id, company_id, name, description, customer_id, bank_account_id,
             payment_terms, currency, created_at, updated_at, archived_at
      FROM invoice_templates
      WHERE id = ?1
      "#,
    )
    .bind(id.to_string())
    .fetch_optional(&self.pool)
    .await?;

    row.map(parse_template_row).transpose()
  }

  async fn find_by_company_id(
    &self,
    company_id: Uuid,
  ) -> Result<Vec<InvoiceTemplate>, InvoiceError> {
    let rows = sqlx::query_as::<_, TemplateRow>(
      r#"
      SELECT id, company_id, name, description, customer_id, bank_account_id,
             payment_terms, currency, created_at, updated_at, archived_at
      FROM invoice_templates
      WHERE company_id = ?1
      ORDER BY created_at DESC
      "#,
    )
    .bind(company_id.to_string())
    .fetch_all(&self.pool)
    .await?;

    rows.into_iter().map(parse_template_row).collect()
  }

  async fn find_active_by_company_id(
    &self,
    company_id: Uuid,
  ) -> Result<Vec<InvoiceTemplate>, InvoiceError> {
    let rows = sqlx::query_as::<_, TemplateRow>(
      r#"
      SELECT id, company_id, name, description, customer_id, bank_account_id,
             payment_terms, currency, created_at, updated_at, archived_at
      FROM invoice_templates
      WHERE company_id = ?1 AND archived_at IS NULL
      ORDER BY created_at DESC
      "#,
    )
    .bind(company_id.to_string())
    .fetch_all(&self.pool)
    .await?;

    rows.into_iter().map(parse_template_row).collect()
  }

  async fn exists_by_name(
    &self,
    company_id: Uuid,
    name: &str,
    exclude_id: Option<Uuid>,
  ) -> Result<bool, InvoiceError> {
    let count: i32 = if let Some(id) = exclude_id {
      sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM invoice_templates
        WHERE company_id = ?1 AND name = ?2 AND id != ?3
        "#,
      )
      .bind(company_id.to_string())
      .bind(name)
      .bind(id.to_string())
      .fetch_one(&self.pool)
      .await?
    } else {
      sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM invoice_templates
        WHERE company_id = ?1 AND name = ?2
        "#,
      )
      .bind(company_id.to_string())
      .bind(name)
      .fetch_one(&self.pool)
      .await?
    };

    Ok(count > 0)
  }
}
