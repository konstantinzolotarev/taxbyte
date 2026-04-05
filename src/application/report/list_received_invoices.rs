use std::sync::Arc;

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::Serialize;
use uuid::Uuid;

use crate::domain::report::{errors::ReportError, services::ReportService};

#[derive(Debug)]
pub struct ListReceivedInvoicesCommand {
  pub company_id: Uuid,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReceivedInvoiceSummary {
  pub id: Uuid,
  pub vendor_name: String,
  pub amount: Decimal,
  pub currency: String,
  pub invoice_date: Option<NaiveDate>,
  pub invoice_number: Option<String>,
  pub notes: Option<String>,
  pub created_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct ListReceivedInvoicesResponse {
  pub invoices: Vec<ReceivedInvoiceSummary>,
}

pub struct ListReceivedInvoicesUseCase {
  report_service: Arc<ReportService>,
}

impl ListReceivedInvoicesUseCase {
  pub fn new(report_service: Arc<ReportService>) -> Self {
    Self { report_service }
  }

  pub async fn execute(
    &self,
    command: ListReceivedInvoicesCommand,
  ) -> Result<ListReceivedInvoicesResponse, ReportError> {
    let invoices = self
      .report_service
      .list_received_invoices(command.company_id)
      .await?;

    let summaries = invoices
      .into_iter()
      .map(|i| ReceivedInvoiceSummary {
        id: i.id,
        vendor_name: i.vendor_name,
        amount: i.amount,
        currency: i.currency,
        invoice_date: i.invoice_date,
        invoice_number: i.invoice_number,
        notes: i.notes,
        created_at: i.created_at,
      })
      .collect();

    Ok(ListReceivedInvoicesResponse {
      invoices: summaries,
    })
  }
}
