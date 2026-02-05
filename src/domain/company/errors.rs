use thiserror::Error;

use crate::domain::auth::errors::RepositoryError;
use crate::domain::auth::value_objects::ValueObjectError;

#[derive(Debug, Error)]
pub enum CompanyError {
  #[error("Company not found")]
  NotFound,

  #[error("User is not a member of this company")]
  NotMember,

  #[error("User is already a member of this company")]
  AlreadyMember,

  #[error("Insufficient permissions to perform this action")]
  InsufficientPermissions,

  #[error("Cannot remove the last owner from a company")]
  CannotRemoveLastOwner,

  #[error("User not found")]
  UserNotFound,

  #[error("Bank account not found")]
  BankAccountNotFound,

  #[error("Cannot archive the active bank account")]
  CannotArchiveActiveBankAccount,

  #[error("A bank account with this IBAN already exists for this company")]
  DuplicateIban,

  #[error("Repository error: {0}")]
  Repository(#[from] RepositoryError),

  #[error("Validation error: {0}")]
  Validation(#[from] ValidationError),

  #[error("Auth error: {0}")]
  Auth(#[from] crate::domain::auth::errors::AuthError),
}

#[derive(Debug, Error)]
pub enum ValidationError {
  #[error("Company name must be at least {min} characters")]
  CompanyNameTooShort { min: usize },

  #[error("Company name must be at most {max} characters")]
  CompanyNameTooLong { max: usize },

  #[error("Invalid role")]
  InvalidRole,

  #[error("Phone number must be between {min} and {max} characters")]
  PhoneNumberInvalidLength { min: usize, max: usize },

  #[error("Phone number contains invalid characters (only digits, spaces, +, -, (, ) allowed)")]
  PhoneNumberInvalidCharacters,

  #[error("Address field '{field}' must be at most {max} characters")]
  AddressFieldTooLong { field: String, max: usize },

  #[error("Registry code must be at most {max} characters")]
  RegistryCodeTooLong { max: usize },

  #[error("VAT number must be at most {max} characters")]
  VatNumberTooLong { max: usize },

  #[error("Bank account name must be at least {min} characters")]
  BankAccountNameTooShort { min: usize },

  #[error("Bank account name must be at most {max} characters")]
  BankAccountNameTooLong { max: usize },

  #[error("IBAN must be between {min} and {max} characters")]
  IbanInvalidLength { min: usize, max: usize },

  #[error("IBAN has invalid format (must start with 2 letters + 2 digits)")]
  IbanInvalidFormat,

  #[error("IBAN checksum validation failed")]
  IbanInvalidChecksum,

  #[error("Bank details must be at most {max} characters")]
  BankDetailsTooLong { max: usize },

  #[error("Invalid format: {0}")]
  InvalidFormat(String),
}

impl From<CompanyError> for RepositoryError {
  fn from(error: CompanyError) -> Self {
    match error {
      CompanyError::Repository(repo_err) => repo_err,
      CompanyError::NotFound
      | CompanyError::NotMember
      | CompanyError::AlreadyMember
      | CompanyError::InsufficientPermissions
      | CompanyError::CannotRemoveLastOwner
      | CompanyError::UserNotFound
      | CompanyError::BankAccountNotFound
      | CompanyError::CannotArchiveActiveBankAccount
      | CompanyError::DuplicateIban
      | CompanyError::Validation(_)
      | CompanyError::Auth(_) => RepositoryError::QueryFailed(error.to_string()),
    }
  }
}

impl From<ValueObjectError> for CompanyError {
  fn from(error: ValueObjectError) -> Self {
    // ValueObjectError -> AuthError (via From) -> CompanyError
    CompanyError::Auth(crate::domain::auth::errors::AuthError::ValueObject(error))
  }
}

impl From<sqlx::Error> for CompanyError {
  fn from(error: sqlx::Error) -> Self {
    CompanyError::Repository(RepositoryError::from(error))
  }
}
