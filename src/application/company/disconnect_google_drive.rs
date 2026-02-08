use std::sync::Arc;
use uuid::Uuid;

use crate::domain::company::{CompanyError, CompanyRepository};

/// Command to disconnect Google Drive
pub struct DisconnectGoogleDriveCommand {
  pub company_id: Uuid,
  pub user_id: Uuid,
}

/// Use case for disconnecting Google Drive
pub struct DisconnectGoogleDriveUseCase {
  company_repo: Arc<dyn CompanyRepository>,
}

impl DisconnectGoogleDriveUseCase {
  pub fn new(company_repo: Arc<dyn CompanyRepository>) -> Self {
    Self { company_repo }
  }

  /// Execute disconnect - remove OAuth tokens
  pub async fn execute(&self, cmd: DisconnectGoogleDriveCommand) -> Result<(), CompanyError> {
    // Verify company exists
    let _company = self
      .company_repo
      .find_by_id(cmd.company_id)
      .await?
      .ok_or(CompanyError::NotFound)?;

    // TODO: Verify user has owner/admin role for this company

    // Clear OAuth tokens (hard delete)
    self
      .company_repo
      .clear_oauth_tokens(&cmd.company_id)
      .await?;

    Ok(())
  }
}
