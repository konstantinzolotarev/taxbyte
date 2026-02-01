use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};

use crate::adapters::http::templates::TemplateEngine;
use crate::domain::auth::entities::User;

/// Render login page
pub async fn login_page(templates: web::Data<TemplateEngine>) -> Result<HttpResponse, actix_web::Error> {
  let mut context = tera::Context::new();
  context.insert("title", "Login");

  let html = templates
    .render("pages/login.html.tera", &context)
    .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

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
    .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

  Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

/// Render dashboard page (authenticated)
pub async fn dashboard_page(
  templates: web::Data<TemplateEngine>,
  req: HttpRequest,
) -> Result<HttpResponse, actix_web::Error> {
  // Get user from request extensions (set by auth middleware)
  let user = req
    .extensions()
    .get::<User>()
    .ok_or_else(|| actix_web::error::ErrorUnauthorized("Not authenticated"))?
    .clone();

  let mut context = tera::Context::new();
  context.insert("title", "Dashboard");
  context.insert("user", &serde_json::json!({
      "email": user.email.as_str(),
      "full_name": user.full_name,
      "created_at": user.created_at.to_rfc3339(),
  }));

  let html = templates
    .render("pages/dashboard.html.tera", &context)
    .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

  Ok(HttpResponse::Ok().content_type("text/html").body(html))
}
