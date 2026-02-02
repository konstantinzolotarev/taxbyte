use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::invoice::{InvoiceError, InvoiceService, InvoiceStatus};

#[derive(Debug, Deserialize)]
pub struct ChangeInvoiceStatusCommand {
  pub user_id: Uuid,
  pub invoice_id: Uuid,
  pub new_status: String,
}

#[derive(Debug, Serialize)]
pub struct ChangeInvoiceStatusResponse {
  pub invoice_id: Uuid,
  pub status: String,
}

pub struct ChangeInvoiceStatusUseCase {
  invoice_service: Arc<InvoiceService>,
}

impl ChangeInvoiceStatusUseCase {
  pub fn new(invoice_service: Arc<InvoiceService>) -> Self {
    Self { invoice_service }
  }

  pub async fn execute(
    &self,
    command: ChangeInvoiceStatusCommand,
  ) -> Result<ChangeInvoiceStatusResponse, InvoiceError> {
    let new_status = InvoiceStatus::from_str(&command.new_status)?;

    let invoice = self
      .invoice_service
      .change_invoice_status(command.user_id, command.invoice_id, new_status)
      .await?;

    Ok(ChangeInvoiceStatusResponse {
      invoice_id: invoice.id,
      status: invoice.status.as_str().to_string(),
    })
  }
}
