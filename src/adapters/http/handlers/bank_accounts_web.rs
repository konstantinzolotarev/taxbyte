use actix_web::{HttpRequest, HttpResponse, web};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::adapters::http::{
  errors::ApiError,
  handlers::{get_company_context, get_user},
  templates::TemplateEngine,
};
use crate::application::company::{
  ArchiveBankAccountCommand, ArchiveBankAccountUseCase, CreateBankAccountCommand,
  CreateBankAccountUseCase, GetBankAccountsCommand, GetBankAccountsUseCase,
  SetActiveBankAccountCommand, SetActiveBankAccountUseCase, UpdateBankAccountCommand,
  UpdateBankAccountUseCase,
};
use crate::domain::company::ports::ActiveBankAccountRepository;

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

/// GET /c/:company_id/bank-accounts - Bank accounts page
pub async fn bank_accounts_page(
  req: HttpRequest,
  templates: web::Data<TemplateEngine>,
  get_accounts_use_case: web::Data<Arc<GetBankAccountsUseCase>>,
  active_bank_account_repo: web::Data<Arc<dyn ActiveBankAccountRepository>>,
  get_companies_use_case: web::Data<Arc<crate::application::company::GetUserCompaniesUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let company_context = get_company_context(&req)?;
  let company_id = company_context.company_id;

  // Fetch user's companies for the navbar selector
  let companies_response = get_companies_use_case
    .execute(crate::application::company::GetUserCompaniesCommand { user_id: user.id })
    .await?;

  // Find current company from the list for the selector
  let active_company = companies_response
    .companies
    .iter()
    .find(|c| c.company_id == company_id)
    .map(|c| {
      serde_json::json!({
        "company_id": c.company_id,
        "name": c.name,
        "role": c.role,
      })
    });

  // Get accounts
  let response = get_accounts_use_case
    .execute(GetBankAccountsCommand {
      company_id,
      requester_id: user.id,
      include_archived: false,
    })
    .await?;

  // Get active bank account ID
  let active_account_id = active_bank_account_repo
    .get_active(company_id)
    .await
    .ok()
    .flatten();

  let mut context = tera::Context::new();
  context.insert("user", &user);
  context.insert("companies", &companies_response.companies);
  context.insert("active_company", &active_company);
  context.insert("company_id", &company_id.to_string());
  context.insert("current_page", "bank-accounts");
  context.insert("accounts", &response.accounts);
  context.insert("active_account_id", &active_account_id);

  let html = templates
    .render("pages/bank_accounts.html.tera", &context)
    .map_err(|e| ApiError::Internal(format!("Template error: {}", e)))?;

  Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

/// POST /c/:company_id/bank-accounts/create - Create bank account form submission
pub async fn create_bank_account_submit(
  req: HttpRequest,
  form: web::Form<CreateBankAccountFormData>,
  templates: web::Data<TemplateEngine>,
  create_use_case: web::Data<Arc<CreateBankAccountUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let company_context = get_company_context(&req)?;
  let company_id = company_context.company_id;

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
    company_id,
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
          .insert_header(("HX-Redirect", format!("/c/{}/bank-accounts", company_id)))
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

/// POST /c/:company_id/bank-accounts/:account_id/update - Update bank account
pub async fn update_bank_account_submit(
  path: web::Path<(Uuid, Uuid)>,
  req: HttpRequest,
  form: web::Form<UpdateBankAccountFormData>,
  update_use_case: web::Data<Arc<UpdateBankAccountUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let company_context = get_company_context(&req)?;

  let (company_id, account_id) = path.into_inner();

  // Verify the company_id from URL matches the context
  if company_id != company_context.company_id {
    return Err(ApiError::Auth(
      crate::adapters::http::errors::AuthErrorKind::Forbidden,
    ));
  }

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
      .insert_header(("HX-Redirect", format!("/c/{}/bank-accounts", company_id)))
      .finish(),
  )
}

/// DELETE /c/:company_id/bank-accounts/:account_id/archive - Archive bank account
pub async fn archive_bank_account_handler(
  path: web::Path<(Uuid, Uuid)>,
  req: HttpRequest,
  archive_use_case: web::Data<Arc<ArchiveBankAccountUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let company_context = get_company_context(&req)?;

  let (company_id, account_id) = path.into_inner();

  // Verify the company_id from URL matches the context
  if company_id != company_context.company_id {
    return Err(ApiError::Auth(
      crate::adapters::http::errors::AuthErrorKind::Forbidden,
    ));
  }

  let command = ArchiveBankAccountCommand {
    company_id,
    requester_id: user.id,
    account_id,
  };

  archive_use_case.execute(command).await?;

  Ok(
    HttpResponse::Ok()
      .insert_header(("HX-Redirect", format!("/c/{}/bank-accounts", company_id)))
      .finish(),
  )
}

/// POST /c/:company_id/bank-accounts/:account_id/set-active - Set active bank account
pub async fn set_active_bank_account_handler(
  path: web::Path<(Uuid, Uuid)>,
  req: HttpRequest,
  set_active_use_case: web::Data<Arc<SetActiveBankAccountUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let company_context = get_company_context(&req)?;

  let (company_id, account_id) = path.into_inner();

  // Verify the company_id from URL matches the context
  if company_id != company_context.company_id {
    return Err(ApiError::Auth(
      crate::adapters::http::errors::AuthErrorKind::Forbidden,
    ));
  }

  let command = SetActiveBankAccountCommand {
    company_id,
    requester_id: user.id,
    account_id,
  };

  set_active_use_case.execute(command).await?;

  Ok(
    HttpResponse::Ok()
      .insert_header(("HX-Redirect", format!("/c/{}/bank-accounts", company_id)))
      .finish(),
  )
}
