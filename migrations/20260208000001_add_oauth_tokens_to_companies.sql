-- Add OAuth token fields to companies table for Google Drive integration
-- These fields store encrypted OAuth 2.0 tokens for user consent flow

ALTER TABLE companies
  ADD COLUMN oauth_access_token TEXT,
  ADD COLUMN oauth_refresh_token TEXT,
  ADD COLUMN oauth_token_expires_at TIMESTAMPTZ,
  ADD COLUMN oauth_connected_by UUID REFERENCES users(id) ON DELETE SET NULL,
  ADD COLUMN oauth_connected_at TIMESTAMPTZ;

-- Create index for token lookups (only on non-null tokens)
CREATE INDEX idx_companies_oauth_tokens
  ON companies(oauth_refresh_token)
  WHERE oauth_refresh_token IS NOT NULL;

-- Add comment for clarity
COMMENT ON COLUMN companies.oauth_access_token IS 'Encrypted OAuth 2.0 access token for Google Drive API';
COMMENT ON COLUMN companies.oauth_refresh_token IS 'Encrypted OAuth 2.0 refresh token for Google Drive API';
COMMENT ON COLUMN companies.oauth_token_expires_at IS 'Expiration timestamp for the OAuth access token';
COMMENT ON COLUMN companies.oauth_connected_by IS 'User who connected the Google Drive account (owner/admin)';
COMMENT ON COLUMN companies.oauth_connected_at IS 'Timestamp when Google Drive was connected';
