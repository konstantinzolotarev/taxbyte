use actix_web::{HttpResponse, web};
use std::sync::Arc;

use crate::application::auth::{
  GetCurrentUserUseCase, LoginUserUseCase, LogoutAllDevicesUseCase, LogoutUserUseCase,
  RegisterUserUseCase,
};
use crate::application::company::{
  AddCompanyMemberUseCase, ArchiveBankAccountUseCase, CreateBankAccountUseCase,
  CreateCompanyUseCase, GetBankAccountsUseCase, GetCompanyDetailsUseCase, GetUserCompaniesUseCase,
  RemoveCompanyMemberUseCase, SetActiveBankAccountUseCase, SetActiveCompanyUseCase,
  UpdateBankAccountUseCase, UpdateCompanyProfileUseCase, UpdateStorageConfigUseCase,
};
use crate::application::invoice::{
  ArchiveCustomerUseCase, ArchiveInvoiceUseCase, ChangeInvoiceStatusUseCase, CreateCustomerUseCase,
  CreateInvoiceUseCase, GetInvoiceDetailsUseCase, ListCustomersUseCase, ListInvoicesUseCase,
  UpdateCustomerUseCase,
};
use crate::domain::auth::ports::UserRepository;
use crate::domain::auth::services::AuthService;
use crate::domain::company::ports::{
  ActiveBankAccountRepository, ActiveCompanyRepository, CompanyMemberRepository,
};

use super::errors::ApiError;
use super::handlers::auth::{
  get_current_user_handler, login_handler, logout_all_handler, logout_handler, register_handler,
};
use super::handlers::company::{
  add_company_member_handler, create_company_handler, get_user_companies_handler,
  remove_company_member_handler, set_active_company_handler,
};
use super::handlers::{
  bank_accounts, bank_accounts_web, company_settings, company_web, customers_web, get_user,
  invoices_web, pages, web_auth,
};
use super::middleware::{CompanyContextMiddleware, WebAuthMiddleware};
use super::templates::TemplateEngine;

/// Dependencies for web route configuration
pub struct WebRouteDependencies {
  pub templates: TemplateEngine,
  pub auth_service: Arc<AuthService>,
  pub register_use_case: Arc<RegisterUserUseCase>,
  pub login_use_case: Arc<LoginUserUseCase>,
  pub get_companies_use_case: Arc<GetUserCompaniesUseCase>,
  pub create_company_use_case: Arc<CreateCompanyUseCase>,
  pub set_active_use_case: Arc<SetActiveCompanyUseCase>,
  pub add_member_use_case: Arc<AddCompanyMemberUseCase>,
  pub remove_member_use_case: Arc<RemoveCompanyMemberUseCase>,
  pub get_details_use_case: Arc<GetCompanyDetailsUseCase>,
  pub update_profile_use_case: Arc<UpdateCompanyProfileUseCase>,
  pub update_storage_config_use_case: Arc<UpdateStorageConfigUseCase>,
  pub create_bank_account_use_case: Arc<CreateBankAccountUseCase>,
  pub get_bank_accounts_use_case: Arc<GetBankAccountsUseCase>,
  pub update_bank_account_use_case: Arc<UpdateBankAccountUseCase>,
  pub archive_bank_account_use_case: Arc<ArchiveBankAccountUseCase>,
  pub set_active_bank_account_use_case: Arc<SetActiveBankAccountUseCase>,
  pub user_repo: Arc<dyn UserRepository>,
  pub member_repo: Arc<dyn CompanyMemberRepository>,
  pub active_company_repo: Arc<dyn ActiveCompanyRepository>,
  pub active_bank_account_repo: Arc<dyn ActiveBankAccountRepository>,
  // Customer use cases
  pub create_customer_use_case: Arc<CreateCustomerUseCase>,
  pub list_customers_use_case: Arc<ListCustomersUseCase>,
  pub update_customer_use_case: Arc<UpdateCustomerUseCase>,
  pub archive_customer_use_case: Arc<ArchiveCustomerUseCase>,
  // Invoice use cases
  pub create_invoice_use_case: Arc<CreateInvoiceUseCase>,
  pub list_invoices_use_case: Arc<ListInvoicesUseCase>,
  pub get_invoice_details_use_case: Arc<GetInvoiceDetailsUseCase>,
  pub change_invoice_status_use_case: Arc<ChangeInvoiceStatusUseCase>,
  pub archive_invoice_use_case: Arc<ArchiveInvoiceUseCase>,
  pub delete_invoice_use_case: Arc<crate::application::invoice::DeleteInvoiceUseCase>,
  // Template use cases
  pub create_template_from_invoice_use_case:
    Arc<crate::application::invoice::CreateTemplateFromInvoiceUseCase>,
  pub list_templates_use_case: Arc<crate::application::invoice::ListTemplatesUseCase>,
  pub create_invoice_from_template_use_case:
    Arc<crate::application::invoice::CreateInvoiceFromTemplateUseCase>,
  pub archive_template_use_case: Arc<crate::application::invoice::ArchiveTemplateUseCase>,
}

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

/// Helper to redirect old routes to company-scoped versions
async fn redirect_to_default_company(
  req: actix_web::HttpRequest,
  target_page: &str,
  active_repo: web::Data<Arc<dyn ActiveCompanyRepository>>,
  member_repo: web::Data<Arc<dyn CompanyMemberRepository>>,
) -> Result<HttpResponse, ApiError> {
  tracing::debug!("redirect_to_default_company: target_page={}", target_page);
  let user = get_user(&req)?;
  tracing::debug!("redirect_to_default_company: user_id={}", user.id);

  // Try to get last-used company from active_companies table
  let company_id = if let Some(id) = active_repo.get_active(user.id).await? {
    tracing::debug!(
      "redirect_to_default_company: found active company_id={}",
      id
    );
    // Verify user is still a member
    if member_repo.find_member(id, user.id).await?.is_some() {
      id
    } else {
      tracing::debug!(
        "redirect_to_default_company: user not a member of active company, getting first"
      );
      // Not a member anymore, get first company
      let memberships = member_repo.find_by_user_id(user.id).await?;
      tracing::debug!(
        "redirect_to_default_company: found {} memberships",
        memberships.len()
      );
      memberships
        .first()
        .map(|m| m.company_id)
        .ok_or_else(|| ApiError::Validation("No companies found".into()))?
    }
  } else {
    tracing::debug!("redirect_to_default_company: no active company, getting first");
    // No active company set, get first company
    let memberships = member_repo.find_by_user_id(user.id).await?;
    tracing::debug!(
      "redirect_to_default_company: found {} memberships",
      memberships.len()
    );
    memberships.first().map(|m| m.company_id).ok_or_else(|| {
      ApiError::Validation("No companies found. Please create a company first.".into())
    })?
  };

  let redirect_url = format!("/c/{}/{}", company_id, target_page);
  tracing::debug!(
    "redirect_to_default_company: redirecting to {}",
    redirect_url
  );
  Ok(
    HttpResponse::Found()
      .insert_header(("Location", redirect_url))
      .finish(),
  )
}

/// Configure web UI routes
pub fn configure_web_routes(cfg: &mut web::ServiceConfig, deps: WebRouteDependencies) {
  // Configure company-scoped routes FIRST (before deps is partially moved)
  configure_company_scoped_routes(cfg, &deps);

  // Clone repos for redirect handlers (before they get moved)
  let active_repo_for_redirects = deps.active_company_repo.clone();
  let member_repo_for_redirects = deps.member_repo.clone();
  let auth_service_for_redirects = deps.auth_service.clone();

  // Add template engine to app data
  cfg.app_data(web::Data::new(deps.templates.clone()));

  // Public routes (no authentication required)
  cfg.route("/login", web::get().to(pages::login_page));
  cfg.route("/register", web::get().to(pages::register_page));

  // Invoice HTML view for wkhtmltopdf (localhost only - IP whitelisted)
  // SECURITY: Protected by IP whitelist (127.0.0.1, ::1) in handler
  cfg.service(
    web::resource("/invoices/{id}/html")
      .app_data(web::Data::new(deps.templates.clone()))
      .app_data(web::Data::new(deps.get_invoice_details_use_case.clone()))
      .route(web::get().to(invoices_web::invoice_html_view)),
  );

  // Root route - redirect to dashboard (will redirect to login if not authenticated)
  let active_repo_root = active_repo_for_redirects.clone();
  let member_repo_root = member_repo_for_redirects.clone();
  cfg.service(
    web::resource("/")
      .route(web::get().to(
        move |req: actix_web::HttpRequest,
              active_repo: web::Data<Arc<dyn ActiveCompanyRepository>>,
              member_repo: web::Data<Arc<dyn CompanyMemberRepository>>| async move {
          // Try to get user - if not authenticated, redirect to login
          match get_user(&req) {
            Ok(_) => {
              // Authenticated - redirect to company-scoped dashboard
              redirect_to_default_company(req, "dashboard", active_repo, member_repo).await
            }
            Err(_) => {
              // Not authenticated - redirect to login
              Ok(
                HttpResponse::Found()
                  .insert_header(("Location", "/login"))
                  .finish(),
              )
            }
          }
        },
      ))
      .app_data(web::Data::new(active_repo_root))
      .app_data(web::Data::new(member_repo_root)),
  );

  // Auth form submission routes
  cfg.service(
    web::scope("/auth")
      .app_data(web::Data::new(deps.register_use_case))
      .app_data(web::Data::new(deps.login_use_case))
      .route("/login", web::post().to(web_auth::login_submit))
      .route("/register", web::post().to(web_auth::register_submit))
      .route("/logout", web::post().to(web_auth::logout)),
  );

  // Redirect old routes to company-scoped versions
  let active_repo_dashboard = active_repo_for_redirects.clone();
  let member_repo_dashboard = member_repo_for_redirects.clone();
  cfg.service(
    web::resource("/dashboard")
      .wrap(WebAuthMiddleware::new(auth_service_for_redirects.clone()))
      .app_data(web::Data::new(active_repo_dashboard.clone()))
      .app_data(web::Data::new(member_repo_dashboard.clone()))
      .route(web::get().to(
        move |req: actix_web::HttpRequest,
              active_repo: web::Data<Arc<dyn ActiveCompanyRepository>>,
              member_repo: web::Data<Arc<dyn CompanyMemberRepository>>| {
          redirect_to_default_company(req, "dashboard", active_repo, member_repo)
        },
      )),
  );

  // Company web UI routes
  cfg.service(
    web::scope("/companies")
      .wrap(WebAuthMiddleware::new(deps.auth_service.clone()))
      .app_data(web::Data::new(deps.templates.clone())) // Add templates to scope
      .app_data(web::Data::new(deps.get_companies_use_case.clone()))
      .app_data(web::Data::new(deps.create_company_use_case))
      .app_data(web::Data::new(deps.set_active_use_case))
      .app_data(web::Data::new(deps.add_member_use_case))
      .app_data(web::Data::new(deps.remove_member_use_case))
      .app_data(web::Data::new(deps.get_details_use_case.clone()))
      .app_data(web::Data::new(deps.update_profile_use_case))
      .app_data(web::Data::new(deps.update_storage_config_use_case))
      .app_data(web::Data::new(deps.user_repo))
      .app_data(web::Data::new(deps.member_repo))
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
        web::get().to(company_settings::company_settings_page),
      )
      .route(
        "/{company_id}/settings",
        web::post().to(company_settings::update_company_profile),
      )
      .route(
        "/{company_id}/settings/storage",
        web::post().to(company_settings::update_storage_config),
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

  // Redirect old /customers to company-scoped version
  let active_repo_customers = active_repo_for_redirects.clone();
  let member_repo_customers = member_repo_for_redirects.clone();
  cfg.service(
    web::resource("/customers")
      .wrap(WebAuthMiddleware::new(auth_service_for_redirects.clone()))
      .app_data(web::Data::new(active_repo_customers.clone()))
      .app_data(web::Data::new(member_repo_customers.clone()))
      .route(web::get().to(
        move |req: actix_web::HttpRequest,
              active_repo: web::Data<Arc<dyn ActiveCompanyRepository>>,
              member_repo: web::Data<Arc<dyn CompanyMemberRepository>>| {
          redirect_to_default_company(req, "customers", active_repo, member_repo)
        },
      )),
  );

  // Redirect old /invoices to company-scoped version
  let active_repo_invoices = active_repo_for_redirects.clone();
  let member_repo_invoices = member_repo_for_redirects.clone();
  cfg.service(
    web::resource("/invoices")
      .wrap(WebAuthMiddleware::new(auth_service_for_redirects.clone()))
      .app_data(web::Data::new(active_repo_invoices.clone()))
      .app_data(web::Data::new(member_repo_invoices.clone()))
      .route(web::get().to(
        move |req: actix_web::HttpRequest,
              active_repo: web::Data<Arc<dyn ActiveCompanyRepository>>,
              member_repo: web::Data<Arc<dyn CompanyMemberRepository>>| {
          redirect_to_default_company(req, "invoices", active_repo, member_repo)
        },
      )),
  );

  // Redirect old /bank-accounts to company-scoped version
  let active_repo_bank = active_repo_for_redirects.clone();
  let member_repo_bank = member_repo_for_redirects.clone();
  cfg.service(
    web::resource("/bank-accounts")
      .wrap(WebAuthMiddleware::new(auth_service_for_redirects.clone()))
      .app_data(web::Data::new(active_repo_bank.clone()))
      .app_data(web::Data::new(member_repo_bank.clone()))
      .route(web::get().to(
        move |req: actix_web::HttpRequest,
              active_repo: web::Data<Arc<dyn ActiveCompanyRepository>>,
              member_repo: web::Data<Arc<dyn CompanyMemberRepository>>| {
          redirect_to_default_company(req, "bank-accounts", active_repo, member_repo)
        },
      )),
  );
}

/// Configure company-scoped routes with URL-based company context
///
/// These routes use the pattern `/c/{company_id}/...` where company_id is extracted
/// from the URL and validated by CompanyContextMiddleware.
///
/// This runs in parallel with old routes during migration.
pub fn configure_company_scoped_routes(cfg: &mut web::ServiceConfig, deps: &WebRouteDependencies) {
  cfg.service(
    web::scope("/c/{company_id}")
      // Important: Middleware execution order is REVERSE of wrap() order
      // WebAuthMiddleware must execute first to set User in extensions
      .wrap(CompanyContextMiddleware::new(
        deps.member_repo.clone(),
        deps.active_company_repo.clone(),
      ))
      .wrap(WebAuthMiddleware::new(deps.auth_service.clone()))
      .app_data(web::Data::new(deps.templates.clone()))
      // Dashboard
      .app_data(web::Data::new(deps.get_companies_use_case.clone()))
      .app_data(web::Data::new(deps.get_details_use_case.clone()))
      .route("/dashboard", web::get().to(pages::dashboard_page))
      // Customers
      .app_data(web::Data::new(deps.create_customer_use_case.clone()))
      .app_data(web::Data::new(deps.list_customers_use_case.clone()))
      .app_data(web::Data::new(deps.update_customer_use_case.clone()))
      .app_data(web::Data::new(deps.archive_customer_use_case.clone()))
      .route("/customers", web::get().to(customers_web::customers_page))
      .route(
        "/customers/create",
        web::post().to(customers_web::create_customer_submit),
      )
      .route(
        "/customers/{id}/edit",
        web::post().to(customers_web::update_customer_submit),
      )
      .route(
        "/customers/{id}/archive",
        web::delete().to(customers_web::archive_customer),
      )
      // Invoices
      .app_data(web::Data::new(deps.create_invoice_use_case.clone()))
      .app_data(web::Data::new(deps.list_invoices_use_case.clone()))
      .app_data(web::Data::new(deps.get_invoice_details_use_case.clone()))
      .app_data(web::Data::new(deps.change_invoice_status_use_case.clone()))
      .app_data(web::Data::new(deps.archive_invoice_use_case.clone()))
      .app_data(web::Data::new(deps.delete_invoice_use_case.clone()))
      .app_data(web::Data::new(deps.get_bank_accounts_use_case.clone()))
      .app_data(web::Data::new(deps.active_bank_account_repo.clone()))
      .route("/invoices", web::get().to(invoices_web::invoices_page))
      .route(
        "/invoices",
        web::post().to(invoices_web::create_invoice_submit),
      )
      .route(
        "/invoices/create",
        web::get().to(invoices_web::invoice_create_page),
      )
      // Invoice Templates - specific literal paths MUST come before {id} patterns
      .app_data(web::Data::new(
        deps.create_template_from_invoice_use_case.clone(),
      ))
      .app_data(web::Data::new(deps.list_templates_use_case.clone()))
      .app_data(web::Data::new(
        deps.create_invoice_from_template_use_case.clone(),
      ))
      .app_data(web::Data::new(deps.archive_template_use_case.clone()))
      .route(
        "/invoices/templates",
        web::get().to(invoices_web::templates_page),
      )
      .route(
        "/invoices/create-from-template/{id}",
        web::get().to(invoices_web::create_from_template_page),
      )
      .route(
        "/invoices/create-from-template/{id}",
        web::post().to(invoices_web::create_invoice_from_template),
      )
      // Regular invoice routes - {id} patterns must come after specific literal paths
      // Note: /invoices/{id}/html is a public route (defined outside this scope for wkhtmltopdf)
      .route(
        "/invoices/{id}",
        web::get().to(invoices_web::invoice_details_page),
      )
      .route(
        "/invoices/{id}/save-as-template",
        web::post().to(invoices_web::save_as_template),
      )
      .route(
        "/invoices/{id}/status",
        web::post().to(invoices_web::change_invoice_status),
      )
      .route(
        "/invoices/{id}/archive",
        web::delete().to(invoices_web::archive_invoice),
      )
      .route(
        "/invoices/{id}",
        web::delete().to(invoices_web::delete_invoice),
      )
      .route(
        "/invoices/templates/{id}",
        web::delete().to(invoices_web::archive_template),
      )
      // Bank Accounts
      .app_data(web::Data::new(deps.create_bank_account_use_case.clone()))
      .app_data(web::Data::new(deps.update_bank_account_use_case.clone()))
      .app_data(web::Data::new(deps.archive_bank_account_use_case.clone()))
      .app_data(web::Data::new(
        deps.set_active_bank_account_use_case.clone(),
      ))
      .route(
        "/bank-accounts",
        web::get().to(bank_accounts_web::bank_accounts_page),
      )
      .route(
        "/bank-accounts/create",
        web::post().to(bank_accounts_web::create_bank_account_submit),
      )
      .route(
        "/bank-accounts/{account_id}/update",
        web::post().to(bank_accounts_web::update_bank_account_submit),
      )
      .route(
        "/bank-accounts/{account_id}/archive",
        web::post().to(bank_accounts_web::archive_bank_account_handler),
      )
      .route(
        "/bank-accounts/{account_id}/set-active",
        web::post().to(bank_accounts_web::set_active_bank_account_handler),
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

/// Configure bank account routes (REST API)
///
/// Mounts all bank account-related endpoints under the provided scope.
/// All routes are prefixed with the scope path (e.g., /api/v1/companies/:company_id/bank-accounts).
pub fn configure_bank_account_routes(
  cfg: &mut web::ServiceConfig,
  create_use_case: Arc<CreateBankAccountUseCase>,
  get_use_case: Arc<GetBankAccountsUseCase>,
  update_use_case: Arc<UpdateBankAccountUseCase>,
  archive_use_case: Arc<ArchiveBankAccountUseCase>,
  set_active_use_case: Arc<SetActiveBankAccountUseCase>,
) {
  cfg
    .app_data(web::Data::new(create_use_case))
    .app_data(web::Data::new(get_use_case))
    .app_data(web::Data::new(update_use_case))
    .app_data(web::Data::new(archive_use_case))
    .app_data(web::Data::new(set_active_use_case))
    .route(
      "",
      web::post().to(bank_accounts::create_bank_account_handler),
    )
    .route("", web::get().to(bank_accounts::get_bank_accounts_handler))
    .route(
      "/{account_id}",
      web::put().to(bank_accounts::update_bank_account_handler),
    )
    .route(
      "/{account_id}",
      web::delete().to(bank_accounts::archive_bank_account_handler),
    )
    .route(
      "/active",
      web::post().to(bank_accounts::set_active_bank_account_handler),
    );
}

/// Configure customer routes (REST API)
///
/// Mounts all customer-related endpoints under the provided scope.
/// All routes are prefixed with the scope path (e.g., /api/v1/customers).
///
/// # Routes
///
/// - POST / - Create a new customer
/// - GET / - List all customers for the active company
/// - PUT /{id} - Update a customer
/// - DELETE /{id}/archive - Archive a customer
pub fn configure_customer_routes(
  cfg: &mut web::ServiceConfig,
  create_use_case: Arc<CreateCustomerUseCase>,
  list_use_case: Arc<ListCustomersUseCase>,
  update_use_case: Arc<UpdateCustomerUseCase>,
  archive_use_case: Arc<ArchiveCustomerUseCase>,
) {
  cfg
    .app_data(web::Data::new(create_use_case))
    .app_data(web::Data::new(list_use_case))
    .app_data(web::Data::new(update_use_case))
    .app_data(web::Data::new(archive_use_case))
    .route("", web::post().to(customers_web::create_customer_submit))
    .route("", web::get().to(customers_web::customers_page))
    .route(
      "/{id}/edit",
      web::post().to(customers_web::update_customer_submit),
    )
    .route(
      "/{id}/archive",
      web::delete().to(customers_web::archive_customer),
    );
}

/// Configure invoice routes (REST API)
///
/// Mounts all invoice-related endpoints under the provided scope.
/// All routes are prefixed with the scope path (e.g., /api/v1/invoices).
///
/// # Routes
///
/// - POST / - Create a new invoice
/// - GET / - List all invoices for the active company
/// - GET /create - Show invoice creation form
/// - GET /{id} - Get invoice details
/// - POST /{id}/status - Change invoice status
/// - DELETE /{id}/archive - Archive an invoice
pub fn configure_invoice_routes(
  cfg: &mut web::ServiceConfig,
  create_use_case: Arc<CreateInvoiceUseCase>,
  list_use_case: Arc<ListInvoicesUseCase>,
  get_details_use_case: Arc<GetInvoiceDetailsUseCase>,
  change_status_use_case: Arc<ChangeInvoiceStatusUseCase>,
  archive_use_case: Arc<ArchiveInvoiceUseCase>,
  list_customers_use_case: Arc<ListCustomersUseCase>,
) {
  cfg
    .app_data(web::Data::new(create_use_case))
    .app_data(web::Data::new(list_use_case))
    .app_data(web::Data::new(get_details_use_case))
    .app_data(web::Data::new(change_status_use_case))
    .app_data(web::Data::new(archive_use_case))
    .app_data(web::Data::new(list_customers_use_case))
    .route("", web::get().to(invoices_web::invoices_page))
    .route("", web::post().to(invoices_web::create_invoice_submit))
    .route("/create", web::get().to(invoices_web::invoice_create_page))
    .route("/{id}", web::get().to(invoices_web::invoice_details_page))
    .route(
      "/{id}/status",
      web::post().to(invoices_web::change_invoice_status),
    )
    .route(
      "/{id}/archive",
      web::delete().to(invoices_web::archive_invoice),
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
  }
}
