use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::company::{CompanyError, CompanyService, StorageProvider};

#[derive(Debug, Deserialize)]
pub struct UpdateStorageConfigCommand {
  pub user_id: Uuid,
  pub company_id: Uuid,
  pub storage_provider: String,
  pub storage_config_json: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UpdateStorageConfigResponse {
  pub company_id: Uuid,
  pub storage_provider: String,
  pub message: String,
}

pub struct UpdateStorageConfigUseCase {
  company_service: Arc<CompanyService>,
}

impl UpdateStorageConfigUseCase {
  pub fn new(company_service: Arc<CompanyService>) -> Self {
    Self { company_service }
  }

  pub async fn execute(
    &self,
    command: UpdateStorageConfigCommand,
  ) -> Result<UpdateStorageConfigResponse, CompanyError> {
    // Parse and validate storage provider
    let provider = command
      .storage_provider
      .parse::<StorageProvider>()
      .map_err(|e| {
        CompanyError::Validation(crate::domain::company::ValidationError::InvalidFormat(e))
      })?;

    // Validate storage config JSON if provided
    if let Some(ref config_json) = command.storage_config_json {
      // Try to parse as JSON to validate format
      serde_json::from_str::<serde_json::Value>(config_json).map_err(|e| {
        CompanyError::Validation(crate::domain::company::ValidationError::InvalidFormat(
          format!("Invalid JSON configuration: {}", e),
        ))
      })?;
    }

    // Update storage configuration through service
    let updated_company = self
      .company_service
      .update_storage_config(
        command.company_id,
        command.user_id,
        Some(provider.as_str().to_string()),
        command.storage_config_json,
      )
      .await?;

    Ok(UpdateStorageConfigResponse {
      company_id: updated_company.id,
      storage_provider: updated_company
        .storage_provider
        .unwrap_or_else(|| "none".to_string()),
      message: "Storage configuration updated successfully".to_string(),
    })
  }
}
