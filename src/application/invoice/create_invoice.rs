use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::invoice::{
  Currency, InvoiceData, InvoiceError, InvoiceService, LineItemDescription, Money, PaymentTerms,
  Quantity, VatRate,
};

#[derive(Debug, Deserialize)]
pub struct CreateInvoiceLineItemDto {
  pub description: String,
  pub quantity: Decimal,
  pub unit_price: Decimal,
  pub vat_rate: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct CreateInvoiceCommand {
  pub user_id: Uuid,
  pub company_id: Uuid,
  pub customer_id: Uuid,
  pub bank_account_id: Option<Uuid>,
  pub invoice_number: String,
  pub invoice_date: NaiveDate,
  pub payment_terms: String,
  pub currency: String,
  pub line_items: Vec<CreateInvoiceLineItemDto>,
}

#[derive(Debug, Serialize)]
pub struct CreateInvoiceResponse {
  pub invoice_id: Uuid,
  pub invoice_number: String,
  pub created_at: DateTime<Utc>,
}

pub struct CreateInvoiceUseCase {
  invoice_service: Arc<InvoiceService>,
}

impl CreateInvoiceUseCase {
  pub fn new(invoice_service: Arc<InvoiceService>) -> Self {
    Self { invoice_service }
  }

  pub async fn execute(
    &self,
    command: CreateInvoiceCommand,
  ) -> Result<CreateInvoiceResponse, InvoiceError> {
    let payment_terms = PaymentTerms::from_str(&command.payment_terms)?;
    let currency = Currency::from_str(&command.currency)?;

    let line_items: Vec<_> = command
      .line_items
      .into_iter()
      .map(|item| {
        let description = LineItemDescription::new(item.description)?;
        let quantity = Quantity::new(item.quantity)?;
        let unit_price = Money::new(item.unit_price, currency)?;
        let vat_rate = VatRate::new(item.vat_rate)?;
        Ok((description, quantity, unit_price, vat_rate))
      })
      .collect::<Result<Vec<_>, InvoiceError>>()?;

    let invoice_data = InvoiceData {
      customer_id: command.customer_id,
      bank_account_id: command.bank_account_id,
      invoice_number: command.invoice_number,
      invoice_date: command.invoice_date,
      payment_terms,
      currency,
      line_items,
    };

    let (invoice, _line_items) = self
      .invoice_service
      .create_invoice(command.user_id, command.company_id, invoice_data)
      .await?;

    Ok(CreateInvoiceResponse {
      invoice_id: invoice.id,
      invoice_number: invoice.invoice_number.into_inner(),
      created_at: invoice.created_at,
    })
  }
}
