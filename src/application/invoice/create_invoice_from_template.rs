use chrono::NaiveDate;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::invoice::{InvoiceError, InvoiceService};

use super::create_invoice::{
  CreateInvoiceCommand, CreateInvoiceLineItemDto, CreateInvoiceResponse, CreateInvoiceUseCase,
};

#[derive(Debug, Deserialize)]
pub struct CreateInvoiceFromTemplateCommand {
  pub user_id: Uuid,
  pub template_id: Uuid,
  pub invoice_number: String,
  pub invoice_date: NaiveDate,
}

pub struct CreateInvoiceFromTemplateUseCase {
  invoice_service: Arc<InvoiceService>,
  create_invoice_use_case: Arc<CreateInvoiceUseCase>,
}

impl CreateInvoiceFromTemplateUseCase {
  pub fn new(
    invoice_service: Arc<InvoiceService>,
    create_invoice_use_case: Arc<CreateInvoiceUseCase>,
  ) -> Self {
    Self {
      invoice_service,
      create_invoice_use_case,
    }
  }

  pub async fn execute(
    &self,
    command: CreateInvoiceFromTemplateCommand,
  ) -> Result<CreateInvoiceResponse, InvoiceError> {
    // Get template with items
    let (template, items) = self
      .invoice_service
      .get_template_with_items(command.user_id, command.template_id)
      .await?;

    // Convert template items to invoice line item DTOs
    let line_items = items
      .into_iter()
      .map(|item| CreateInvoiceLineItemDto {
        description: item.description.value().to_string(),
        quantity: item.quantity.value(),
        unit_price: item.unit_price.amount,
        vat_rate: item.vat_rate.value(),
      })
      .collect();

    // Create invoice using standard create flow
    let create_command = CreateInvoiceCommand {
      user_id: command.user_id,
      company_id: template.company_id,
      customer_id: template.customer_id,
      bank_account_id: template.bank_account_id,
      invoice_number: command.invoice_number,
      invoice_date: command.invoice_date,
      payment_terms: template.payment_terms.as_str(),
      currency: template.currency.as_str().to_string(),
      line_items,
    };

    self.create_invoice_use_case.execute(create_command).await
  }
}
