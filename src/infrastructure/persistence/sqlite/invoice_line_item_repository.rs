use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::{FromRow, SqlitePool};
use std::str::FromStr;
use uuid::Uuid;

use crate::domain::invoice::{
  Currency, InvoiceLineItem, LineItemDescription, Money, Quantity, VatRate, errors::InvoiceError,
  ports::InvoiceLineItemRepository,
};

#[derive(Debug, FromRow)]
struct LineItemRow {
  id: String,
  invoice_id: String,
  description: String,
  quantity: String,
  unit_price_amount: String,
  unit_price_currency: String,
  vat_rate: String,
  line_order: i32,
}

fn parse_line_item_row(row: LineItemRow) -> Result<InvoiceLineItem, InvoiceError> {
  let id = Uuid::parse_str(&row.id)
    .map_err(|e| InvoiceError::Internal(format!("Failed to parse UUID: {}", e)))?;
  let invoice_id = Uuid::parse_str(&row.invoice_id)
    .map_err(|e| InvoiceError::Internal(format!("Failed to parse UUID: {}", e)))?;
  let description = LineItemDescription::new(row.description)?;
  let quantity_val = Decimal::from_str(&row.quantity)
    .map_err(|e| InvoiceError::Internal(format!("Failed to parse decimal: {}", e)))?;
  let quantity = Quantity::new(quantity_val)?;
  let amount = Decimal::from_str(&row.unit_price_amount)
    .map_err(|e| InvoiceError::Internal(format!("Failed to parse decimal: {}", e)))?;
  let currency = Currency::from_str(&row.unit_price_currency)?;
  let unit_price = Money::new(amount, currency)?;
  let vat_rate_val = Decimal::from_str(&row.vat_rate)
    .map_err(|e| InvoiceError::Internal(format!("Failed to parse decimal: {}", e)))?;
  let vat_rate = VatRate::new(vat_rate_val)?;

  Ok(InvoiceLineItem {
    id,
    invoice_id,
    description,
    quantity,
    unit_price,
    vat_rate,
    line_order: row.line_order,
  })
}

pub struct SqliteInvoiceLineItemRepository {
  pool: SqlitePool,
}

impl SqliteInvoiceLineItemRepository {
  pub fn new(pool: SqlitePool) -> Self {
    Self { pool }
  }
}

#[async_trait]
impl InvoiceLineItemRepository for SqliteInvoiceLineItemRepository {
  async fn create(&self, line_item: InvoiceLineItem) -> Result<InvoiceLineItem, InvoiceError> {
    let row = sqlx::query_as::<_, LineItemRow>(
      r#"
      INSERT INTO invoice_line_items (
          id, invoice_id, description, quantity,
          unit_price_amount, unit_price_currency, vat_rate, line_order
      )
      VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
      RETURNING id, invoice_id, description, quantity,
                unit_price_amount, unit_price_currency, vat_rate, line_order
      "#,
    )
    .bind(line_item.id.to_string())
    .bind(line_item.invoice_id.to_string())
    .bind(line_item.description.value())
    .bind(line_item.quantity.value().to_string())
    .bind(line_item.unit_price.amount.to_string())
    .bind(line_item.unit_price.currency.as_str())
    .bind(line_item.vat_rate.value().to_string())
    .bind(line_item.line_order)
    .fetch_one(&self.pool)
    .await?;

    parse_line_item_row(row)
  }

  async fn create_many(
    &self,
    line_items: Vec<InvoiceLineItem>,
  ) -> Result<Vec<InvoiceLineItem>, InvoiceError> {
    let mut created_items = Vec::new();

    for item in line_items {
      let created = self.create(item).await?;
      created_items.push(created);
    }

    Ok(created_items)
  }

  async fn update(&self, line_item: InvoiceLineItem) -> Result<InvoiceLineItem, InvoiceError> {
    let row = sqlx::query_as::<_, LineItemRow>(
      r#"
      UPDATE invoice_line_items
      SET description = ?2, quantity = ?3, unit_price_amount = ?4,
          unit_price_currency = ?5, vat_rate = ?6, line_order = ?7
      WHERE id = ?1
      RETURNING id, invoice_id, description, quantity,
                unit_price_amount, unit_price_currency, vat_rate, line_order
      "#,
    )
    .bind(line_item.id.to_string())
    .bind(line_item.description.value())
    .bind(line_item.quantity.value().to_string())
    .bind(line_item.unit_price.amount.to_string())
    .bind(line_item.unit_price.currency.as_str())
    .bind(line_item.vat_rate.value().to_string())
    .bind(line_item.line_order)
    .fetch_one(&self.pool)
    .await?;

    parse_line_item_row(row)
  }

  async fn delete(&self, id: Uuid) -> Result<(), InvoiceError> {
    sqlx::query("DELETE FROM invoice_line_items WHERE id = ?1")
      .bind(id.to_string())
      .execute(&self.pool)
      .await?;

    Ok(())
  }

  async fn delete_by_invoice_id(&self, invoice_id: Uuid) -> Result<(), InvoiceError> {
    sqlx::query("DELETE FROM invoice_line_items WHERE invoice_id = ?1")
      .bind(invoice_id.to_string())
      .execute(&self.pool)
      .await?;

    Ok(())
  }

  async fn find_by_id(&self, id: Uuid) -> Result<Option<InvoiceLineItem>, InvoiceError> {
    let row = sqlx::query_as::<_, LineItemRow>(
      r#"
      SELECT id, invoice_id, description, quantity,
             unit_price_amount, unit_price_currency, vat_rate, line_order
      FROM invoice_line_items
      WHERE id = ?1
      "#,
    )
    .bind(id.to_string())
    .fetch_optional(&self.pool)
    .await?;

    row.map(parse_line_item_row).transpose()
  }

  async fn find_by_invoice_id(
    &self,
    invoice_id: Uuid,
  ) -> Result<Vec<InvoiceLineItem>, InvoiceError> {
    let rows = sqlx::query_as::<_, LineItemRow>(
      r#"
      SELECT id, invoice_id, description, quantity,
             unit_price_amount, unit_price_currency, vat_rate, line_order
      FROM invoice_line_items
      WHERE invoice_id = ?1
      ORDER BY line_order ASC
      "#,
    )
    .bind(invoice_id.to_string())
    .fetch_all(&self.pool)
    .await?;

    rows.into_iter().map(parse_line_item_row).collect()
  }
}
