use std::sync::Arc;
use uuid::Uuid;

use crate::domain::company::{CompanyError, CompanyRepository};

/// Command to test Drive connection
pub struct TestDriveConnectionCommand {
  pub company_id: Uuid,
}

/// Response from connection test
pub struct TestDriveConnectionResponse {
  pub success: bool,
  pub message: String,
}

/// Use case for testing Google Drive connection
pub struct TestDriveConnectionUseCase {
  company_repo: Arc<dyn CompanyRepository>,
}

impl TestDriveConnectionUseCase {
  pub fn new(company_repo: Arc<dyn CompanyRepository>) -> Self {
    Self { company_repo }
  }

  /// Test the OAuth connection
  pub async fn execute(
    &self,
    cmd: TestDriveConnectionCommand,
  ) -> Result<TestDriveConnectionResponse, CompanyError> {
    let company = self
      .company_repo
      .find_by_id(cmd.company_id)
      .await?
      .ok_or(CompanyError::NotFound)?;

    // Check if OAuth is connected
    if !company.has_oauth_connection() {
      return Ok(TestDriveConnectionResponse {
        success: false,
        message: "Google Drive is not connected".to_string(),
      });
    }

    // Check if token is expired
    if !company.has_valid_oauth_token() {
      return Ok(TestDriveConnectionResponse {
        success: false,
        message: "OAuth tokens have expired. Please reconnect.".to_string(),
      });
    }

    // TODO: Actually test the connection by making a Drive API call
    // For now, just check token validity
    Ok(TestDriveConnectionResponse {
      success: true,
      message: "Connection is active and tokens are valid".to_string(),
    })
  }
}
