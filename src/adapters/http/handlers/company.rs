use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use std::sync::Arc;
use uuid::Uuid;
use validator::Validate;

use crate::{
  adapters::http::{
    dtos::{
      AddCompanyMemberRequest, CompanyListItemDto, CompanyListResponse, CreateCompanyRequest,
      CreateCompanyResponse as DtoCreateCompanyResponse, SetActiveCompanyRequest,
    },
    errors::ApiError,
  },
  application::company::*,
};

/// Helper to extract authenticated user ID from request
fn get_user_id(req: &HttpRequest) -> Uuid {
  let extensions = req.extensions();
  let user = extensions
    .get::<crate::domain::auth::entities::User>()
    .expect("User not found in request extensions - ensure AuthMiddleware is applied");
  user.id
}

/// Create new company
/// POST /api/v1/companies
pub async fn create_company_handler(
  request: web::Json<CreateCompanyRequest>,
  use_case: web::Data<Arc<CreateCompanyUseCase>>,
  http_req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
  request.validate()?;

  let user_id = get_user_id(&http_req);

  let command = CreateCompanyCommand {
    name: request.name.clone(),
    owner_id: user_id,
  };

  let response = use_case.execute(command).await?;

  Ok(HttpResponse::Created().json(DtoCreateCompanyResponse {
    company_id: response.company_id,
    name: response.name,
    created_at: response.created_at,
  }))
}

/// Get user's companies
/// GET /api/v1/companies
pub async fn get_user_companies_handler(
  use_case: web::Data<Arc<GetUserCompaniesUseCase>>,
  http_req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
  let user_id = get_user_id(&http_req);

  let command = GetUserCompaniesCommand { user_id };

  let response = use_case.execute(command).await?;

  let companies: Vec<CompanyListItemDto> = response
    .companies
    .into_iter()
    .map(|c| CompanyListItemDto {
      company_id: c.company_id,
      name: c.name,
      role: c.role,
      is_active: c.is_active,
    })
    .collect();

  Ok(HttpResponse::Ok().json(CompanyListResponse { companies }))
}

/// Set active company
/// POST /api/v1/companies/active
pub async fn set_active_company_handler(
  request: web::Json<SetActiveCompanyRequest>,
  use_case: web::Data<Arc<SetActiveCompanyUseCase>>,
  http_req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
  let user_id = get_user_id(&http_req);

  let command = SetActiveCompanyCommand {
    user_id,
    company_id: request.company_id,
  };

  use_case.execute(command).await?;

  Ok(HttpResponse::Ok().json(serde_json::json!({
      "message": "Active company updated",
      "company_id": request.company_id
  })))
}

/// Add member to company
/// POST /api/v1/companies/:company_id/members
pub async fn add_company_member_handler(
  company_id: web::Path<Uuid>,
  request: web::Json<AddCompanyMemberRequest>,
  use_case: web::Data<Arc<AddCompanyMemberUseCase>>,
  http_req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
  request.validate()?;

  let user_id = get_user_id(&http_req);

  let command = AddCompanyMemberCommand {
    company_id: *company_id,
    requester_id: user_id,
    member_email: request.email.clone(),
    role: request.role.clone(),
  };

  use_case.execute(command).await?;

  Ok(HttpResponse::Created().json(serde_json::json!({
      "message": "Member added successfully"
  })))
}

/// Remove member from company
/// DELETE /api/v1/companies/:company_id/members/:user_id
pub async fn remove_company_member_handler(
  path: web::Path<(Uuid, Uuid)>,
  use_case: web::Data<Arc<RemoveCompanyMemberUseCase>>,
  http_req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
  let (company_id, member_id) = path.into_inner();
  let user_id = get_user_id(&http_req);

  let command = RemoveCompanyMemberCommand {
    company_id,
    requester_id: user_id,
    member_id,
  };

  use_case.execute(command).await?;

  Ok(HttpResponse::Ok().json(serde_json::json!({
      "message": "Member removed successfully"
  })))
}
