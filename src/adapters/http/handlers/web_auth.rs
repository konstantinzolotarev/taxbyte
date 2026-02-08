use actix_web::{HttpRequest, HttpResponse, cookie::Cookie, cookie::SameSite, web};
use serde::Deserialize;
use std::sync::Arc;

use crate::adapters::http::templates::TemplateEngine;
use crate::application::auth::{
  LoginUserCommand, LoginUserUseCase, RegisterUserCommand, RegisterUserUseCase,
};

/// Extract IP address from request
fn extract_ip_address(req: &HttpRequest) -> Option<std::net::IpAddr> {
  req
    .connection_info()
    .realip_remote_addr()
    .and_then(|addr| addr.parse().ok())
}

/// Extract User-Agent from request
fn extract_user_agent(req: &HttpRequest) -> Option<String> {
  req
    .headers()
    .get("user-agent")
    .and_then(|h| h.to_str().ok())
    .map(|s| s.to_string())
}

#[derive(Deserialize)]
pub struct LoginFormData {
  email: String,
  password: String,
  remember_me: Option<String>, // HTML checkbox: "on" or absent
}

#[derive(Deserialize)]
pub struct RegisterFormData {
  email: String,
  password: String,
  full_name: String,
}

/// Handle login form submission
pub async fn login_submit(
  form: web::Form<LoginFormData>,
  use_case: web::Data<Arc<LoginUserUseCase>>,
  templates: web::Data<TemplateEngine>,
  req: HttpRequest,
) -> Result<HttpResponse, actix_web::Error> {
  let ip_address = extract_ip_address(&req);
  let user_agent = extract_user_agent(&req);
  let remember_me = form.remember_me.is_some();

  let command = LoginUserCommand {
    email: form.email.clone(),
    password: form.password.clone(),
    remember_me,
  };

  match use_case.execute(command, ip_address, user_agent).await {
    Ok(response) => {
      tracing::info!("Login successful for user_id={}", response.user_id);
      tracing::debug!(
        "Setting session_token cookie (first 10 chars): {}",
        &response.session_token.chars().take(10).collect::<String>()
      );

      // Create session cookie
      let max_age = if remember_me {
        actix_web::cookie::time::Duration::days(30)
      } else {
        actix_web::cookie::time::Duration::hours(1)
      };

      let cookie = Cookie::build("session_token", response.session_token)
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(max_age)
        .finish();

      // Redirect to dashboard via HTMX
      Ok(
        HttpResponse::Ok()
          .cookie(cookie)
          .insert_header(("HX-Redirect", "/dashboard"))
          .finish(),
      )
    }
    Err(e) => {
      // Render error fragment
      let mut context = tera::Context::new();
      context.insert("error", &e.to_string());
      context.insert("email", &form.email);

      let html = templates
        .render("partials/login_form.html.tera", &context)
        .map_err(actix_web::error::ErrorInternalServerError)?;

      Ok(
        HttpResponse::BadRequest()
          .content_type("text/html")
          .body(html),
      )
    }
  }
}

/// Handle registration form submission
pub async fn register_submit(
  form: web::Form<RegisterFormData>,
  use_case: web::Data<Arc<RegisterUserUseCase>>,
  templates: web::Data<TemplateEngine>,
  _req: HttpRequest,
) -> Result<HttpResponse, actix_web::Error> {
  let command = RegisterUserCommand {
    email: form.email.clone(),
    password: form.password.clone(),
    full_name: form.full_name.clone(),
  };

  match use_case.execute(command).await {
    Ok(response) => {
      // Create session cookie (1 hour for initial registration)
      let cookie = Cookie::build("session_token", response.session_token)
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(actix_web::cookie::time::Duration::hours(1))
        .finish();

      // Redirect to dashboard via HTMX
      Ok(
        HttpResponse::Ok()
          .cookie(cookie)
          .insert_header(("HX-Redirect", "/dashboard"))
          .finish(),
      )
    }
    Err(e) => {
      // Render error fragment
      let mut context = tera::Context::new();
      context.insert("error", &e.to_string());
      context.insert("email", &form.email);
      context.insert("full_name", &form.full_name);

      let html = templates
        .render("partials/register_form.html.tera", &context)
        .map_err(actix_web::error::ErrorInternalServerError)?;

      Ok(
        HttpResponse::BadRequest()
          .content_type("text/html")
          .body(html),
      )
    }
  }
}

/// Handle logout
pub async fn logout(_req: HttpRequest) -> Result<HttpResponse, actix_web::Error> {
  // Clear session cookie
  let cookie = Cookie::build("session_token", "")
    .path("/")
    .http_only(true)
    .same_site(SameSite::Strict)
    .max_age(actix_web::cookie::time::Duration::seconds(0))
    .finish();

  Ok(
    HttpResponse::Found()
      .cookie(cookie)
      .insert_header(("Location", "/login"))
      .finish(),
  )
}
