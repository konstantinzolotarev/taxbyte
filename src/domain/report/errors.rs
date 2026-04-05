use thiserror::Error;

use crate::domain::auth::errors::RepositoryError;

#[derive(Debug, Error)]
pub enum ReportError {
  #[error("Report not found")]
  NotFound,

  #[error("Transaction not found")]
  TransactionNotFound,

  #[error("Received invoice not found")]
  ReceivedInvoiceNotFound,

  #[error("A report for this month already exists")]
  DuplicateReport,

  #[error("Transaction is already matched")]
  AlreadyMatched,

  #[error("Transaction is not matched")]
  NotMatched,

  #[error("Cannot match: direction mismatch")]
  DirectionMismatch,

  #[error("Report must be in draft status")]
  NotDraft,

  #[error("No matched transactions to generate")]
  NoMatchedTransactions,

  #[error("CSV parse error: {0}")]
  CsvParse(String),

  #[error("Validation error: {0}")]
  Validation(String),

  #[error("Cloud storage error: {0}")]
  CloudStorage(String),

  #[error("File error: {0}")]
  FileError(String),

  #[error("Repository error: {0}")]
  Repository(#[from] RepositoryError),
}

impl From<sqlx::Error> for ReportError {
  fn from(error: sqlx::Error) -> Self {
    ReportError::Repository(RepositoryError::from(error))
  }
}
