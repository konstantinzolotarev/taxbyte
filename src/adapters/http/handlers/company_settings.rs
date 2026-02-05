use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::adapters::http::errors::ApiError;
use crate::adapters::http::templates::TemplateEngine;
use crate::application::company::{
  CompanyAddressData, GetCompanyDetailsCommand, GetCompanyDetailsUseCase,
  UpdateCompanyProfileCommand, UpdateCompanyProfileUseCase, UpdateStorageConfigCommand,
  UpdateStorageConfigUseCase,
};
use crate::domain::auth::entities::User;

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

/// GET /companies/:id/settings - Company settings page
pub async fn company_settings_page(
  req: HttpRequest,
  path: web::Path<Uuid>,
  templates: web::Data<TemplateEngine>,
  get_company_details: web::Data<Arc<GetCompanyDetailsUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let company_id = path.into_inner();

  // Get company details
  let company_details = get_company_details
    .execute(GetCompanyDetailsCommand {
      requester_id: user.id,
      company_id,
    })
    .await?;

  let mut context = tera::Context::new();
  context.insert("user", &user);
  context.insert("company", &company_details);
  context.insert("current_page", "settings");

  let html = templates
    .render("pages/company_settings.html.tera", &context)
    .map_err(|e| ApiError::Internal(format!("Template error: {}", e)))?;

  Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

#[derive(Debug, Deserialize)]
pub struct UpdateStorageConfigForm {
  pub storage_provider: String,
  pub google_drive_key: Option<String>,
  pub google_drive_folder_id: Option<String>,
  pub s3_bucket: Option<String>,
  pub s3_region: Option<String>,
  pub s3_access_key: Option<String>,
  pub s3_secret_key: Option<String>,
  pub s3_prefix: Option<String>,
}

/// POST /companies/:id/settings/storage - Update storage configuration
pub async fn update_storage_config(
  req: HttpRequest,
  path: web::Path<Uuid>,
  form: web::Form<UpdateStorageConfigForm>,
  update_storage_use_case: web::Data<Arc<UpdateStorageConfigUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let company_id = path.into_inner();

  // Build storage config JSON based on provider
  let storage_config_json = match form.storage_provider.as_str() {
    "google_drive" => {
      if let Some(key) = &form.google_drive_key {
        Some(
          serde_json::json!({
            "provider": "google_drive",
            "service_account_key": key,
            "parent_folder_id": form.google_drive_folder_id,
            "folder_path": "Invoices"
          })
          .to_string(),
        )
      } else {
        return Err(ApiError::Validation(
          "Google Drive service account key is required".to_string(),
        ));
      }
    }
    "s3" => {
      if let (Some(bucket), Some(region), Some(access_key), Some(secret_key)) = (
        &form.s3_bucket,
        &form.s3_region,
        &form.s3_access_key,
        &form.s3_secret_key,
      ) {
        Some(
          serde_json::json!({
            "provider": "s3",
            "bucket": bucket,
            "region": region,
            "access_key_id": access_key,
            "secret_access_key": secret_key,
            "prefix": form.s3_prefix.as_deref().unwrap_or("")
          })
          .to_string(),
        )
      } else {
        return Err(ApiError::Validation(
          "S3 bucket, region, and credentials are required".to_string(),
        ));
      }
    }
    "none" => None,
    _ => {
      return Err(ApiError::Validation(format!(
        "Invalid storage provider: {}",
        form.storage_provider
      )));
    }
  };

  // Update storage configuration
  update_storage_use_case
    .execute(UpdateStorageConfigCommand {
      user_id: user.id,
      company_id,
      storage_provider: form.storage_provider.clone(),
      storage_config_json,
    })
    .await?;

  // Redirect back to settings page with success message
  Ok(
    HttpResponse::SeeOther()
      .insert_header(("Location", format!("/companies/{}/settings", company_id)))
      .finish(),
  )
}

#[derive(Debug, Deserialize)]
pub struct UpdateCompanyProfileForm {
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

/// POST /companies/:id/settings - Update company profile
pub async fn update_company_profile(
  req: HttpRequest,
  path: web::Path<Uuid>,
  form: web::Form<UpdateCompanyProfileForm>,
  templates: web::Data<TemplateEngine>,
  update_profile_use_case: web::Data<Arc<UpdateCompanyProfileUseCase>>,
  get_company_details: web::Data<Arc<GetCompanyDetailsUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let company_id = path.into_inner();

  // Build address data if any field is provided
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

  // Update profile
  let result = update_profile_use_case
    .execute(UpdateCompanyProfileCommand {
      company_id,
      requester_id: user.id,
      email: form.email.clone(),
      phone: form.phone.clone(),
      address,
      registry_code: form.registry_code.clone(),
      vat_number: form.vat_number.clone(),
    })
    .await;

  match result {
    Ok(_) => {
      // On success, get updated company details and re-render the form
      let company_details = get_company_details
        .execute(GetCompanyDetailsCommand {
          requester_id: user.id,
          company_id,
        })
        .await?;

      let mut context = tera::Context::new();
      context.insert("company", &company_details);
      context.insert("success", &"Profile updated successfully!");

      let html = templates
        .render("partials/company_settings_form.html.tera", &context)
        .map_err(|e| ApiError::Internal(format!("Template error: {}", e)))?;

      Ok(HttpResponse::Ok().content_type("text/html").body(html))
    }
    Err(e) => {
      // On error, re-render the form with error message and form values
      let company_details = get_company_details
        .execute(GetCompanyDetailsCommand {
          requester_id: user.id,
          company_id,
        })
        .await?;

      let mut context = tera::Context::new();
      context.insert("company", &company_details);
      context.insert("error", &e.to_string());
      // Preserve form values
      context.insert("form_email", &form.email);
      context.insert("form_phone", &form.phone);

      let html = templates
        .render("partials/company_settings_form.html.tera", &context)
        .map_err(|e| ApiError::Internal(format!("Template error: {}", e)))?;

      Ok(
        HttpResponse::BadRequest()
          .content_type("text/html")
          .body(html),
      )
    }
  }
}
