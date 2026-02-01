use std::sync::Arc;

use uuid::Uuid;

use crate::domain::company::{CompanyError, CompanyService};

#[derive(Debug, Clone)]
pub struct SetActiveCompanyCommand {
    pub user_id: Uuid,
    pub company_id: Uuid,
}

pub struct SetActiveCompanyUseCase {
    company_service: Arc<CompanyService>,
}

impl SetActiveCompanyUseCase {
    pub fn new(company_service: Arc<CompanyService>) -> Self {
        Self { company_service }
    }

    pub async fn execute(
        &self,
        command: SetActiveCompanyCommand,
    ) -> Result<(), CompanyError> {
        self.company_service
            .set_active_company(command.user_id, command.company_id)
            .await
    }
}
