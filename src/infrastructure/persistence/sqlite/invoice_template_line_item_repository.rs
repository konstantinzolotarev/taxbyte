use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::{FromRow, SqlitePool};
use std::str::FromStr;
use uuid::Uuid;

use crate::domain::invoice::{
  entities::InvoiceTemplateLineItem,
  errors::InvoiceError,
  ports::InvoiceTemplateLineItemRepository,
  value_objects::{Currency, LineItemDescription, Money, Quantity, VatRate},
};

#[derive(Debug, FromRow)]
struct TemplateLineItemRow {
  id: String,
  template_id: String,
  description: String,
  quantity: String,
  unit_price_amount: String,
  unit_price_currency: String,
  vat_rate: String,
  line_order: i32,
}

fn parse_template_line_item_row(
  row: TemplateLineItemRow,
) -> Result<InvoiceTemplateLineItem, InvoiceError> {
  let id = Uuid::parse_str(&row.id)
    .map_err(|e| InvoiceError::Internal(format!("Failed to parse UUID: {}", e)))?;
  let template_id = Uuid::parse_str(&row.template_id)
    .map_err(|e| InvoiceError::Internal(format!("Failed to parse UUID: {}", e)))?;
  let quantity_val = Decimal::from_str(&row.quantity)
    .map_err(|e| InvoiceError::Internal(format!("Failed to parse decimal: {}", e)))?;
  let amount = Decimal::from_str(&row.unit_price_amount)
    .map_err(|e| InvoiceError::Internal(format!("Failed to parse decimal: {}", e)))?;
  let vat_rate_val = Decimal::from_str(&row.vat_rate)
    .map_err(|e| InvoiceError::Internal(format!("Failed to parse decimal: {}", e)))?;

  Ok(InvoiceTemplateLineItem {
    id,
    template_id,
    description: LineItemDescription::new(row.description)?,
    quantity: Quantity::new(quantity_val)?,
    unit_price: Money::new(amount, Currency::from_str(&row.unit_price_currency)?)?,
    vat_rate: VatRate::new(vat_rate_val)?,
    line_order: row.line_order,
  })
}

pub struct SqliteInvoiceTemplateLineItemRepository {
  pool: SqlitePool,
}

impl SqliteInvoiceTemplateLineItemRepository {
  pub fn new(pool: SqlitePool) -> Self {
    Self { pool }
  }
}

#[async_trait]
impl InvoiceTemplateLineItemRepository for SqliteInvoiceTemplateLineItemRepository {
  async fn create_many(
    &self,
    items: Vec<InvoiceTemplateLineItem>,
  ) -> Result<Vec<InvoiceTemplateLineItem>, InvoiceError> {
    if items.is_empty() {
      return Ok(Vec::new());
    }

    let mut created_items = Vec::new();

    for item in items {
      let row = sqlx::query_as::<_, TemplateLineItemRow>(
        r#"
        INSERT INTO invoice_template_line_items (
          id, template_id, description, quantity,
          unit_price_amount, unit_price_currency, vat_rate, line_order
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        RETURNING id, template_id, description, quantity,
                  unit_price_amount, unit_price_currency, vat_rate, line_order
        "#,
      )
      .bind(item.id.to_string())
      .bind(item.template_id.to_string())
      .bind(item.description.value())
      .bind(item.quantity.value().to_string())
      .bind(item.unit_price.amount.to_string())
      .bind(item.unit_price.currency.as_str())
      .bind(item.vat_rate.value().to_string())
      .bind(item.line_order)
      .fetch_one(&self.pool)
      .await?;

      created_items.push(parse_template_line_item_row(row)?);
    }

    Ok(created_items)
  }

  async fn find_by_template_id(
    &self,
    template_id: Uuid,
  ) -> Result<Vec<InvoiceTemplateLineItem>, InvoiceError> {
    let rows = sqlx::query_as::<_, TemplateLineItemRow>(
      r#"
      SELECT id, template_id, description, quantity,
             unit_price_amount, unit_price_currency, vat_rate, line_order
      FROM invoice_template_line_items
      WHERE template_id = ?1
      ORDER BY line_order ASC
      "#,
    )
    .bind(template_id.to_string())
    .fetch_all(&self.pool)
    .await?;

    rows.into_iter().map(parse_template_line_item_row).collect()
  }

  async fn delete_by_template_id(&self, template_id: Uuid) -> Result<(), InvoiceError> {
    sqlx::query("DELETE FROM invoice_template_line_items WHERE template_id = ?1")
      .bind(template_id.to_string())
      .execute(&self.pool)
      .await?;

    Ok(())
  }
}
