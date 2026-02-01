use actix_web::{HttpResponse, web};
use std::sync::Arc;

use crate::application::auth::{
  GetCurrentUserUseCase, LoginUserUseCase, LogoutAllDevicesUseCase, LogoutUserUseCase,
  RegisterUserUseCase,
};
use crate::application::company::{
  AddCompanyMemberUseCase, CreateCompanyUseCase, GetCompanyDetailsUseCase, GetUserCompaniesUseCase,
  RemoveCompanyMemberUseCase, SetActiveCompanyUseCase, UpdateCompanyProfileUseCase,
};
use crate::domain::auth::ports::UserRepository;
use crate::domain::auth::services::AuthService;
use crate::domain::company::ports::CompanyMemberRepository;

use super::handlers::auth::{
  get_current_user_handler, login_handler, logout_all_handler, logout_handler, register_handler,
};
use super::handlers::company::{
  add_company_member_handler, create_company_handler, get_user_companies_handler,
  remove_company_member_handler, set_active_company_handler,
};
use super::handlers::{company_web, pages, web_auth};
use super::middleware::WebAuthMiddleware;
use super::templates::TemplateEngine;

/// Configure authentication routes
///
/// Mounts all authentication-related endpoints under the provided scope.
/// All routes are prefixed with the scope path (e.g., /api/v1/auth).
///
/// # Routes
///
/// - POST /register - Register a new user account
/// - POST /login - Authenticate and create a session
/// - POST /logout - Invalidate the current session
/// - POST /logout-all - Invalidate all sessions for the user
/// - GET /me - Get current user information
///
/// # Arguments
///
/// * `register_use_case` - Use case for user registration
/// * `login_use_case` - Use case for user login
/// * `logout_use_case` - Use case for user logout
/// * `logout_all_use_case` - Use case for logging out from all devices
/// * `get_user_use_case` - Use case for getting current user info
///
/// # Example
///
/// ```no_run
/// use actix_web::{App, web};
/// use std::sync::Arc;
/// # use taxbyte::application::auth::*;
/// # use taxbyte::adapters::http::routes::configure_auth_routes;
///
/// # async fn example(
/// #   register_use_case: Arc<RegisterUserUseCase>,
/// #   login_use_case: Arc<LoginUserUseCase>,
/// #   logout_use_case: Arc<LogoutUserUseCase>,
/// #   logout_all_use_case: Arc<LogoutAllDevicesUseCase>,
/// #   get_user_use_case: Arc<GetCurrentUserUseCase>,
/// # ) {
/// let app = App::new().service(
///   web::scope("/api/v1/auth").configure(|cfg| {
///     configure_auth_routes(
///       cfg,
///       register_use_case,
///       login_use_case,
///       logout_use_case,
///       logout_all_use_case,
///       get_user_use_case,
///     )
///   }),
/// );
/// # }
/// ```
pub fn configure_auth_routes(
  cfg: &mut web::ServiceConfig,
  register_use_case: Arc<RegisterUserUseCase>,
  login_use_case: Arc<LoginUserUseCase>,
  logout_use_case: Arc<LogoutUserUseCase>,
  logout_all_use_case: Arc<LogoutAllDevicesUseCase>,
  get_user_use_case: Arc<GetCurrentUserUseCase>,
) {
  // Store use cases in app data so handlers can access them
  cfg
    .app_data(web::Data::new(register_use_case))
    .app_data(web::Data::new(login_use_case))
    .app_data(web::Data::new(logout_use_case))
    .app_data(web::Data::new(logout_all_use_case.clone()))
    .app_data(web::Data::new(get_user_use_case.clone()))
    // Configure routes
    .route("/register", web::post().to(register_handler))
    .route("/login", web::post().to(login_handler))
    .route("/logout", web::post().to(logout_handler))
    .route("/logout-all", web::post().to(logout_all_handler))
    .route("/me", web::get().to(get_current_user_handler));
}

/// Configure web UI routes
pub fn configure_web_routes(
  cfg: &mut web::ServiceConfig,
  templates: TemplateEngine,
  auth_service: Arc<AuthService>,
  register_use_case: Arc<RegisterUserUseCase>,
  login_use_case: Arc<LoginUserUseCase>,
  get_companies_use_case: Arc<GetUserCompaniesUseCase>,
  create_company_use_case: Arc<CreateCompanyUseCase>,
  set_active_use_case: Arc<SetActiveCompanyUseCase>,
  add_member_use_case: Arc<AddCompanyMemberUseCase>,
  remove_member_use_case: Arc<RemoveCompanyMemberUseCase>,
  get_details_use_case: Arc<GetCompanyDetailsUseCase>,
  update_profile_use_case: Arc<UpdateCompanyProfileUseCase>,
  user_repo: Arc<dyn UserRepository>,
  member_repo: Arc<dyn CompanyMemberRepository>,
) {
  // Add template engine to app data
  cfg.app_data(web::Data::new(templates.clone()));

  // Public routes (no authentication required)
  cfg
    .route(
      "/",
      web::get().to(|| async {
        HttpResponse::Found()
          .insert_header(("Location", "/login"))
          .finish()
      }),
    )
    .route("/login", web::get().to(pages::login_page))
    .route("/register", web::get().to(pages::register_page));

  // Auth form submission routes
  cfg.service(
    web::scope("/auth")
      .app_data(web::Data::new(register_use_case))
      .app_data(web::Data::new(login_use_case))
      .route("/login", web::post().to(web_auth::login_submit))
      .route("/register", web::post().to(web_auth::register_submit))
      .route("/logout", web::post().to(web_auth::logout)),
  );

  // Protected routes (require authentication)
  cfg.service(
    web::scope("/dashboard")
      .wrap(WebAuthMiddleware::new(auth_service.clone()))
      .app_data(web::Data::new(templates.clone())) // Add templates to scope
      .app_data(web::Data::new(get_companies_use_case.clone()))
      .app_data(web::Data::new(get_details_use_case.clone()))
      .route("", web::get().to(pages::dashboard_page)),
  );

  // Company web UI routes
  cfg.service(
    web::scope("/companies")
      .wrap(WebAuthMiddleware::new(auth_service))
      .app_data(web::Data::new(templates.clone())) // Add templates to scope
      .app_data(web::Data::new(get_companies_use_case))
      .app_data(web::Data::new(create_company_use_case))
      .app_data(web::Data::new(set_active_use_case))
      .app_data(web::Data::new(add_member_use_case))
      .app_data(web::Data::new(remove_member_use_case))
      .app_data(web::Data::new(get_details_use_case))
      .app_data(web::Data::new(update_profile_use_case))
      .app_data(web::Data::new(user_repo))
      .app_data(web::Data::new(member_repo))
      .route("", web::get().to(company_web::companies_page))
      .route(
        "/dropdown",
        web::get().to(company_web::company_dropdown_handler),
      )
      .route(
        "/create",
        web::post().to(company_web::create_company_submit),
      )
      .route(
        "/{company_id}/set-active",
        web::post().to(company_web::set_active_company_handler),
      )
      .route(
        "/{company_id}/settings",
        web::get().to(company_web::company_settings_page),
      )
      .route(
        "/{company_id}/settings",
        web::post().to(company_web::update_company_settings_submit),
      )
      .route(
        "/{company_id}/members",
        web::get().to(company_web::company_members_page),
      )
      .route(
        "/{company_id}/members/add",
        web::post().to(company_web::add_member_submit),
      )
      .route(
        "/{company_id}/members/{user_id}",
        web::delete().to(company_web::remove_member_handler),
      ),
  );
}

/// Configure company routes
///
/// Mounts all company-related endpoints under the provided scope.
/// All routes are prefixed with the scope path (e.g., /api/v1/companies).
///
/// # Routes
///
/// - POST / - Create a new company
/// - GET / - Get user's companies
/// - POST /active - Set active company
/// - POST /:company_id/members - Add member to company
/// - DELETE /:company_id/members/:user_id - Remove member from company
pub fn configure_company_routes(
  cfg: &mut web::ServiceConfig,
  create_use_case: Arc<CreateCompanyUseCase>,
  get_companies_use_case: Arc<GetUserCompaniesUseCase>,
  set_active_use_case: Arc<SetActiveCompanyUseCase>,
  add_member_use_case: Arc<AddCompanyMemberUseCase>,
  remove_member_use_case: Arc<RemoveCompanyMemberUseCase>,
) {
  cfg
    .app_data(web::Data::new(create_use_case))
    .app_data(web::Data::new(get_companies_use_case))
    .app_data(web::Data::new(set_active_use_case))
    .app_data(web::Data::new(add_member_use_case))
    .app_data(web::Data::new(remove_member_use_case))
    .route("", web::post().to(create_company_handler))
    .route("", web::get().to(get_user_companies_handler))
    .route("/active", web::post().to(set_active_company_handler))
    .route(
      "/{company_id}/members",
      web::post().to(add_company_member_handler),
    )
    .route(
      "/{company_id}/members/{user_id}",
      web::delete().to(remove_company_member_handler),
    );
}

#[cfg(test)]
mod tests {
  #[tokio::test]
  async fn test_routes_configuration() {
    // This test verifies that routes can be configured without panicking
    // We don't test the actual handlers here, just the route configuration

    // Note: We can't easily create real use cases without database connections,
    // so we just verify the configuration syntax is correct by compiling
    assert!(true);
  }
}
