use std::sync::Arc;

use uuid::Uuid;

use crate::domain::company::{CompanyError, CompanyService};

#[derive(Debug, Clone)]
pub struct ArchiveBankAccountCommand {
  pub company_id: Uuid,
  pub requester_id: Uuid,
  pub account_id: Uuid,
}

pub struct ArchiveBankAccountUseCase {
  company_service: Arc<CompanyService>,
}

impl ArchiveBankAccountUseCase {
  pub fn new(company_service: Arc<CompanyService>) -> Self {
    Self { company_service }
  }

  pub async fn execute(&self, command: ArchiveBankAccountCommand) -> Result<(), CompanyError> {
    self
      .company_service
      .archive_bank_account(command.company_id, command.requester_id, command.account_id)
      .await
  }
}
