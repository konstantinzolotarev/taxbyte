use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::domain::invoice::{
  Customer, CustomerName, errors::InvoiceError, ports::CustomerRepository,
  value_objects::CustomerAddress,
};

#[derive(Debug, FromRow)]
struct CustomerRow {
  id: String,
  company_id: String,
  name: String,
  address: Option<String>,
  created_at: String,
  updated_at: String,
  archived_at: Option<String>,
}

fn parse_customer_row(row: CustomerRow) -> Result<Customer, InvoiceError> {
  let id = Uuid::parse_str(&row.id)
    .map_err(|e| InvoiceError::Internal(format!("Failed to parse UUID: {}", e)))?;
  let company_id = Uuid::parse_str(&row.company_id)
    .map_err(|e| InvoiceError::Internal(format!("Failed to parse UUID: {}", e)))?;
  let name = CustomerName::new(row.name)?;
  let address = if let Some(addr_str) = row.address {
    Some(
      serde_json::from_str::<CustomerAddress>(&addr_str)
        .map_err(|e| InvoiceError::Internal(format!("Failed to parse customer address: {}", e)))?,
    )
  } else {
    None
  };
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

  Ok(Customer {
    id,
    company_id,
    name,
    address,
    created_at,
    updated_at,
    archived_at,
  })
}

pub struct SqliteCustomerRepository {
  pool: SqlitePool,
}

impl SqliteCustomerRepository {
  pub fn new(pool: SqlitePool) -> Self {
    Self { pool }
  }
}

#[async_trait]
impl CustomerRepository for SqliteCustomerRepository {
  async fn create(&self, customer: Customer) -> Result<Customer, InvoiceError> {
    let address_json = customer
      .address
      .as_ref()
      .map(serde_json::to_string)
      .transpose()
      .map_err(|e| InvoiceError::Internal(format!("Failed to serialize address: {}", e)))?;

    let row = sqlx::query_as::<_, CustomerRow>(
      r#"
      INSERT INTO customers (id, company_id, name, address, created_at, updated_at, archived_at)
      VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
      RETURNING id, company_id, name, address, created_at, updated_at, archived_at
      "#,
    )
    .bind(customer.id.to_string())
    .bind(customer.company_id.to_string())
    .bind(customer.name.value())
    .bind(address_json.as_deref())
    .bind(customer.created_at.to_rfc3339())
    .bind(customer.updated_at.to_rfc3339())
    .bind(customer.archived_at.map(|dt| dt.to_rfc3339()))
    .fetch_one(&self.pool)
    .await?;

    parse_customer_row(row)
  }

  async fn update(&self, customer: Customer) -> Result<Customer, InvoiceError> {
    let address_json = customer
      .address
      .as_ref()
      .map(serde_json::to_string)
      .transpose()
      .map_err(|e| InvoiceError::Internal(format!("Failed to serialize address: {}", e)))?;

    let row = sqlx::query_as::<_, CustomerRow>(
      r#"
      UPDATE customers
      SET name = ?2, address = ?3, updated_at = ?4, archived_at = ?5
      WHERE id = ?1
      RETURNING id, company_id, name, address, created_at, updated_at, archived_at
      "#,
    )
    .bind(customer.id.to_string())
    .bind(customer.name.value())
    .bind(address_json.as_deref())
    .bind(customer.updated_at.to_rfc3339())
    .bind(customer.archived_at.map(|dt| dt.to_rfc3339()))
    .fetch_one(&self.pool)
    .await?;

    parse_customer_row(row)
  }

  async fn find_by_id(&self, id: Uuid) -> Result<Option<Customer>, InvoiceError> {
    let row = sqlx::query_as::<_, CustomerRow>(
      r#"
      SELECT id, company_id, name, address, created_at, updated_at, archived_at
      FROM customers
      WHERE id = ?1
      "#,
    )
    .bind(id.to_string())
    .fetch_optional(&self.pool)
    .await?;

    row.map(parse_customer_row).transpose()
  }

  async fn find_by_company_id(&self, company_id: Uuid) -> Result<Vec<Customer>, InvoiceError> {
    let rows = sqlx::query_as::<_, CustomerRow>(
      r#"
      SELECT id, company_id, name, address, created_at, updated_at, archived_at
      FROM customers
      WHERE company_id = ?1
      ORDER BY name ASC
      "#,
    )
    .bind(company_id.to_string())
    .fetch_all(&self.pool)
    .await?;

    rows.into_iter().map(parse_customer_row).collect()
  }

  async fn find_active_by_company_id(
    &self,
    company_id: Uuid,
  ) -> Result<Vec<Customer>, InvoiceError> {
    let rows = sqlx::query_as::<_, CustomerRow>(
      r#"
      SELECT id, company_id, name, address, created_at, updated_at, archived_at
      FROM customers
      WHERE company_id = ?1 AND archived_at IS NULL
      ORDER BY name ASC
      "#,
    )
    .bind(company_id.to_string())
    .fetch_all(&self.pool)
    .await?;

    rows.into_iter().map(parse_customer_row).collect()
  }

  async fn exists_by_name(
    &self,
    company_id: Uuid,
    name: &str,
    exclude_id: Option<Uuid>,
  ) -> Result<bool, InvoiceError> {
    let count: i32 = if let Some(exclude_id) = exclude_id {
      sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM customers
        WHERE company_id = ?1 AND name = ?2 AND id != ?3 AND archived_at IS NULL
        "#,
      )
      .bind(company_id.to_string())
      .bind(name)
      .bind(exclude_id.to_string())
      .fetch_one(&self.pool)
      .await?
    } else {
      sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM customers
        WHERE company_id = ?1 AND name = ?2 AND archived_at IS NULL
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
