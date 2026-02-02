use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use validator::Validate;

use crate::{
  adapters::http::errors::ApiError,
  application::company::{
    ArchiveBankAccountCommand, ArchiveBankAccountUseCase, BankAccountDto, CreateBankAccountCommand,
    CreateBankAccountUseCase, GetBankAccountsCommand, GetBankAccountsUseCase,
    SetActiveBankAccountCommand, SetActiveBankAccountUseCase, UpdateBankAccountCommand,
    UpdateBankAccountUseCase,
  },
};

/// Helper to extract authenticated user ID from request
fn get_user_id(req: &HttpRequest) -> Uuid {
  let extensions = req.extensions();
  let user = extensions
    .get::<crate::domain::auth::entities::User>()
    .expect("User not found in request extensions - ensure AuthMiddleware is applied");
  user.id
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateBankAccountRequest {
  #[validate(length(min = 1, max = 100))]
  pub name: String,
  #[validate(length(min = 15, max = 34))]
  pub iban: String,
  #[validate(length(max = 1000))]
  pub bank_details: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateBankAccountResponseDto {
  pub id: Uuid,
  pub company_id: Uuid,
  pub name: String,
  pub iban: String,
  pub bank_details: Option<String>,
  pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateBankAccountRequest {
  #[validate(length(min = 1, max = 100))]
  pub name: String,
  #[validate(length(min = 15, max = 34))]
  pub iban: String,
  #[validate(length(max = 1000))]
  pub bank_details: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BankAccountListResponse {
  pub accounts: Vec<BankAccountDto>,
}

#[derive(Debug, Deserialize)]
pub struct SetActiveBankAccountRequest {
  pub account_id: Uuid,
}

/// Create bank account
/// POST /api/v1/companies/:company_id/bank-accounts
pub async fn create_bank_account_handler(
  company_id: web::Path<Uuid>,
  request: web::Json<CreateBankAccountRequest>,
  use_case: web::Data<Arc<CreateBankAccountUseCase>>,
  http_req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
  request.validate()?;

  let user_id = get_user_id(&http_req);

  let command = CreateBankAccountCommand {
    company_id: *company_id,
    requester_id: user_id,
    name: request.name.clone(),
    iban: request.iban.clone(),
    bank_details: request.bank_details.clone(),
  };

  let response = use_case.execute(command).await?;

  Ok(HttpResponse::Created().json(CreateBankAccountResponseDto {
    id: response.id,
    company_id: response.company_id,
    name: response.name,
    iban: response.iban,
    bank_details: response.bank_details,
    created_at: response.created_at,
  }))
}

/// Get company bank accounts
/// GET /api/v1/companies/:company_id/bank-accounts
pub async fn get_bank_accounts_handler(
  company_id: web::Path<Uuid>,
  query: web::Query<std::collections::HashMap<String, String>>,
  use_case: web::Data<Arc<GetBankAccountsUseCase>>,
  http_req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
  let user_id = get_user_id(&http_req);
  let include_archived = query
    .get("include_archived")
    .and_then(|v| v.parse::<bool>().ok())
    .unwrap_or(false);

  let command = GetBankAccountsCommand {
    company_id: *company_id,
    requester_id: user_id,
    include_archived,
  };

  let response = use_case.execute(command).await?;

  Ok(HttpResponse::Ok().json(BankAccountListResponse {
    accounts: response.accounts,
  }))
}

/// Update bank account
/// PUT /api/v1/companies/:company_id/bank-accounts/:account_id
pub async fn update_bank_account_handler(
  path: web::Path<(Uuid, Uuid)>,
  request: web::Json<UpdateBankAccountRequest>,
  use_case: web::Data<Arc<UpdateBankAccountUseCase>>,
  http_req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
  request.validate()?;

  let (company_id, account_id) = path.into_inner();
  let user_id = get_user_id(&http_req);

  let command = UpdateBankAccountCommand {
    company_id,
    requester_id: user_id,
    account_id,
    name: request.name.clone(),
    iban: request.iban.clone(),
    bank_details: request.bank_details.clone(),
  };

  use_case.execute(command).await?;

  Ok(HttpResponse::Ok().json(serde_json::json!({
    "message": "Bank account updated successfully"
  })))
}

/// Archive bank account
/// DELETE /api/v1/companies/:company_id/bank-accounts/:account_id
pub async fn archive_bank_account_handler(
  path: web::Path<(Uuid, Uuid)>,
  use_case: web::Data<Arc<ArchiveBankAccountUseCase>>,
  http_req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
  let (company_id, account_id) = path.into_inner();
  let user_id = get_user_id(&http_req);

  let command = ArchiveBankAccountCommand {
    company_id,
    requester_id: user_id,
    account_id,
  };

  use_case.execute(command).await?;

  Ok(HttpResponse::Ok().json(serde_json::json!({
    "message": "Bank account archived successfully"
  })))
}

/// Set active bank account
/// POST /api/v1/companies/:company_id/bank-accounts/active
pub async fn set_active_bank_account_handler(
  company_id: web::Path<Uuid>,
  request: web::Json<SetActiveBankAccountRequest>,
  use_case: web::Data<Arc<SetActiveBankAccountUseCase>>,
  http_req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
  let user_id = get_user_id(&http_req);

  let command = SetActiveBankAccountCommand {
    company_id: *company_id,
    requester_id: user_id,
    account_id: request.account_id,
  };

  use_case.execute(command).await?;

  Ok(HttpResponse::Ok().json(serde_json::json!({
    "message": "Active bank account set successfully"
  })))
}
