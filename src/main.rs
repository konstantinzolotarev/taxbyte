use actix_files as fs;
use actix_web::{App, HttpServer, middleware::Logger, web};
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
    AddCompanyMemberUseCase, ArchiveBankAccountUseCase, ConnectGoogleDriveUseCase,
    CreateBankAccountUseCase, CreateCompanyUseCase, DisconnectGoogleDriveUseCase,
    GetBankAccountsUseCase, GetCompanyDetailsUseCase, GetUserCompaniesUseCase,
    RemoveCompanyMemberUseCase, SetActiveBankAccountUseCase, SetActiveCompanyUseCase,
    TestDriveConnectionUseCase, UpdateBankAccountUseCase, UpdateCompanyProfileUseCase,
  },
  application::invoice::{
    ArchiveCustomerUseCase, ArchiveInvoiceUseCase, ArchiveTemplateUseCase,
    ChangeInvoiceStatusUseCase, CreateCustomerUseCase, CreateInvoiceFromTemplateUseCase,
    CreateInvoiceUseCase, CreateTemplateFromInvoiceUseCase, DeleteInvoiceUseCase,
    GetInvoiceDetailsUseCase, ListArchivedInvoicesUseCase, ListCustomersUseCase,
    ListInvoicesUseCase, ListTemplatesUseCase, PermanentlyDeleteInvoiceUseCase,
    ReuploadInvoiceUseCase, UnarchiveInvoiceUseCase, UpdateCustomerUseCase,
  },
  domain::auth::{
    ports::{LoginAttemptRepository, SessionRepository, UserRepository},
    services::{AuthService, AuthServiceConfig},
  },
  domain::company::{
    ports::{
      ActiveBankAccountRepository, ActiveCompanyRepository, BankAccountRepository,
      CompanyMemberRepository, CompanyRepository,
    },
    services::CompanyService,
  },
  domain::invoice::{
    InvoiceService, InvoiceServiceDependencies,
    ports::{
      CustomerRepository, InvoiceLineItemRepository, InvoiceRepository,
      InvoiceTemplateLineItemRepository, InvoiceTemplateRepository,
    },
  },
  domain::report::ports::{
    BankTransactionRepository as BankTxRepo, MonthlyReportRepository,
    ReceivedInvoiceRepository as RecvInvRepo,
  },
  infrastructure::{
    cloud::{GoogleOAuthManager, MockOAuthManager, OAuthManager},
    config::{Config, DatabaseBackend},
    security::{AesTokenEncryption, Argon2PasswordHasher, SecureTokenGenerator},
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
  tracing::info!(
    "Configuration loaded successfully (backend: {})",
    config.database.backend
  );

  // Type-erased repository handles (filled by backend-specific init)
  let user_repo: Arc<dyn UserRepository>;
  let session_repo: Arc<dyn SessionRepository>;
  let login_attempt_repo: Arc<dyn LoginAttemptRepository>;
  let company_repo: Arc<dyn CompanyRepository>;
  let company_member_repo: Arc<dyn CompanyMemberRepository>;
  let active_company_repo: Arc<dyn ActiveCompanyRepository>;
  let bank_account_repo: Arc<dyn BankAccountRepository>;
  let active_bank_account_repo: Arc<dyn ActiveBankAccountRepository>;
  let customer_repo: Arc<dyn CustomerRepository>;
  let invoice_repo: Arc<dyn InvoiceRepository>;
  let invoice_line_item_repo: Arc<dyn InvoiceLineItemRepository>;
  let invoice_template_repo: Arc<dyn InvoiceTemplateRepository>;
  let invoice_template_line_item_repo: Arc<dyn InvoiceTemplateLineItemRepository>;
  let monthly_report_repo: Arc<dyn MonthlyReportRepository>;
  let bank_transaction_repo: Arc<dyn BankTxRepo>;
  let received_invoice_repo: Arc<dyn RecvInvRepo>;

  match config.database.backend {
    DatabaseBackend::Postgres => {
      use sqlx::postgres::PgPoolOptions;
      use taxbyte::infrastructure::persistence::postgres::*;

      tracing::info!("Connecting to PostgreSQL: {}", config.database.url);

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

      tracing::info!("PostgreSQL connection pool created");

      // Run PostgreSQL migrations
      tracing::info!("Running PostgreSQL migrations");
      sqlx::migrate!("./migrations/postgresql")
        .run(&db_pool)
        .await
        .expect("Failed to run database migrations");
      tracing::info!("Database migrations completed");

      // Set up Redis connection (required for PostgreSQL mode)
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

      // Initialize PostgreSQL repositories
      user_repo = Arc::new(PostgresUserRepository::new(db_pool.clone()));
      session_repo = Arc::new(PostgresSessionRepository::new(
        db_pool.clone(),
        redis_conn.clone(),
      ));
      login_attempt_repo = Arc::new(PostgresLoginAttemptRepository::new(db_pool.clone()));
      company_repo = Arc::new(PostgresCompanyRepository::new(db_pool.clone()));
      company_member_repo = Arc::new(PostgresCompanyMemberRepository::new(db_pool.clone()));
      active_company_repo = Arc::new(PostgresActiveCompanyRepository::new(db_pool.clone()));
      bank_account_repo = Arc::new(PostgresBankAccountRepository::new(db_pool.clone()));
      active_bank_account_repo =
        Arc::new(PostgresActiveBankAccountRepository::new(db_pool.clone()));
      customer_repo = Arc::new(PostgresCustomerRepository::new(db_pool.clone()));
      invoice_repo = Arc::new(PostgresInvoiceRepository::new(db_pool.clone()));
      invoice_line_item_repo = Arc::new(PostgresInvoiceLineItemRepository::new(db_pool.clone()));
      invoice_template_repo = Arc::new(PostgresInvoiceTemplateRepository::new(db_pool.clone()));
      invoice_template_line_item_repo = Arc::new(PostgresInvoiceTemplateLineItemRepository::new(
        db_pool.clone(),
      ));
      monthly_report_repo = Arc::new(PostgresMonthlyReportRepository::new(db_pool.clone()));
      bank_transaction_repo = Arc::new(PostgresBankTransactionRepository::new(db_pool.clone()));
      received_invoice_repo = Arc::new(PostgresReceivedInvoiceRepository::new(db_pool.clone()));
    }

    DatabaseBackend::Sqlite => {
      use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
      use std::str::FromStr;
      use taxbyte::infrastructure::persistence::sqlite::*;

      tracing::info!("Using SQLite backend: {}", config.database.url);

      // Ensure data directory exists for the SQLite database file
      if let Some(db_path) = config.database.url.strip_prefix("sqlite://") {
        let db_path = db_path.split('?').next().unwrap_or(db_path);
        if let Some(parent) = std::path::Path::new(db_path).parent() {
          std::fs::create_dir_all(parent).map_err(|e| {
            std::io::Error::new(
              e.kind(),
              format!("Failed to create database directory {:?}: {}", parent, e),
            )
          })?;
        }
      }

      let sqlite_options = SqliteConnectOptions::from_str(&config.database.url)
        .map_err(|e| std::io::Error::other(format!("Invalid SQLite URL: {}", e)))?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true);

      let db_pool = SqlitePoolOptions::new()
        .max_connections(config.database.max_connections)
        .acquire_timeout(Duration::from_secs(config.database.acquire_timeout_seconds))
        .connect_with(sqlite_options)
        .await
        .map_err(|e| {
          tracing::error!("Failed to connect to SQLite: {}", e);
          std::io::Error::other(format!("SQLite error: {}", e))
        })?;

      tracing::info!("SQLite connection pool created");

      // Run SQLite migrations
      tracing::info!("Running SQLite migrations");
      sqlx::migrate!("./migrations/sqlite")
        .run(&db_pool)
        .await
        .expect("Failed to run SQLite migrations");
      tracing::info!("SQLite migrations completed");

      tracing::info!("Redis skipped (not needed for SQLite backend)");

      // Initialize SQLite repositories
      user_repo = Arc::new(SqliteUserRepository::new(db_pool.clone()));
      session_repo = Arc::new(SqliteSessionRepository::new(db_pool.clone()));
      login_attempt_repo = Arc::new(SqliteLoginAttemptRepository::new(db_pool.clone()));
      company_repo = Arc::new(SqliteCompanyRepository::new(db_pool.clone()));
      company_member_repo = Arc::new(SqliteCompanyMemberRepository::new(db_pool.clone()));
      active_company_repo = Arc::new(SqliteActiveCompanyRepository::new(db_pool.clone()));
      bank_account_repo = Arc::new(SqliteBankAccountRepository::new(db_pool.clone()));
      active_bank_account_repo = Arc::new(SqliteActiveBankAccountRepository::new(db_pool.clone()));
      customer_repo = Arc::new(SqliteCustomerRepository::new(db_pool.clone()));
      invoice_repo = Arc::new(SqliteInvoiceRepository::new(db_pool.clone()));
      invoice_line_item_repo = Arc::new(SqliteInvoiceLineItemRepository::new(db_pool.clone()));
      invoice_template_repo = Arc::new(SqliteInvoiceTemplateRepository::new(db_pool.clone()));
      invoice_template_line_item_repo = Arc::new(SqliteInvoiceTemplateLineItemRepository::new(
        db_pool.clone(),
      ));
      monthly_report_repo = Arc::new(SqliteMonthlyReportRepository::new(db_pool.clone()));
      bank_transaction_repo = Arc::new(SqliteBankTransactionRepository::new(db_pool.clone()));
      received_invoice_repo = Arc::new(SqliteReceivedInvoiceRepository::new(db_pool.clone()));
    }
  }

  // --- Everything below is backend-agnostic ---

  // Initialize security services
  let password_hasher =
    Arc::new(Argon2PasswordHasher::new().expect("Failed to create password hasher"));
  let token_generator = Arc::new(SecureTokenGenerator::new());

  // Initialize domain service
  let auth_config = AuthServiceConfig {
    session_ttl_seconds: config.security.session_ttl_seconds as i64,
    remember_me_ttl_seconds: config.security.remember_me_ttl_seconds as i64,
    rate_limit_window_seconds: config.rate_limit.login_window_seconds as i64,
    max_failed_attempts: config.rate_limit.login_max_attempts as i64,
  };

  let auth_service = Arc::new(AuthService::new(
    user_repo.clone(),
    session_repo.clone(),
    login_attempt_repo.clone(),
    password_hasher,
    token_generator,
    auth_config,
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
  let invoice_service = Arc::new(InvoiceService::new(InvoiceServiceDependencies {
    invoice_repo: invoice_repo.clone(),
    line_item_repo: invoice_line_item_repo.clone(),
    customer_repo: customer_repo.clone(),
    company_member_repo: company_member_repo.clone(),
    company_repo: company_repo.clone(),
    bank_account_repo: bank_account_repo.clone(),
    template_repo: invoice_template_repo.clone(),
    template_line_item_repo: invoice_template_line_item_repo.clone(),
  }));

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
  let update_storage_config_use_case = Arc::new(
    taxbyte::application::company::UpdateStorageConfigUseCase::new(company_service.clone()),
  );

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

  // Initialize OAuth dependencies and use cases
  let token_encryption = Arc::new(
    AesTokenEncryption::new(&config.security.encryption_key_base64)
      .expect("Failed to create token encryption"),
  );

  let oauth_manager: Option<Arc<dyn OAuthManager>> =
    if std::env::var("MOCK_OAUTH").unwrap_or_default() == "true" {
      tracing::info!("Using mock OAuth manager for development");
      Some(Arc::new(
        MockOAuthManager::new(
          "mock-client-id".to_string(),
          "mock-client-secret".to_string(),
          "http://localhost:8080/oauth/google/callback".to_string(),
        )
        .expect("Failed to create mock OAuth manager"),
      ))
    } else if let Some(google_drive_config) = &config.google_drive {
      if let (Some(client_id), Some(client_secret), Some(redirect_url)) = (
        &google_drive_config.oauth_client_id,
        &google_drive_config.oauth_client_secret,
        &google_drive_config.oauth_redirect_url,
      ) {
        Some(Arc::new(
          GoogleOAuthManager::new(
            client_id.clone(),
            client_secret.clone(),
            redirect_url.clone(),
          )
          .expect("Failed to create OAuth manager"),
        ))
      } else {
        tracing::warn!("Google Drive OAuth credentials not configured");
        None
      }
    } else {
      tracing::warn!("Google Drive configuration not found");
      None
    };

  let connect_google_drive_use_case = Arc::new(ConnectGoogleDriveUseCase::new(
    oauth_manager.clone().expect("OAuth manager not configured"),
    company_repo.clone(),
    token_encryption.clone(),
  ));

  let disconnect_google_drive_use_case =
    Arc::new(DisconnectGoogleDriveUseCase::new(company_repo.clone()));

  let test_drive_connection_use_case =
    Arc::new(TestDriveConnectionUseCase::new(company_repo.clone()));

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
  let archive_invoice_use_case = Arc::new(ArchiveInvoiceUseCase::new(invoice_service.clone()));
  let delete_invoice_use_case = Arc::new(DeleteInvoiceUseCase::new(invoice_service.clone()));
  let list_archived_invoices_use_case =
    Arc::new(ListArchivedInvoicesUseCase::new(invoice_service.clone()));
  let unarchive_invoice_use_case = Arc::new(UnarchiveInvoiceUseCase::new(invoice_service.clone()));
  let permanently_delete_invoice_use_case = Arc::new(PermanentlyDeleteInvoiceUseCase::new(
    invoice_service.clone(),
  ));

  // Initialize template use cases
  let create_template_from_invoice_use_case = Arc::new(CreateTemplateFromInvoiceUseCase::new(
    invoice_service.clone(),
  ));
  let list_templates_use_case = Arc::new(ListTemplatesUseCase::new(
    invoice_service.clone(),
    customer_repo.clone(),
  ));
  let create_invoice_from_template_use_case = Arc::new(CreateInvoiceFromTemplateUseCase::new(
    invoice_service.clone(),
    create_invoice_use_case.clone(),
  ));
  let archive_template_use_case = Arc::new(ArchiveTemplateUseCase::new(invoice_service.clone()));

  // Initialize report service and use cases
  let report_service = Arc::new(taxbyte::domain::report::ReportService::new(
    monthly_report_repo.clone(),
    bank_transaction_repo.clone(),
    received_invoice_repo.clone(),
  ));

  let csv_parser: Arc<dyn taxbyte::domain::report::BankStatementParser> =
    Arc::new(taxbyte::infrastructure::csv::SwedbankCsvParser::new());

  let invoice_data_extractor: Arc<dyn taxbyte::domain::report::InvoiceDataExtractor> =
    Arc::new(taxbyte::infrastructure::pdf::PdfInvoiceExtractor::new());

  let create_empty_report_use_case = Arc::new(
    taxbyte::application::report::CreateEmptyReportUseCase::new(report_service.clone()),
  );
  let import_bank_statement_use_case = Arc::new(
    taxbyte::application::report::ImportBankStatementUseCase::new(
      report_service.clone(),
      csv_parser,
    ),
  );
  let list_monthly_reports_use_case =
    Arc::new(taxbyte::application::report::ListMonthlyReportsUseCase::new(report_service.clone()));
  let get_report_details_use_case = Arc::new(
    taxbyte::application::report::GetReportDetailsUseCase::new(report_service.clone()),
  );
  let upload_received_invoice_use_case = Arc::new(
    taxbyte::application::report::UploadReceivedInvoiceUseCase::new(report_service.clone()),
  );
  let list_received_invoices_use_case = Arc::new(
    taxbyte::application::report::ListReceivedInvoicesUseCase::new(report_service.clone()),
  );
  let match_transaction_use_case = Arc::new(
    taxbyte::application::report::MatchTransactionUseCase::new(report_service.clone()),
  );
  let unmatch_transaction_use_case =
    Arc::new(taxbyte::application::report::UnmatchTransactionUseCase::new(report_service.clone()));
  let delete_report_use_case = Arc::new(taxbyte::application::report::DeleteReportUseCase::new(
    report_service.clone(),
  ));
  let delete_received_invoice_use_case = Arc::new(
    taxbyte::application::report::DeleteReceivedInvoiceUseCase::new(report_service.clone()),
  );
  let upload_receipt_use_case = Arc::new(taxbyte::application::report::UploadReceiptUseCase::new(
    report_service.clone(),
  ));

  // Generate report use case needs cloud storage — use a no-op placeholder
  // (actual Drive adapter is created per-company when generating)
  let report_cloud_storage: Arc<dyn taxbyte::domain::report::ReportCloudStorage> =
    Arc::new(taxbyte::infrastructure::cloud::report_drive_adapter::NoOpReportCloudStorage);
  let generate_report_use_case =
    Arc::new(taxbyte::application::report::GenerateReportUseCase::new(
      report_service.clone(),
      company_repo.clone(),
      invoice_repo.clone(),
      report_cloud_storage,
    ));

  // Initialize template engine
  let templates = TemplateEngine::new().expect("Failed to initialize template engine");
  tracing::info!("Template engine initialized");

  // Initialize PDF generator
  let pdf_output_dir = std::path::PathBuf::from(&config.pdf.output_dir);
  let pdf_generator = Arc::new(taxbyte::infrastructure::pdf::WkHtmlToPdfGenerator::new(
    pdf_output_dir,
    config.pdf.wkhtmltopdf_path.clone(),
    config.server.base_url.clone(),
  )) as Arc<dyn taxbyte::domain::invoice::ports::PdfGenerator>;
  tracing::info!("PDF generator initialized");

  // Initialize change invoice status use case (cloud storage configured per-company)
  let change_invoice_status_use_case = Arc::new(ChangeInvoiceStatusUseCase::new(
    invoice_service.clone(),
    pdf_generator.clone(),
    get_invoice_details_use_case.clone(),
    company_repo.clone(),
    token_encryption.clone(),
    connect_google_drive_use_case.clone(),
    Arc::new(config.clone()),
  ));

  let reupload_invoice_use_case = Arc::new(ReuploadInvoiceUseCase::new(
    invoice_service.clone(),
    pdf_generator.clone(),
    get_invoice_details_use_case.clone(),
    company_repo.clone(),
    token_encryption.clone(),
    connect_google_drive_use_case.clone(),
    Arc::new(config.clone()),
  ));

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
            update_storage_config_use_case: update_storage_config_use_case.clone(),
            create_bank_account_use_case: create_bank_account_use_case.clone(),
            get_bank_accounts_use_case: get_bank_accounts_use_case.clone(),
            update_bank_account_use_case: update_bank_account_use_case.clone(),
            archive_bank_account_use_case: archive_bank_account_use_case.clone(),
            set_active_bank_account_use_case: set_active_bank_account_use_case.clone(),
            user_repo: user_repo.clone(),
            member_repo: company_member_repo.clone(),
            active_company_repo: active_company_repo.clone(),
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
            reupload_invoice_use_case: reupload_invoice_use_case.clone(),
            archive_invoice_use_case: archive_invoice_use_case.clone(),
            delete_invoice_use_case: delete_invoice_use_case.clone(),
            // Archived invoice use cases
            list_archived_invoices_use_case: list_archived_invoices_use_case.clone(),
            unarchive_invoice_use_case: unarchive_invoice_use_case.clone(),
            permanently_delete_invoice_use_case: permanently_delete_invoice_use_case.clone(),
            // Template use cases
            create_template_from_invoice_use_case: create_template_from_invoice_use_case.clone(),
            list_templates_use_case: list_templates_use_case.clone(),
            create_invoice_from_template_use_case: create_invoice_from_template_use_case.clone(),
            archive_template_use_case: archive_template_use_case.clone(),
            // OAuth use cases
            connect_google_drive_use_case: connect_google_drive_use_case.clone(),
            disconnect_google_drive_use_case: disconnect_google_drive_use_case.clone(),
            test_drive_connection_use_case: test_drive_connection_use_case.clone(),
            // Report use cases
            create_empty_report_use_case: create_empty_report_use_case.clone(),
            import_bank_statement_use_case: import_bank_statement_use_case.clone(),
            list_monthly_reports_use_case: list_monthly_reports_use_case.clone(),
            get_report_details_use_case: get_report_details_use_case.clone(),
            upload_received_invoice_use_case: upload_received_invoice_use_case.clone(),
            list_received_invoices_use_case: list_received_invoices_use_case.clone(),
            match_transaction_use_case: match_transaction_use_case.clone(),
            unmatch_transaction_use_case: unmatch_transaction_use_case.clone(),
            generate_report_use_case: generate_report_use_case.clone(),
            delete_report_use_case: delete_report_use_case.clone(),
            delete_received_invoice_use_case: delete_received_invoice_use_case.clone(),
            upload_receipt_use_case: upload_receipt_use_case.clone(),
            invoice_data_extractor: invoice_data_extractor.clone(),
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
