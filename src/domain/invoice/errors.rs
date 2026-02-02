use super::value_objects::ValueObjectError;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum InvoiceError {
  #[error("Validation error: {0}")]
  Validation(#[from] ValueObjectError),

  #[error("Customer not found: {0}")]
  CustomerNotFound(Uuid),

  #[error("Invoice not found: {0}")]
  InvoiceNotFound(Uuid),

  #[error("Line item not found: {0}")]
  LineItemNotFound(Uuid),

  #[error("Customer name already exists for company")]
  CustomerNameAlreadyExists,

  #[error("Invoice number '{0}' already exists")]
  InvoiceNumberAlreadyExists(String),

  #[error("Cannot edit invoice: {0}")]
  CannotEditInvoice(String),

  #[error("Invalid status transition: {0}")]
  InvalidStatusTransition(String),

  #[error("Permission denied: {0}")]
  PermissionDenied(String),

  #[error("Currency mismatch: expected {expected}, got {actual}")]
  CurrencyMismatch { expected: String, actual: String },

  #[error("No line items provided")]
  NoLineItems,

  #[error("Invalid line item order")]
  InvalidLineItemOrder,

  #[error("PDF generation failed: {0}")]
  PdfGenerationFailed(String),

  #[error("Repository error: {0}")]
  Repository(String),

  #[error("Database error: {0}")]
  Database(#[from] sqlx::Error),

  #[error("Internal error: {0}")]
  Internal(String),
}
