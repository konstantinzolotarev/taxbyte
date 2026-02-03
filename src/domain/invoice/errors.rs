use super::value_objects::{InvoiceStatus, ValueObjectError};
use thiserror::Error;
use uuid::Uuid;

/// Entity-level errors for Invoice operations
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum InvoiceEntityError {
  #[error("Cannot edit invoice with status '{current_status}'. Only draft invoices can be edited.")]
  CannotEditNonDraftInvoice { current_status: InvoiceStatus },

  #[error("Invalid status transition from '{from}' to '{to}'")]
  InvalidStatusTransition {
    from: InvoiceStatus,
    to: InvoiceStatus,
  },
}

#[derive(Debug, Error)]
pub enum InvoiceError {
  #[error("Validation error: {0}")]
  Validation(#[from] ValueObjectError),

  #[error("Entity error: {0}")]
  Entity(#[from] InvoiceEntityError),

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

  #[error("Template not found: {0}")]
  TemplateNotFound(Uuid),

  #[error("Template name '{0}' already exists")]
  TemplateNameAlreadyExists(String),

  #[error("Cannot delete invoice: {0}")]
  CannotDeleteInvoice(String),

  #[error("PDF generation failed: {0}")]
  PdfGenerationFailed(String),

  #[error("Repository error: {0}")]
  Repository(String),

  #[error("Database error: {0}")]
  Database(#[from] sqlx::Error),

  #[error("Internal error: {0}")]
  Internal(String),
}
