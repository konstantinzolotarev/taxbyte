use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::adapters::http::errors::ApiError;
use crate::adapters::http::templates::TemplateEngine;
use crate::application::company::{
  ArchiveBankAccountCommand, ArchiveBankAccountUseCase, CreateBankAccountCommand,
  CreateBankAccountUseCase, GetBankAccountsCommand, GetBankAccountsUseCase,
  SetActiveBankAccountCommand, SetActiveBankAccountUseCase, UpdateBankAccountCommand,
  UpdateBankAccountUseCase,
};
use crate::domain::auth::entities::User;
use crate::domain::company::ports::ActiveBankAccountRepository;

/// Helper function to extract authenticated user from request
fn get_user(req: &HttpRequest) -> Result<User, ApiError> {
  match req.extensions().get::<User>() {
    Some(user) => Ok(user.clone()),
    None => Err(ApiError::Auth(
      crate::adapters::http::errors::AuthErrorKind::InvalidSession,
    )),
  }
}

#[derive(Deserialize)]
pub struct CreateBankAccountFormData {
  pub name: String,
  pub iban: String,
  pub bank_details: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateBankAccountFormData {
  pub name: String,
  pub iban: String,
  pub bank_details: Option<String>,
}

/// GET /companies/:company_id/bank-accounts - Bank accounts page
pub async fn bank_accounts_page(
  company_id: web::Path<Uuid>,
  req: HttpRequest,
  templates: web::Data<TemplateEngine>,
  get_accounts_use_case: web::Data<Arc<GetBankAccountsUseCase>>,
  active_bank_account_repo: web::Data<Arc<dyn ActiveBankAccountRepository>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;

  // Get accounts
  let response = get_accounts_use_case
    .execute(GetBankAccountsCommand {
      company_id: *company_id,
      requester_id: user.id,
      include_archived: false,
    })
    .await?;

  // Get active bank account ID
  let active_account_id = active_bank_account_repo
    .get_active(*company_id)
    .await
    .ok()
    .flatten();

  let mut context = tera::Context::new();
  context.insert("user", &user);
  context.insert("company_id", &company_id.to_string());
  context.insert("accounts", &response.accounts);
  context.insert("active_account_id", &active_account_id);

  let html = templates
    .render("pages/bank_accounts.html.tera", &context)
    .map_err(|e| ApiError::Internal(format!("Template error: {}", e)))?;

  Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

/// POST /companies/:company_id/bank-accounts/create - Create bank account form submission
pub async fn create_bank_account_submit(
  company_id: web::Path<Uuid>,
  req: HttpRequest,
  form: web::Form<CreateBankAccountFormData>,
  templates: web::Data<TemplateEngine>,
  create_use_case: web::Data<Arc<CreateBankAccountUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;

  // Validate
  let name = form.name.trim();
  let iban = form.iban.trim();

  if name.is_empty() || iban.is_empty() {
    let mut context = tera::Context::new();
    context.insert("company_id", &company_id.to_string());
    context.insert("error", "Name and IBAN are required");
    context.insert("name", &form.name);
    context.insert("iban", &form.iban);
    context.insert("bank_details", &form.bank_details);

    let html = templates
      .render("partials/create_bank_account_form.html.tera", &context)
      .map_err(|e| ApiError::Internal(format!("Template error: {}", e)))?;

    return Ok(
      HttpResponse::BadRequest()
        .content_type("text/html")
        .body(html),
    );
  }

  // Execute command
  let command = CreateBankAccountCommand {
    company_id: *company_id,
    requester_id: user.id,
    name: name.to_string(),
    iban: iban.to_string(),
    bank_details: form
      .bank_details
      .as_ref()
      .filter(|s| !s.trim().is_empty())
      .map(|s| s.trim().to_string()),
  };

  match create_use_case.execute(command).await {
    Ok(_) => {
      // Redirect back to bank accounts page
      Ok(
        HttpResponse::Ok()
          .insert_header((
            "HX-Redirect",
            format!("/companies/{}/bank-accounts", company_id),
          ))
          .finish(),
      )
    }
    Err(e) => {
      let mut context = tera::Context::new();
      context.insert("company_id", &company_id.to_string());
      context.insert("error", &e.to_string());
      context.insert("name", &form.name);
      context.insert("iban", &form.iban);
      context.insert("bank_details", &form.bank_details);

      let html = templates
        .render("partials/create_bank_account_form.html.tera", &context)
        .map_err(|e| ApiError::Internal(format!("Template error: {}", e)))?;

      Ok(
        HttpResponse::BadRequest()
          .content_type("text/html")
          .body(html),
      )
    }
  }
}

/// POST /companies/:company_id/bank-accounts/:account_id/update - Update bank account
pub async fn update_bank_account_submit(
  path: web::Path<(Uuid, Uuid)>,
  req: HttpRequest,
  form: web::Form<UpdateBankAccountFormData>,
  update_use_case: web::Data<Arc<UpdateBankAccountUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let (company_id, account_id) = path.into_inner();

  let command = UpdateBankAccountCommand {
    company_id,
    requester_id: user.id,
    account_id,
    name: form.name.trim().to_string(),
    iban: form.iban.trim().to_string(),
    bank_details: form
      .bank_details
      .as_ref()
      .filter(|s| !s.trim().is_empty())
      .map(|s| s.trim().to_string()),
  };

  update_use_case.execute(command).await?;

  Ok(
    HttpResponse::Ok()
      .insert_header((
        "HX-Redirect",
        format!("/companies/{}/bank-accounts", company_id),
      ))
      .finish(),
  )
}

/// POST /companies/:company_id/bank-accounts/:account_id/archive - Archive bank account
pub async fn archive_bank_account_handler(
  path: web::Path<(Uuid, Uuid)>,
  req: HttpRequest,
  archive_use_case: web::Data<Arc<ArchiveBankAccountUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let (company_id, account_id) = path.into_inner();

  let command = ArchiveBankAccountCommand {
    company_id,
    requester_id: user.id,
    account_id,
  };

  archive_use_case.execute(command).await?;

  Ok(
    HttpResponse::Ok()
      .insert_header((
        "HX-Redirect",
        format!("/companies/{}/bank-accounts", company_id),
      ))
      .finish(),
  )
}

/// POST /companies/:company_id/bank-accounts/:account_id/set-active - Set active bank account
pub async fn set_active_bank_account_handler(
  path: web::Path<(Uuid, Uuid)>,
  req: HttpRequest,
  set_active_use_case: web::Data<Arc<SetActiveBankAccountUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let (company_id, account_id) = path.into_inner();

  let command = SetActiveBankAccountCommand {
    company_id,
    requester_id: user.id,
    account_id,
  };

  set_active_use_case.execute(command).await?;

  Ok(
    HttpResponse::Ok()
      .insert_header((
        "HX-Redirect",
        format!("/companies/{}/bank-accounts", company_id),
      ))
      .finish(),
  )
}
