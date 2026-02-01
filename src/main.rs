use actix_files as fs;
use actix_web::{middleware::Logger, web, App, HttpServer};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use taxbyte::{
  adapters::http::{
    configure_auth_routes, configure_company_routes, configure_web_routes, AuthMiddleware,
    RequestIdMiddleware, TemplateEngine,
  },
  application::auth::{
    GetCurrentUserUseCase, LoginUserUseCase, LogoutAllDevicesUseCase, LogoutUserUseCase,
    RegisterUserUseCase,
  },
  application::company::{
    AddCompanyMemberUseCase, CreateCompanyUseCase, GetCompanyDetailsUseCase,
    GetUserCompaniesUseCase, RemoveCompanyMemberUseCase, SetActiveCompanyUseCase,
    UpdateCompanyProfileUseCase,
  },
  domain::auth::services::AuthService,
  domain::company::services::CompanyService,
  infrastructure::{
    config::Config,
    persistence::postgres::{
      PostgresActiveCompanyRepository, PostgresCompanyMemberRepository, PostgresCompanyRepository,
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
      _ => std::io::Error::new(std::io::ErrorKind::Other, format!("Database error: {}", e)),
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
          templates.clone(),
          auth_service.clone(),
          register_use_case.clone(),
          login_use_case.clone(),
          get_companies_use_case.clone(),
          create_company_use_case.clone(),
          set_active_use_case.clone(),
          add_member_use_case.clone(),
          remove_member_use_case.clone(),
          get_details_use_case.clone(),
          update_profile_use_case.clone(),
          user_repo.clone(),
          company_member_repo.clone(),
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
