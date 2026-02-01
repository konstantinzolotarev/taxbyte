use std::sync::Arc;

use uuid::Uuid;

use crate::domain::{
  auth::value_objects::Email,
  company::{CompanyError, CompanyRole, CompanyService},
};

#[derive(Debug, Clone)]
pub struct AddCompanyMemberCommand {
  pub company_id: Uuid,
  pub requester_id: Uuid,
  pub member_email: String,
  pub role: String,
}

pub struct AddCompanyMemberUseCase {
  company_service: Arc<CompanyService>,
}

impl AddCompanyMemberUseCase {
  pub fn new(company_service: Arc<CompanyService>) -> Self {
    Self { company_service }
  }

  pub async fn execute(&self, command: AddCompanyMemberCommand) -> Result<(), CompanyError> {
    let email = Email::new(command.member_email)?;
    let role = CompanyRole::from_str(&command.role)?;

    self
      .company_service
      .add_member(command.company_id, command.requester_id, email, role)
      .await?;

    Ok(())
  }
}
