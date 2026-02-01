use std::sync::Arc;

use uuid::Uuid;

use crate::domain::company::{CompanyError, CompanyService};

#[derive(Debug, Clone)]
pub struct RemoveCompanyMemberCommand {
    pub company_id: Uuid,
    pub requester_id: Uuid,
    pub member_id: Uuid,
}

pub struct RemoveCompanyMemberUseCase {
    company_service: Arc<CompanyService>,
}

impl RemoveCompanyMemberUseCase {
    pub fn new(company_service: Arc<CompanyService>) -> Self {
        Self { company_service }
    }

    pub async fn execute(
        &self,
        command: RemoveCompanyMemberCommand,
    ) -> Result<(), CompanyError> {
        self.company_service
            .remove_member(command.company_id, command.requester_id, command.member_id)
            .await
    }
}
