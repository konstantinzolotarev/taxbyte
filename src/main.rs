use actix_files as fs;
use actix_web::{App, HttpServer, middleware::Logger, web};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use taxbyte::{
  adapters::http::{
    AuthMiddleware, RequestIdMiddleware, TemplateEngine, WebRouteDependencies,
    configure_auth_routes, configure_bank_account_routes, configure_company_routes,
    configure_customer_routes, configure_invoice_routes, configure_web_routes,
  },
  application::auth::{
    GetCurrentUserUseCase, LoginUserUseCase, LogoutAllDevicesUseCase, LogoutUserUseCase,
    RegisterUserUseCase,
  },
  application::company::{
    AddCompanyMemberUseCase, ArchiveBankAccountUseCase, CreateBankAccountUseCase,
    CreateCompanyUseCase, GetBankAccountsUseCase, GetCompanyDetailsUseCase,
    GetUserCompaniesUseCase, RemoveCompanyMemberUseCase, SetActiveBankAccountUseCase,
    SetActiveCompanyUseCase, UpdateBankAccountUseCase, UpdateCompanyProfileUseCase,
  },
  application::invoice::{
    ArchiveCustomerUseCase, ArchiveInvoiceUseCase, ChangeInvoiceStatusUseCase,
    CreateCustomerUseCase, CreateInvoiceUseCase, GetInvoiceDetailsUseCase, ListCustomersUseCase,
    ListInvoicesUseCase, UpdateCustomerUseCase,
  },
  domain::auth::services::AuthService,
  domain::company::services::CompanyService,
  domain::invoice::InvoiceService,
  infrastructure::{
    config::Config,
    persistence::postgres::{
      PostgresActiveBankAccountRepository, PostgresActiveCompanyRepository,
      PostgresBankAccountRepository, PostgresCompanyMemberRepository, PostgresCompanyRepository,
      PostgresCustomerRepository, PostgresInvoiceLineItemRepository, PostgresInvoiceRepository,
      PostgresLoginAttemptRepository, PostgresSessionRepository, PostgresUserRepository,
    },
    security::{Argon2PasswordHasher, SecureTokenGenerator},
  },
};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
  // Initialize environment variables from .env file
  dotenvy::dotenv().ok();

  // Initialize tracing subscriber for logging
  tracing_subscriber::registry()
    .with(
      tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "taxbyte=debug,actix_web=info".into()),
    )
    .with(tracing_subscriber::fmt::layer())
    .init();

  tracing::info!("Starting TaxByte application");

  // Load configuration
  let config = Config::load().expect("Failed to load configuration");
  tracing::info!("Configuration loaded successfully");

  // Set up database connection pool with timeout
  tracing::info!("Connecting to database: {}", config.database.url);

  let db_pool = tokio::time::timeout(
    Duration::from_secs(config.database.connect_timeout_seconds),
    PgPoolOptions::new()
      .max_connections(config.database.max_connections)
      .acquire_timeout(Duration::from_secs(config.database.acquire_timeout_seconds))
      .connect(&config.database.url),
  )
  .await
  .map_err(|_| {
    tracing::error!(
      "Database connection timed out after {} seconds. Is PostgreSQL running?",
      config.database.connect_timeout_seconds
    );
    std::io::Error::new(
      std::io::ErrorKind::TimedOut,
      format!(
        "Database connection timed out after {} seconds",
        config.database.connect_timeout_seconds
      ),
    )
  })?
  .map_err(|e| {
    tracing::error!("Failed to connect to database: {}", e);
    match e {
      sqlx::Error::Io(_) => std::io::Error::new(
        std::io::ErrorKind::ConnectionRefused,
        format!(
          "Could not connect to database. Is PostgreSQL running at {}?",
          config.database.url
        ),
      ),
      _ => std::io::Error::other(format!("Database error: {}", e)),
    }
  })?;

  tracing::info!("Database connection pool created");

  // Run database migrations
  tracing::info!("Running database migrations");
  sqlx::migrate!("./migrations")
    .run(&db_pool)
    .await
    .expect("Failed to run database migrations");
  tracing::info!("Database migrations completed");

  // Set up Redis connection with timeout
  tracing::info!("Connecting to Redis: {}", config.redis.url);

  let redis_client = redis::Client::open(config.redis.url.clone()).map_err(|e| {
    tracing::error!("Failed to create Redis client: {}", e);
    std::io::Error::new(
      std::io::ErrorKind::InvalidInput,
      format!("Invalid Redis URL: {}", e),
    )
  })?;

  let redis_conn = tokio::time::timeout(
    Duration::from_secs(config.redis.connect_timeout_seconds),
    redis_client.get_connection_manager(),
  )
  .await
  .map_err(|_| {
    tracing::error!(
      "Redis connection timed out after {} seconds. Is Redis running?",
      config.redis.connect_timeout_seconds
    );
    std::io::Error::new(
      std::io::ErrorKind::TimedOut,
      format!(
        "Redis connection timed out after {} seconds",
        config.redis.connect_timeout_seconds
      ),
    )
  })?
  .map_err(|e| {
    tracing::error!("Failed to connect to Redis: {}", e);
    std::io::Error::new(
      std::io::ErrorKind::ConnectionRefused,
      format!(
        "Could not connect to Redis. Is Redis running at {}?",
        config.redis.url
      ),
    )
  })?;

  tracing::info!("Redis connection established");

  // Initialize repositories
  let user_repo = Arc::new(PostgresUserRepository::new(db_pool.clone()));
  let session_repo = Arc::new(PostgresSessionRepository::new(
    db_pool.clone(),
    redis_conn.clone(),
  ));
  let login_attempt_repo = Arc::new(PostgresLoginAttemptRepository::new(db_pool.clone()));

  // Initialize company repositories
  let company_repo = Arc::new(PostgresCompanyRepository::new(db_pool.clone()));
  let company_member_repo = Arc::new(PostgresCompanyMemberRepository::new(db_pool.clone()));
  let active_company_repo = Arc::new(PostgresActiveCompanyRepository::new(db_pool.clone()));
  let bank_account_repo = Arc::new(PostgresBankAccountRepository::new(db_pool.clone()));
  let active_bank_account_repo =
    Arc::new(PostgresActiveBankAccountRepository::new(db_pool.clone()));

  // Initialize invoice repositories
  let customer_repo = Arc::new(PostgresCustomerRepository::new(db_pool.clone()));
  let invoice_repo = Arc::new(PostgresInvoiceRepository::new(db_pool.clone()));
  let invoice_line_item_repo = Arc::new(PostgresInvoiceLineItemRepository::new(db_pool.clone()));

  // Initialize security services
  let password_hasher =
    Arc::new(Argon2PasswordHasher::new().expect("Failed to create password hasher"));
  let token_generator = Arc::new(SecureTokenGenerator::new());

  // Initialize domain service
  let auth_service = Arc::new(AuthService::new(
    user_repo.clone(),
    session_repo.clone(),
    login_attempt_repo.clone(),
    password_hasher,
    token_generator,
  ));

  // Initialize company service
  let company_service = Arc::new(CompanyService::new(
    company_repo.clone(),
    company_member_repo.clone(),
    active_company_repo.clone(),
    user_repo.clone(),
    bank_account_repo.clone(),
    active_bank_account_repo.clone(),
  ));

  // Initialize invoice service
  let invoice_service = Arc::new(InvoiceService::new(
    invoice_repo.clone(),
    invoice_line_item_repo.clone(),
    customer_repo.clone(),
    company_member_repo.clone(),
    company_repo.clone(),
    bank_account_repo.clone(),
  ));

  // Initialize use cases
  let register_use_case = Arc::new(RegisterUserUseCase::new(auth_service.clone()));
  let login_use_case = Arc::new(LoginUserUseCase::new(auth_service.clone()));
  let logout_use_case = Arc::new(LogoutUserUseCase::new(auth_service.clone()));
  let logout_all_use_case = Arc::new(LogoutAllDevicesUseCase::new(auth_service.clone()));
  let get_user_use_case = Arc::new(GetCurrentUserUseCase::new(auth_service.clone()));

  // Initialize company use cases
  let create_company_use_case = Arc::new(CreateCompanyUseCase::new(company_service.clone()));
  let get_companies_use_case = Arc::new(GetUserCompaniesUseCase::new(
    company_service.clone(),
    company_member_repo.clone(),
    active_company_repo.clone(),
  ));
  let set_active_use_case = Arc::new(SetActiveCompanyUseCase::new(company_service.clone()));
  let add_member_use_case = Arc::new(AddCompanyMemberUseCase::new(company_service.clone()));
  let remove_member_use_case = Arc::new(RemoveCompanyMemberUseCase::new(company_service.clone()));
  let get_details_use_case = Arc::new(GetCompanyDetailsUseCase::new(
    company_service.clone(),
    company_member_repo.clone(),
  ));
  let update_profile_use_case = Arc::new(UpdateCompanyProfileUseCase::new(company_service.clone()));

  // Initialize bank account use cases
  let create_bank_account_use_case =
    Arc::new(CreateBankAccountUseCase::new(company_service.clone()));
  let get_bank_accounts_use_case = Arc::new(GetBankAccountsUseCase::new(company_service.clone()));
  let update_bank_account_use_case =
    Arc::new(UpdateBankAccountUseCase::new(company_service.clone()));
  let archive_bank_account_use_case =
    Arc::new(ArchiveBankAccountUseCase::new(company_service.clone()));
  let set_active_bank_account_use_case =
    Arc::new(SetActiveBankAccountUseCase::new(company_service.clone()));

  // Initialize customer use cases
  let create_customer_use_case = Arc::new(CreateCustomerUseCase::new(invoice_service.clone()));
  let list_customers_use_case = Arc::new(ListCustomersUseCase::new(invoice_service.clone()));
  let update_customer_use_case = Arc::new(UpdateCustomerUseCase::new(invoice_service.clone()));
  let archive_customer_use_case = Arc::new(ArchiveCustomerUseCase::new(invoice_service.clone()));

  // Initialize invoice use cases
  let create_invoice_use_case = Arc::new(CreateInvoiceUseCase::new(invoice_service.clone()));
  let list_invoices_use_case = Arc::new(ListInvoicesUseCase::new(invoice_service.clone()));
  let get_invoice_details_use_case =
    Arc::new(GetInvoiceDetailsUseCase::new(invoice_service.clone()));
  let change_invoice_status_use_case =
    Arc::new(ChangeInvoiceStatusUseCase::new(invoice_service.clone()));
  let archive_invoice_use_case = Arc::new(ArchiveInvoiceUseCase::new(invoice_service.clone()));

  // Initialize template engine
  let templates = TemplateEngine::new().expect("Failed to initialize template engine");
  tracing::info!("Template engine initialized");

  let server_host = config.server.host.clone();
  let server_port = config.server.port;

  tracing::info!("Starting HTTP server on {}:{}", server_host, server_port);

  // Create and start the HTTP server
  HttpServer::new(move || {
    App::new()
      // Add request ID middleware
      .wrap(RequestIdMiddleware::new())
      // Add logging middleware
      .wrap(Logger::default())
      // Configure web UI routes
      .configure(|cfg| {
        configure_web_routes(
          cfg,
          WebRouteDependencies {
            templates: templates.clone(),
            auth_service: auth_service.clone(),
            register_use_case: register_use_case.clone(),
            login_use_case: login_use_case.clone(),
            get_companies_use_case: get_companies_use_case.clone(),
            create_company_use_case: create_company_use_case.clone(),
            set_active_use_case: set_active_use_case.clone(),
            add_member_use_case: add_member_use_case.clone(),
            remove_member_use_case: remove_member_use_case.clone(),
            get_details_use_case: get_details_use_case.clone(),
            update_profile_use_case: update_profile_use_case.clone(),
            create_bank_account_use_case: create_bank_account_use_case.clone(),
            get_bank_accounts_use_case: get_bank_accounts_use_case.clone(),
            update_bank_account_use_case: update_bank_account_use_case.clone(),
            archive_bank_account_use_case: archive_bank_account_use_case.clone(),
            set_active_bank_account_use_case: set_active_bank_account_use_case.clone(),
            user_repo: user_repo.clone(),
            member_repo: company_member_repo.clone(),
            active_bank_account_repo: active_bank_account_repo.clone(),
            // Customer use cases
            create_customer_use_case: create_customer_use_case.clone(),
            list_customers_use_case: list_customers_use_case.clone(),
            update_customer_use_case: update_customer_use_case.clone(),
            archive_customer_use_case: archive_customer_use_case.clone(),
            // Invoice use cases
            create_invoice_use_case: create_invoice_use_case.clone(),
            list_invoices_use_case: list_invoices_use_case.clone(),
            get_invoice_details_use_case: get_invoice_details_use_case.clone(),
            change_invoice_status_use_case: change_invoice_status_use_case.clone(),
            archive_invoice_use_case: archive_invoice_use_case.clone(),
          },
        )
      })
      // Configure API routes
      .service(web::scope("/api/v1/auth").configure(|cfg| {
        configure_auth_routes(
          cfg,
          register_use_case.clone(),
          login_use_case.clone(),
          logout_use_case.clone(),
          logout_all_use_case.clone(),
          get_user_use_case.clone(),
        )
      }))
      // Configure company API routes (protected with AuthMiddleware)
      .service(
        web::scope("/api/v1/companies")
          .wrap(AuthMiddleware::new(get_user_use_case.clone()))
          .configure(|cfg| {
            configure_company_routes(
              cfg,
              create_company_use_case.clone(),
              get_companies_use_case.clone(),
              set_active_use_case.clone(),
              add_member_use_case.clone(),
              remove_member_use_case.clone(),
            )
          })
          .service(web::scope("/{company_id}/bank-accounts").configure(|cfg| {
            configure_bank_account_routes(
              cfg,
              create_bank_account_use_case.clone(),
              get_bank_accounts_use_case.clone(),
              update_bank_account_use_case.clone(),
              archive_bank_account_use_case.clone(),
              set_active_bank_account_use_case.clone(),
            )
          })),
      )
      // Configure customer API routes (protected with AuthMiddleware)
      .service(
        web::scope("/api/v1/customers")
          .wrap(AuthMiddleware::new(get_user_use_case.clone()))
          .configure(|cfg| {
            configure_customer_routes(
              cfg,
              create_customer_use_case.clone(),
              list_customers_use_case.clone(),
              update_customer_use_case.clone(),
              archive_customer_use_case.clone(),
            )
          }),
      )
      // Configure invoice API routes (protected with AuthMiddleware)
      .service(
        web::scope("/api/v1/invoices")
          .wrap(AuthMiddleware::new(get_user_use_case.clone()))
          .configure(|cfg| {
            configure_invoice_routes(
              cfg,
              create_invoice_use_case.clone(),
              list_invoices_use_case.clone(),
              get_invoice_details_use_case.clone(),
              change_invoice_status_use_case.clone(),
              archive_invoice_use_case.clone(),
              list_customers_use_case.clone(),
            )
          }),
      )
      // Static files
      .service(fs::Files::new("/static", "./static"))
      // Health check endpoint
      .route("/health", web::get().to(health_check))
  })
  .bind((server_host.as_str(), server_port))?
  .run()
  .await
}

/// Health check endpoint
async fn health_check() -> &'static str {
  "OK"
}
