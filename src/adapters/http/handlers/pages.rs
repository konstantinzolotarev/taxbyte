use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use std::sync::Arc;

use crate::adapters::http::templates::TemplateEngine;
use crate::application::company::{
  GetCompanyDetailsCommand, GetCompanyDetailsUseCase, GetUserCompaniesCommand,
  GetUserCompaniesUseCase,
};
use crate::domain::auth::entities::User;

/// Render login page
pub async fn login_page(
  templates: web::Data<TemplateEngine>,
) -> Result<HttpResponse, actix_web::Error> {
  let mut context = tera::Context::new();
  context.insert("title", "Login");

  let html = templates
    .render("pages/login.html.tera", &context)
    .map_err(actix_web::error::ErrorInternalServerError)?;

  Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

/// Render registration page
pub async fn register_page(
  templates: web::Data<TemplateEngine>,
) -> Result<HttpResponse, actix_web::Error> {
  let mut context = tera::Context::new();
  context.insert("title", "Register");

  let html = templates
    .render("pages/register.html.tera", &context)
    .map_err(actix_web::error::ErrorInternalServerError)?;

  Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

/// Render dashboard page (authenticated)
pub async fn dashboard_page(
  templates: web::Data<TemplateEngine>,
  req: HttpRequest,
  get_companies_use_case: web::Data<Arc<GetUserCompaniesUseCase>>,
  get_details_use_case: web::Data<Arc<GetCompanyDetailsUseCase>>,
) -> Result<HttpResponse, actix_web::Error> {
  // Get user from request extensions (set by auth middleware)
  let user = req
    .extensions()
    .get::<User>()
    .ok_or_else(|| actix_web::error::ErrorUnauthorized("Not authenticated"))?
    .clone();

  // Fetch user's companies (includes is_active flag)
  let companies_response = get_companies_use_case
    .execute(GetUserCompaniesCommand { user_id: user.id })
    .await
    .map_err(|e| {
      actix_web::error::ErrorInternalServerError(format!("Failed to fetch companies: {}", e))
    })?;

  let has_companies = !companies_response.companies.is_empty();
  let active_company = companies_response
    .companies
    .iter()
    .find(|c| c.is_active)
    .map(|c| serde_json::json!({ "id": c.company_id, "name": c.name }));

  // Fetch active company details if one is selected
  let active_company_details = if let Some(active) = &active_company {
    let company_id: uuid::Uuid = active["id"]
      .as_str()
      .ok_or_else(|| actix_web::error::ErrorInternalServerError("Invalid company ID"))?
      .parse()
      .map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to parse company ID: {}", e))
      })?;

    match get_details_use_case
      .execute(GetCompanyDetailsCommand {
        company_id,
        requester_id: user.id,
      })
      .await
    {
      Ok(details) => Some(details),
      Err(e) => {
        tracing::warn!("Failed to fetch active company details: {:?}", e);
        None
      }
    }
  } else {
    None
  };

  let mut context = tera::Context::new();
  context.insert("title", "Dashboard");
  context.insert(
    "user",
    &serde_json::json!({
        "email": user.email.as_str(),
        "full_name": user.full_name,
        "created_at": user.created_at.to_rfc3339(),
    }),
  );
  context.insert("has_companies", &has_companies);
  context.insert("companies", &companies_response.companies);
  context.insert("active_company", &active_company);
  context.insert("active_company_details", &active_company_details);

  let html = templates
    .render("pages/dashboard.html.tera", &context)
    .map_err(actix_web::error::ErrorInternalServerError)?;

  Ok(HttpResponse::Ok().content_type("text/html").body(html))
}
