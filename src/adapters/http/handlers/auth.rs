use actix_web::{HttpRequest, HttpResponse, web};
use std::sync::Arc;
use validator::Validate;

use crate::adapters::http::{
  dtos::{
    CurrentUserResponse, LoginRequest, LoginResponse, LogoutAllResponse, RegisterRequest,
    RegisterResponse, SuccessResponse,
  },
  errors::ApiError,
};
use crate::application::auth::{
  GetCurrentUserResponse as UseCaseCurrentUserResponse, GetCurrentUserUseCase, LoginUserCommand,
  LoginUserResponse as UseCaseLoginResponse, LoginUserUseCase,
  LogoutAllDevicesResponse as UseCaseLogoutAllResponse, LogoutAllDevicesUseCase, LogoutUserUseCase,
  RegisterUserCommand, RegisterUserResponse as UseCaseRegisterResponse, RegisterUserUseCase,
};

/// Extract session token from Authorization header
fn extract_session_token(req: &HttpRequest) -> Result<String, ApiError> {
  req
    .headers()
    .get("Authorization")
    .and_then(|h| h.to_str().ok())
    .and_then(|s| s.strip_prefix("Bearer "))
    .map(|s| s.to_string())
    .ok_or_else(|| ApiError::Validation("Missing or invalid Authorization header".to_string()))
}

/// Extract IP address from the request
fn extract_ip_address(req: &HttpRequest) -> Option<std::net::IpAddr> {
  req.connection_info().realip_remote_addr().and_then(|addr| {
    // Handle both IPv4 and IPv6 formats
    if let Some(ip) = addr.split(':').next() {
      ip.parse().ok()
    } else {
      addr.parse().ok()
    }
  })
}

/// Extract user agent from the request
fn extract_user_agent(req: &HttpRequest) -> Option<String> {
  req
    .headers()
    .get("User-Agent")
    .and_then(|h| h.to_str().ok())
    .map(|s| s.to_string())
}

/// Handler for user registration
///
/// POST /api/auth/register
/// Body: RegisterRequest (JSON)
/// Response: RegisterResponse (JSON) with status 201
pub async fn register_handler(
  request: web::Json<RegisterRequest>,
  use_case: web::Data<Arc<RegisterUserUseCase>>,
) -> Result<HttpResponse, ApiError> {
  // Validate request
  request.validate()?;

  // Create command from request
  let command = RegisterUserCommand {
    email: request.email.clone(),
    password: request.password.clone(),
    full_name: request.full_name.clone(),
  };

  // Execute use case
  let response: UseCaseRegisterResponse = use_case.execute(command).await?;

  // Map to API response
  let api_response = RegisterResponse {
    user_id: response.user_id,
    email: response.email,
    session_token: response.session_token,
    expires_at: response.expires_at,
  };

  Ok(HttpResponse::Created().json(api_response))
}

/// Handler for user login
///
/// POST /api/auth/login
/// Body: LoginRequest (JSON)
/// Response: LoginResponse (JSON) with status 200
pub async fn login_handler(
  request: web::Json<LoginRequest>,
  use_case: web::Data<Arc<LoginUserUseCase>>,
  http_req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
  // Validate request
  request.validate()?;

  // Extract IP address and user agent
  let ip_address = extract_ip_address(&http_req);
  let user_agent = extract_user_agent(&http_req);

  // Create command from request
  let command = LoginUserCommand {
    email: request.email.clone(),
    password: request.password.clone(),
    remember_me: request.remember_me,
  };

  // Execute use case
  let response: UseCaseLoginResponse = use_case.execute(command, ip_address, user_agent).await?;

  // Map to API response
  let api_response = LoginResponse {
    user_id: response.user_id,
    email: response.email,
    session_token: response.session_token,
    expires_at: response.expires_at,
    last_login_at: response.last_login_at,
  };

  Ok(HttpResponse::Ok().json(api_response))
}

/// Handler for user logout
///
/// POST /api/auth/logout
/// Headers: Authorization: Bearer <token>
/// Response: SuccessResponse (JSON) with status 200
pub async fn logout_handler(
  use_case: web::Data<Arc<LogoutUserUseCase>>,
  http_req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
  // Extract session token from Authorization header
  let session_token = extract_session_token(&http_req)?;

  // Execute use case
  use_case.execute(session_token).await?;

  // Return success response
  let response = SuccessResponse {
    message: "Successfully logged out".to_string(),
  };

  Ok(HttpResponse::Ok().json(response))
}

/// Handler for logging out from all devices
///
/// POST /api/auth/logout-all
/// Headers: Authorization: Bearer <token>
/// Response: LogoutAllResponse (JSON) with status 200
pub async fn logout_all_handler(
  use_case: web::Data<Arc<LogoutAllDevicesUseCase>>,
  get_user_use_case: web::Data<Arc<GetCurrentUserUseCase>>,
  http_req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
  // Extract session token from Authorization header
  let session_token = extract_session_token(&http_req)?;

  // First, get the current user to obtain their user_id
  let current_user: UseCaseCurrentUserResponse = get_user_use_case.execute(session_token).await?;

  // Execute logout all use case
  let response: UseCaseLogoutAllResponse = use_case.execute(current_user.user_id).await?;

  // Map to API response
  let api_response = LogoutAllResponse {
    sessions_terminated: response.sessions_terminated,
    message: format!(
      "Successfully logged out from {} device(s)",
      response.sessions_terminated
    ),
  };

  Ok(HttpResponse::Ok().json(api_response))
}

/// Handler for getting current user information
///
/// GET /api/auth/me
/// Headers: Authorization: Bearer <token>
/// Response: CurrentUserResponse (JSON) with status 200
pub async fn get_current_user_handler(
  use_case: web::Data<Arc<GetCurrentUserUseCase>>,
  http_req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
  // Extract session token from Authorization header
  let session_token = extract_session_token(&http_req)?;

  // Execute use case
  let response: UseCaseCurrentUserResponse = use_case.execute(session_token).await?;

  // Map to API response
  let api_response = CurrentUserResponse {
    user_id: response.user_id,
    email: response.email,
    created_at: response.created_at,
    last_login_at: response.last_login_at,
  };

  Ok(HttpResponse::Ok().json(api_response))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_extract_session_token_valid() {
    use actix_web::test::TestRequest;

    let req = TestRequest::default()
      .insert_header(("Authorization", "Bearer test_token_123"))
      .to_http_request();

    let token = extract_session_token(&req).unwrap();
    assert_eq!(token, "test_token_123");
  }

  #[test]
  fn test_extract_session_token_missing() {
    use actix_web::test::TestRequest;

    let req = TestRequest::default().to_http_request();

    let result = extract_session_token(&req);
    assert!(result.is_err());
  }

  #[test]
  fn test_extract_session_token_invalid_format() {
    use actix_web::test::TestRequest;

    let req = TestRequest::default()
      .insert_header(("Authorization", "InvalidFormat token"))
      .to_http_request();

    let result = extract_session_token(&req);
    assert!(result.is_err());
  }
}
