use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use serde::Deserialize;
use std::sync::Arc;

use crate::adapters::http::errors::{ApiError, AuthErrorKind};
use crate::application::company::{CompleteOAuthCommand, ConnectGoogleDriveUseCase};
use crate::domain::auth::entities::User;

#[derive(Debug, Deserialize)]
pub struct OAuthCallbackQuery {
  pub code: String,
  pub state: String,
  #[serde(default)]
  pub error: Option<String>,
}

/// OAuth callback handler - receives authorization code from Google
///
/// This endpoint is called by Google after the user grants/denies permission.
/// The state parameter contains the company_id (we encode it during initiation).
pub async fn oauth_callback(
  query: web::Query<OAuthCallbackQuery>,
  req: HttpRequest,
  use_case: web::Data<Arc<ConnectGoogleDriveUseCase>>,
) -> Result<HttpResponse, ApiError> {
  // Get authenticated user from session
  let user = req
    .extensions()
    .get::<User>()
    .cloned()
    .ok_or(ApiError::Auth(AuthErrorKind::InvalidSession))?;

  // Check for OAuth error
  if let Some(error) = &query.error {
    tracing::warn!("OAuth callback received error: {}", error);
    return Ok(
      HttpResponse::Found()
        .insert_header(("Location", "/companies?error=oauth_denied"))
        .finish(),
    );
  }

  // Parse company_id from state parameter
  // TODO: In production, validate CSRF token from Redis
  // For now, we assume state format is just the company_id as a string
  let company_id = query
    .state
    .parse()
    .map_err(|_| ApiError::Validation("Invalid state parameter".to_string()))?;

  // Complete OAuth flow
  use_case
    .complete_oauth(CompleteOAuthCommand {
      company_id,
      user_id: user.id,
      code: query.code.clone(),
      state: query.state.clone(),
    })
    .await?;

  // Redirect to company settings with success message
  Ok(
    HttpResponse::Found()
      .insert_header((
        "Location",
        format!(
          "/companies/{}/settings?tab=storage&success=drive_connected",
          company_id
        ),
      ))
      .finish(),
  )
}
