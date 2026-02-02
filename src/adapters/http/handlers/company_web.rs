use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::adapters::http::errors::ApiError;
use crate::adapters::http::templates::TemplateEngine;
use crate::application::company::{
  AddCompanyMemberCommand, AddCompanyMemberUseCase, CompanyAddressData, CreateCompanyCommand,
  CreateCompanyUseCase, GetCompanyDetailsCommand, GetCompanyDetailsUseCase,
  GetUserCompaniesCommand, GetUserCompaniesUseCase, RemoveCompanyMemberCommand,
  RemoveCompanyMemberUseCase, SetActiveCompanyCommand, SetActiveCompanyUseCase,
  UpdateCompanyProfileCommand, UpdateCompanyProfileUseCase,
};
use crate::domain::auth::entities::User;
use crate::domain::auth::ports::UserRepository;
use crate::domain::company::ports::CompanyMemberRepository;

/// Helper function to extract authenticated user from request
fn get_user(req: &HttpRequest) -> Result<User, ApiError> {
  match req.extensions().get::<User>() {
    Some(user) => {
      tracing::debug!("User found in request extensions: {}", user.id);
      Ok(user.clone())
    }
    None => {
      tracing::error!("No user found in request extensions - middleware may not have run");
      Err(ApiError::Auth(
        crate::adapters::http::errors::AuthErrorKind::InvalidSession,
      ))
    }
  }
}

#[derive(Deserialize)]
pub struct DropdownQuery {
  pub company_id: Option<Uuid>,
  pub current_page: Option<String>,
}

/// GET /companies/dropdown - Returns company list dropdown HTML
pub async fn company_dropdown_handler(
  req: HttpRequest,
  query: web::Query<DropdownQuery>,
  templates: web::Data<TemplateEngine>,
  get_companies_use_case: web::Data<Arc<GetUserCompaniesUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;

  // Fetch user's companies
  let companies_response = get_companies_use_case
    .execute(GetUserCompaniesCommand { user_id: user.id })
    .await?;

  // Mark the active company based on company_id from query
  let companies_with_active: Vec<serde_json::Value> = companies_response
    .companies
    .iter()
    .map(|c| {
      serde_json::json!({
        "company_id": c.company_id,
        "name": c.name,
        "role": c.role,
        "is_active": query.company_id == Some(c.company_id),
      })
    })
    .collect();

  let mut context = tera::Context::new();
  context.insert("companies", &companies_with_active);
  context.insert(
    "current_page",
    &query.current_page.as_deref().unwrap_or("dashboard"),
  );

  let html = templates
    .render("partials/company_list_dropdown.html.tera", &context)
    .map_err(|e| ApiError::Internal(format!("Template error: {}", e)))?;

  Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

#[derive(Deserialize)]
pub struct CreateCompanyFormData {
  pub name: String,
}

/// POST /companies/create - Creates a new company
pub async fn create_company_submit(
  req: HttpRequest,
  form: web::Form<CreateCompanyFormData>,
  templates: web::Data<TemplateEngine>,
  create_company_use_case: web::Data<Arc<CreateCompanyUseCase>>,
) -> Result<HttpResponse, ApiError> {
  // Get user from request (with logging)
  let user = match get_user(&req) {
    Ok(u) => u,
    Err(e) => {
      tracing::error!("Failed to get user in create_company_submit: {:?}", e);
      return Err(e);
    }
  };

  tracing::info!("User {} creating company: {}", user.id, form.name);

  // Validate name
  let name = form.name.trim();
  if name.is_empty() {
    tracing::warn!("Company creation failed: empty name");
    let mut context = tera::Context::new();
    context.insert("error", "Company name is required");
    context.insert("name", &form.name);

    let html = templates
      .render("partials/create_company_form.html.tera", &context)
      .map_err(|e| {
        tracing::error!("Template render error: {}", e);
        ApiError::Internal(format!("Template error: {}", e))
      })?;

    return Ok(
      HttpResponse::BadRequest()
        .content_type("text/html")
        .body(html),
    );
  }

  if name.len() > 255 {
    tracing::warn!("Company creation failed: name too long ({})", name.len());
    let mut context = tera::Context::new();
    context.insert("error", "Company name must be 255 characters or less");
    context.insert("name", &form.name);

    let html = templates
      .render("partials/create_company_form.html.tera", &context)
      .map_err(|e| {
        tracing::error!("Template render error: {}", e);
        ApiError::Internal(format!("Template error: {}", e))
      })?;

    return Ok(
      HttpResponse::BadRequest()
        .content_type("text/html")
        .body(html),
    );
  }

  // Execute use case
  match create_company_use_case
    .execute(CreateCompanyCommand {
      owner_id: user.id,
      name: name.to_string(),
    })
    .await
  {
    Ok(response) => {
      tracing::info!(
        "Company created successfully: {} ({})",
        response.company_id,
        response.name
      );
      // Success - redirect to companies page
      Ok(
        HttpResponse::Ok()
          .insert_header(("HX-Redirect", "/companies"))
          .finish(),
      )
    }
    Err(e) => {
      tracing::error!("Failed to create company: {:?}", e);
      // Error - re-render form with error message
      let mut context = tera::Context::new();
      context.insert("error", &format!("Failed to create company: {}", e));
      context.insert("name", &form.name);

      let html = templates
        .render("partials/create_company_form.html.tera", &context)
        .map_err(|e| {
          tracing::error!("Template render error: {}", e);
          ApiError::Internal(format!("Template error: {}", e))
        })?;

      Ok(
        HttpResponse::BadRequest()
          .content_type("text/html")
          .body(html),
      )
    }
  }
}

/// GET /companies - Companies list page
pub async fn companies_page(
  req: HttpRequest,
  templates: web::Data<TemplateEngine>,
  get_companies_use_case: web::Data<Arc<GetUserCompaniesUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;

  // Fetch companies (already includes is_active flag)
  let companies_response = get_companies_use_case
    .execute(GetUserCompaniesCommand { user_id: user.id })
    .await?;

  let active_company = companies_response
    .companies
    .iter()
    .find(|c| c.is_active)
    .map(|c| serde_json::json!({ "id": c.company_id, "name": c.name }));

  // Extract active company_id for navbar links
  let active_company_id = companies_response
    .companies
    .iter()
    .find(|c| c.is_active)
    .map(|c| c.company_id);

  let mut context = tera::Context::new();
  context.insert("user", &user);
  context.insert("companies", &companies_response.companies);
  context.insert("active_company", &active_company);
  if let Some(company_id) = active_company_id {
    context.insert("company_id", &company_id.to_string());
  }

  let html = templates
    .render("pages/companies.html.tera", &context)
    .map_err(|e| ApiError::Internal(format!("Template error: {}", e)))?;

  Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

/// POST /companies/:id/set-active - Sets active company
pub async fn set_active_company_handler(
  req: HttpRequest,
  company_id: web::Path<Uuid>,
  set_active_use_case: web::Data<Arc<SetActiveCompanyUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;

  set_active_use_case
    .execute(SetActiveCompanyCommand {
      user_id: user.id,
      company_id: *company_id,
    })
    .await?;

  Ok(HttpResponse::Ok().finish())
}

/// GET /companies/:id/members - Company members page
pub async fn company_members_page(
  req: HttpRequest,
  company_id: web::Path<Uuid>,
  templates: web::Data<TemplateEngine>,
  get_companies_use_case: web::Data<Arc<GetUserCompaniesUseCase>>,
  member_repo: web::Data<Arc<dyn CompanyMemberRepository>>,
  user_repo: web::Data<Arc<dyn UserRepository>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;

  // Fetch user's companies to verify access and get navbar data
  let companies_response = get_companies_use_case
    .execute(GetUserCompaniesCommand { user_id: user.id })
    .await?;

  // Find the requested company
  let company = companies_response
    .companies
    .iter()
    .find(|c| c.company_id == *company_id)
    .ok_or_else(|| ApiError::Internal("Company not found or access denied".to_string()))?;

  // Fetch company members
  let members_raw = member_repo
    .find_by_company_id(*company_id)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to fetch members: {}", e)))?;

  // Fetch user details for each member
  let mut members = Vec::new();
  for member in members_raw {
    if let Ok(Some(member_user)) = user_repo.find_by_id(member.user_id).await {
      members.push(serde_json::json!({
          "user_id": member.user_id,
          "email": member_user.email,
          "full_name": member_user.full_name,
          "role": member.role,
          "joined_at": member.joined_at.format("%Y-%m-%d").to_string(),
      }));
    }
  }

  // Check if user can manage members (owner or admin)
  let can_manage_members = company.role == "owner" || company.role == "admin";

  // Get active company for navbar
  let active_company = companies_response
    .companies
    .iter()
    .find(|c| c.is_active)
    .map(|c| serde_json::json!({ "id": c.company_id, "name": c.name }));

  let mut context = tera::Context::new();
  context.insert("user", &user);
  context.insert("companies", &companies_response.companies);
  context.insert("active_company", &active_company);
  context.insert(
    "company",
    &serde_json::json!({
        "company_id": company.company_id,
        "name": company.name,
    }),
  );
  context.insert("members", &members);
  context.insert("can_manage_members", &can_manage_members);

  let html = templates
    .render("pages/company_members.html.tera", &context)
    .map_err(|e| {
      tracing::error!("Template render error: {:?}", e);
      ApiError::Internal(format!("Template error: {}", e))
    })?;

  Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

#[derive(Deserialize)]
pub struct AddMemberFormData {
  pub email: String,
  pub role: String,
}

/// POST /companies/:id/members/add - Add member to company
pub async fn add_member_submit(
  req: HttpRequest,
  company_id: web::Path<Uuid>,
  form: web::Form<AddMemberFormData>,
  add_member_use_case: web::Data<Arc<AddCompanyMemberUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;

  // Execute use case
  add_member_use_case
    .execute(AddCompanyMemberCommand {
      company_id: *company_id,
      member_email: form.email.clone(),
      role: form.role.clone(),
      requester_id: user.id,
    })
    .await?;

  // Success - trigger page reload
  Ok(HttpResponse::Ok().finish())
}

/// DELETE /companies/:company_id/members/:user_id - Remove member from company
pub async fn remove_member_handler(
  req: HttpRequest,
  path: web::Path<(Uuid, Uuid)>,
  remove_member_use_case: web::Data<Arc<RemoveCompanyMemberUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let (company_id, member_user_id) = path.into_inner();

  // Execute use case
  remove_member_use_case
    .execute(RemoveCompanyMemberCommand {
      company_id,
      member_id: member_user_id,
      requester_id: user.id,
    })
    .await?;

  // Return empty 200 (HTMX will remove the row)
  Ok(HttpResponse::Ok().finish())
}

/// GET /companies/:id/settings - Company settings page
pub async fn company_settings_page(
  req: HttpRequest,
  company_id: web::Path<Uuid>,
  query: web::Query<std::collections::HashMap<String, String>>,
  templates: web::Data<TemplateEngine>,
  get_details_use_case: web::Data<Arc<GetCompanyDetailsUseCase>>,
  get_companies_use_case: web::Data<Arc<GetUserCompaniesUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;

  // Fetch company details
  let company_details = get_details_use_case
    .execute(GetCompanyDetailsCommand {
      company_id: *company_id,
      requester_id: user.id,
    })
    .await?;

  // Fetch all companies for navbar
  let companies_response = get_companies_use_case
    .execute(GetUserCompaniesCommand { user_id: user.id })
    .await?;

  let active_company = companies_response
    .companies
    .iter()
    .find(|c| c.is_active)
    .map(|c| serde_json::json!({ "id": c.company_id, "name": c.name }));

  let mut context = tera::Context::new();
  context.insert("user", &user);
  context.insert("companies", &companies_response.companies);
  context.insert("active_company", &active_company);
  context.insert("company", &company_details);

  // Check for success parameter
  let success = query.get("success").map(|s| s == "true").unwrap_or(false);
  context.insert("success", &success);

  let html = templates
    .render("pages/company_settings.html.tera", &context)
    .map_err(|e| {
      tracing::error!("Template render error: {:?}", e);
      ApiError::Internal(format!("Template error: {}", e))
    })?;

  Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

#[derive(Deserialize)]
pub struct UpdateCompanySettingsFormData {
  pub email: Option<String>,
  pub phone: Option<String>,
  pub street: Option<String>,
  pub city: Option<String>,
  pub state: Option<String>,
  pub postal_code: Option<String>,
  pub country: Option<String>,
  pub registry_code: Option<String>,
  pub vat_number: Option<String>,
}

/// POST /companies/:id/settings - Update company settings
pub async fn update_company_settings_submit(
  req: HttpRequest,
  company_id: web::Path<Uuid>,
  form: web::Form<UpdateCompanySettingsFormData>,
  templates: web::Data<TemplateEngine>,
  update_use_case: web::Data<Arc<UpdateCompanyProfileUseCase>>,
  get_details_use_case: web::Data<Arc<GetCompanyDetailsUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;

  // Build address data
  let address = if form.street.is_some()
    || form.city.is_some()
    || form.state.is_some()
    || form.postal_code.is_some()
    || form.country.is_some()
  {
    Some(CompanyAddressData {
      street: form.street.clone(),
      city: form.city.clone(),
      state: form.state.clone(),
      postal_code: form.postal_code.clone(),
      country: form.country.clone(),
    })
  } else {
    None
  };

  // Execute update
  match update_use_case
    .execute(UpdateCompanyProfileCommand {
      company_id: *company_id,
      requester_id: user.id,
      email: form.email.clone(),
      phone: form.phone.clone(),
      address,
      registry_code: form.registry_code.clone(),
      vat_number: form.vat_number.clone(),
    })
    .await
  {
    Ok(_) => {
      tracing::info!("Company settings updated successfully: {}", company_id);
      // Success - redirect to settings page with success parameter
      Ok(
        HttpResponse::Ok()
          .insert_header((
            "HX-Redirect",
            format!("/companies/{}/settings?success=true", company_id),
          ))
          .finish(),
      )
    }
    Err(e) => {
      tracing::error!("Failed to update company settings: {:?}", e);

      // Re-fetch company details to get current state
      let company_details = get_details_use_case
        .execute(GetCompanyDetailsCommand {
          company_id: *company_id,
          requester_id: user.id,
        })
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to fetch company details: {}", e)))?;

      // Error - re-render form with error message and preserve form values
      let mut context = tera::Context::new();
      context.insert("error", &format!("{}", e));
      context.insert("company", &company_details);

      // Preserve form values
      if let Some(email) = &form.email {
        context.insert("form_email", email);
      }
      if let Some(phone) = &form.phone {
        context.insert("form_phone", phone);
      }

      let html = templates
        .render("partials/company_settings_form.html.tera", &context)
        .map_err(|e| {
          tracing::error!("Template render error: {}", e);
          ApiError::Internal(format!("Template error: {}", e))
        })?;

      Ok(
        HttpResponse::BadRequest()
          .content_type("text/html")
          .body(html),
      )
    }
  }
}
