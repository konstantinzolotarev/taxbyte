use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::invoice::{InvoiceError, InvoiceService};

#[derive(Debug, Deserialize)]
pub struct ListArchivedInvoicesCommand {
  pub user_id: Uuid,
  pub company_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct ArchivedInvoiceListItemDto {
  pub id: Uuid,
  pub invoice_number: String,
  pub customer_id: Uuid,
  pub invoice_date: NaiveDate,
  pub due_date: NaiveDate,
  pub currency: String,
  pub status: String,
  pub created_at: DateTime<Utc>,
  pub archived_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct ListArchivedInvoicesResponse {
  pub invoices: Vec<ArchivedInvoiceListItemDto>,
}

pub struct ListArchivedInvoicesUseCase {
  invoice_service: Arc<InvoiceService>,
}

impl ListArchivedInvoicesUseCase {
  pub fn new(invoice_service: Arc<InvoiceService>) -> Self {
    Self { invoice_service }
  }

  pub async fn execute(
    &self,
    command: ListArchivedInvoicesCommand,
  ) -> Result<ListArchivedInvoicesResponse, InvoiceError> {
    let invoices = self
      .invoice_service
      .list_archived_invoices(command.user_id, command.company_id)
      .await?;

    let invoice_dtos = invoices
      .into_iter()
      .map(|i| ArchivedInvoiceListItemDto {
        id: i.id,
        invoice_number: i.invoice_number.to_string(),
        customer_id: i.customer_id,
        invoice_date: i.invoice_date,
        due_date: i.due_date,
        currency: i.currency.as_str().to_string(),
        status: i.status.as_str().to_string(),
        created_at: i.created_at,
        archived_at: i.archived_at,
      })
      .collect();

    Ok(ListArchivedInvoicesResponse {
      invoices: invoice_dtos,
    })
  }
}
