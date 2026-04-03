-- Add storage provider configuration to companies table
ALTER TABLE companies
ADD COLUMN storage_provider VARCHAR(50) DEFAULT 'none',
ADD COLUMN storage_config TEXT;

-- Add index for storage provider lookups
CREATE INDEX idx_companies_storage_provider ON companies(storage_provider);

-- Add comment
COMMENT ON COLUMN companies.storage_provider IS 'Cloud storage provider type: none, google_drive, s3';
COMMENT ON COLUMN companies.storage_config IS 'JSON configuration for the storage provider';
