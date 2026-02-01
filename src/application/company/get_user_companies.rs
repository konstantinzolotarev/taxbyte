use std::sync::Arc;

use uuid::Uuid;

use crate::domain::company::{
    ActiveCompanyRepository, CompanyError, CompanyMemberRepository, CompanyService,
};

#[derive(Debug, Clone)]
pub struct GetUserCompaniesCommand {
    pub user_id: Uuid,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CompanyListItem {
    pub company_id: Uuid,
    pub name: String,
    pub role: String,
    pub is_active: bool,
}

pub struct GetUserCompaniesResponse {
    pub companies: Vec<CompanyListItem>,
}

pub struct GetUserCompaniesUseCase {
    company_service: Arc<CompanyService>,
    member_repo: Arc<dyn CompanyMemberRepository>,
    active_repo: Arc<dyn ActiveCompanyRepository>,
}

impl GetUserCompaniesUseCase {
    pub fn new(
        company_service: Arc<CompanyService>,
        member_repo: Arc<dyn CompanyMemberRepository>,
        active_repo: Arc<dyn ActiveCompanyRepository>,
    ) -> Self {
        Self {
            company_service,
            member_repo,
            active_repo,
        }
    }

    pub async fn execute(
        &self,
        command: GetUserCompaniesCommand,
    ) -> Result<GetUserCompaniesResponse, CompanyError> {
        let companies = self
            .company_service
            .get_user_companies(command.user_id)
            .await?;
        let active_id = self.active_repo.get_active(command.user_id).await?;

        // Get memberships to find roles
        let memberships = self.member_repo.find_by_user_id(command.user_id).await?;

        let items = companies
            .into_iter()
            .map(|c| {
                let role = memberships
                    .iter()
                    .find(|m| m.company_id == c.id)
                    .map(|m| m.role.as_str().to_string())
                    .unwrap_or_else(|| "member".to_string());

                CompanyListItem {
                    company_id: c.id,
                    name: c.name,
                    role,
                    is_active: Some(c.id) == active_id,
                }
            })
            .collect();

        Ok(GetUserCompaniesResponse { companies: items })
    }
}
