pub mod auth;
pub mod bank_accounts;
pub mod bank_accounts_web;
pub mod company;
pub mod company_web;
pub mod customers_web;
pub mod invoices_web;
pub mod pages;
pub mod web_auth;

use crate::{
  adapters::http::{errors::ApiError, middleware::CompanyContext},
  domain::auth::entities::User,
};
use actix_web::{HttpMessage, HttpRequest};

/// Extract authenticated user from request extensions
pub fn get_user(req: &HttpRequest) -> Result<User, ApiError> {
  let user = req.extensions().get::<User>().cloned();

  if user.is_none() {
    tracing::warn!(
      "get_user: User not found in request extensions for path {}",
      req.path()
    );
  }

  user.ok_or(ApiError::Auth(
    crate::adapters::http::errors::AuthErrorKind::InvalidSession,
  ))
}

/// Extract company context from request extensions (set by CompanyContextMiddleware)
pub fn get_company_context(req: &HttpRequest) -> Result<CompanyContext, ApiError> {
  req
    .extensions()
    .get::<CompanyContext>()
    .cloned()
    .ok_or_else(|| ApiError::Internal("Company context not set by middleware".into()))
}
