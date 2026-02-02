use actix_web::{HttpRequest, HttpResponse, web};
use std::sync::Arc;

use crate::adapters::http::handlers::{get_company_context, get_user};
use crate::adapters::http::templates::TemplateEngine;
use crate::application::company::{
  GetCompanyDetailsCommand, GetCompanyDetailsUseCase, GetUserCompaniesCommand,
  GetUserCompaniesUseCase,
};

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
  // Get user and company context from request extensions (set by middleware)
  let user = get_user(&req).map_err(actix_web::error::ErrorUnauthorized)?;
  let company_context = get_company_context(&req).map_err(actix_web::error::ErrorUnauthorized)?;
  let company_id = company_context.company_id;

  // Fetch user's companies for navbar selector
  let companies_response = get_companies_use_case
    .execute(GetUserCompaniesCommand { user_id: user.id })
    .await
    .map_err(|e| {
      actix_web::error::ErrorInternalServerError(format!("Failed to fetch companies: {}", e))
    })?;

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

  // Fetch company details
  let active_company_details = match get_details_use_case
    .execute(GetCompanyDetailsCommand {
      company_id,
      requester_id: user.id,
    })
    .await
  {
    Ok(details) => Some(details),
    Err(e) => {
      tracing::warn!("Failed to fetch company details: {:?}", e);
      None
    }
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
  context.insert("companies", &companies_response.companies);
  context.insert("has_companies", &!companies_response.companies.is_empty());
  context.insert("active_company", &active_company);
  context.insert("active_company_details", &active_company_details);
  context.insert("company_id", &company_id.to_string());
  context.insert("current_page", "dashboard");

  let html = templates
    .render("pages/dashboard.html.tera", &context)
    .map_err(actix_web::error::ErrorInternalServerError)?;

  Ok(HttpResponse::Ok().content_type("text/html").body(html))
}
