use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::invoice::{InvoiceError, InvoiceService};

#[derive(Debug, Deserialize)]
pub struct ArchiveInvoiceCommand {
  pub user_id: Uuid,
  pub invoice_id: Uuid,
}

pub struct ArchiveInvoiceUseCase {
  invoice_service: Arc<InvoiceService>,
}

impl ArchiveInvoiceUseCase {
  pub fn new(invoice_service: Arc<InvoiceService>) -> Self {
    Self { invoice_service }
  }

  pub async fn execute(&self, command: ArchiveInvoiceCommand) -> Result<(), InvoiceError> {
    self
      .invoice_service
      .archive_invoice(command.user_id, command.invoice_id)
      .await
  }
}
