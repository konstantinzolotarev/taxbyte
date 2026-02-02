use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::invoice::{InvoiceError, InvoiceService, InvoiceStatus};

#[derive(Debug, Deserialize)]
pub struct ListInvoicesCommand {
  pub user_id: Uuid,
  pub company_id: Uuid,
  pub status_filter: Option<String>,
  pub customer_filter: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct InvoiceListItemDto {
  pub id: Uuid,
  pub invoice_number: String,
  pub customer_id: Uuid,
  pub invoice_date: NaiveDate,
  pub due_date: NaiveDate,
  pub currency: String,
  pub status: String,
  pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ListInvoicesResponse {
  pub invoices: Vec<InvoiceListItemDto>,
}

pub struct ListInvoicesUseCase {
  invoice_service: Arc<InvoiceService>,
}

impl ListInvoicesUseCase {
  pub fn new(invoice_service: Arc<InvoiceService>) -> Self {
    Self { invoice_service }
  }

  pub async fn execute(
    &self,
    command: ListInvoicesCommand,
  ) -> Result<ListInvoicesResponse, InvoiceError> {
    let status_filter = if let Some(status_str) = command.status_filter {
      Some(InvoiceStatus::from_str(&status_str)?)
    } else {
      None
    };

    let invoices = self
      .invoice_service
      .list_invoices(
        command.user_id,
        command.company_id,
        status_filter,
        command.customer_filter,
      )
      .await?;

    let invoice_dtos = invoices
      .into_iter()
      .map(|i| InvoiceListItemDto {
        id: i.id,
        invoice_number: i.invoice_number.to_string(),
        customer_id: i.customer_id,
        invoice_date: i.invoice_date,
        due_date: i.due_date,
        currency: i.currency.as_str().to_string(),
        status: i.status.as_str().to_string(),
        created_at: i.created_at,
      })
      .collect();

    Ok(ListInvoicesResponse {
      invoices: invoice_dtos,
    })
  }
}
