use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::invoice::InvoiceError;
use crate::domain::invoice::InvoiceService;

#[derive(Debug, Deserialize)]
pub struct GetInvoiceDetailsCommand {
  pub user_id: Uuid,
  pub invoice_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct InvoiceLineItemDto {
  pub id: Uuid,
  pub description: String,
  pub quantity: Decimal,
  pub unit_price: Decimal,
  pub vat_rate: Decimal,
  pub currency: String,
  pub line_order: i32,
  pub subtotal: Decimal,
  pub vat_amount: Decimal,
  pub total: Decimal,
}

#[derive(Debug, Serialize)]
pub struct InvoiceTotalsDto {
  pub subtotal: Decimal,
  pub total_vat: Decimal,
  pub grand_total: Decimal,
  pub currency: String,
}

#[derive(Debug, Serialize)]
pub struct CustomerDetailsDto {
  pub id: Uuid,
  pub name: String,
  pub street: Option<String>,
  pub city: Option<String>,
  pub state: Option<String>,
  pub postal_code: Option<String>,
  pub country: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CompanyDetailsDto {
  pub id: Uuid,
  pub name: String,
  pub email: Option<String>,
  pub phone: Option<String>,
  pub street: Option<String>,
  pub city: Option<String>,
  pub state: Option<String>,
  pub postal_code: Option<String>,
  pub country: Option<String>,
  pub registry_code: Option<String>,
  pub vat_number: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BankAccountDetailsDto {
  pub id: Uuid,
  pub name: String,
  pub iban: String,
  pub iban_formatted: String,
  pub bank_details: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct InvoiceDetailsResponse {
  pub id: Uuid,
  pub company_id: Uuid,
  pub company: CompanyDetailsDto,
  pub customer: CustomerDetailsDto,
  pub bank_account_id: Option<Uuid>,
  pub bank_account: Option<BankAccountDetailsDto>,
  pub invoice_number: String,
  pub invoice_date: NaiveDate,
  pub due_date: NaiveDate,
  pub payment_terms: String,
  pub currency: String,
  pub status: String,
  pub pdf_path: Option<String>,
  pub line_items: Vec<InvoiceLineItemDto>,
  pub totals: InvoiceTotalsDto,
  pub created_at: DateTime<Utc>,
  pub updated_at: DateTime<Utc>,
}

pub struct GetInvoiceDetailsUseCase {
  invoice_service: Arc<InvoiceService>,
}

impl GetInvoiceDetailsUseCase {
  pub fn new(invoice_service: Arc<InvoiceService>) -> Self {
    Self { invoice_service }
  }

  pub async fn execute(
    &self,
    command: GetInvoiceDetailsCommand,
  ) -> Result<InvoiceDetailsResponse, InvoiceError> {
    let (invoice, line_items, customer, company, bank_account, totals) = self
      .invoice_service
      .get_invoice_with_details(command.user_id, command.invoice_id)
      .await?;

    let line_item_dtos = line_items
      .iter()
      .map(|item| InvoiceLineItemDto {
        id: item.id,
        description: item.description.value().to_string(),
        quantity: item.quantity.value(),
        unit_price: item.unit_price.amount,
        vat_rate: item.vat_rate.value(),
        currency: item.unit_price.currency.as_str().to_string(),
        line_order: item.line_order,
        subtotal: item.subtotal().amount,
        vat_amount: item.vat_amount().amount,
        total: item.total().amount,
      })
      .collect();

    let customer_dto = CustomerDetailsDto {
      id: customer.id,
      name: customer.name.value().to_string(),
      street: customer.address.as_ref().and_then(|a| a.street.clone()),
      city: customer.address.as_ref().and_then(|a| a.city.clone()),
      state: customer.address.as_ref().and_then(|a| a.state.clone()),
      postal_code: customer
        .address
        .as_ref()
        .and_then(|a| a.postal_code.clone()),
      country: customer.address.as_ref().and_then(|a| a.country.clone()),
    };

    let company_dto = CompanyDetailsDto {
      id: company.id,
      name: company.name,
      email: company.email.map(|e| e.as_str().to_string()),
      phone: company.phone.map(|p| p.as_str().to_string()),
      street: company.address.as_ref().and_then(|a| a.street.clone()),
      city: company.address.as_ref().and_then(|a| a.city.clone()),
      state: company.address.as_ref().and_then(|a| a.state.clone()),
      postal_code: company.address.as_ref().and_then(|a| a.postal_code.clone()),
      country: company.address.as_ref().and_then(|a| a.country.clone()),
      registry_code: company.registry_code.map(|r| r.as_str().to_string()),
      vat_number: company.vat_number.map(|v| v.as_str().to_string()),
    };

    let bank_account_dto = bank_account.map(|account| BankAccountDetailsDto {
      id: account.id,
      name: account.name.as_str().to_string(),
      iban: account.iban.clone().into_inner(),
      iban_formatted: account.iban.formatted(),
      bank_details: account.bank_details.map(|d| d.into_inner()),
    });

    let totals_dto = InvoiceTotalsDto {
      subtotal: totals.subtotal.amount,
      total_vat: totals.total_vat.amount,
      grand_total: totals.grand_total.amount,
      currency: totals.subtotal.currency.as_str().to_string(),
    };

    Ok(InvoiceDetailsResponse {
      id: invoice.id,
      company_id: invoice.company_id,
      company: company_dto,
      customer: customer_dto,
      bank_account_id: invoice.bank_account_id,
      bank_account: bank_account_dto,
      invoice_number: invoice.invoice_number.to_string(),
      invoice_date: invoice.invoice_date,
      due_date: invoice.due_date,
      payment_terms: invoice.payment_terms.to_string(),
      currency: invoice.currency.as_str().to_string(),
      status: invoice.status.as_str().to_string(),
      pdf_path: invoice.pdf_path,
      line_items: line_item_dtos,
      totals: totals_dto,
      created_at: invoice.created_at,
      updated_at: invoice.updated_at,
    })
  }
}
