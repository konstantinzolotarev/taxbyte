/// Development-only mock OAuth consent screen
///
/// This handler simulates Google's OAuth consent screen for local development.
/// Only enabled when MOCK_OAUTH=true environment variable is set.
use actix_web::{HttpResponse, web};
use serde::Deserialize;

use crate::adapters::http::errors::ApiError;

#[derive(Debug, Deserialize)]
pub struct MockOAuthQuery {
  pub state: String,
  pub redirect_uri: String,
}

/// GET /dev/mock-oauth - Mock OAuth consent screen
///
/// Shows a fake consent screen that simulates Google's OAuth flow.
/// User can click "Grant Access" to proceed with the mock authorization.
pub async fn mock_oauth_page(query: web::Query<MockOAuthQuery>) -> Result<HttpResponse, ApiError> {
  // Check if mock OAuth is enabled
  if std::env::var("MOCK_OAUTH").unwrap_or_default() != "true" {
    return Ok(HttpResponse::NotFound().content_type("text/html").body(
      "<h1>404 Not Found</h1><p>Mock OAuth is not enabled. Set MOCK_OAUTH=true to enable.</p>",
    ));
  }

  let html = include_str!("../../../../templates/dev/mock_oauth.html")
    .replace("{{REDIRECT_URI}}", &query.redirect_uri)
    .replace(
      "{{AUTH_CODE}}",
      &format!("mock-auth-code-{}", uuid::Uuid::new_v4()),
    )
    .replace("{{STATE}}", &query.state);

  Ok(HttpResponse::Ok().content_type("text/html").body(html))
}
