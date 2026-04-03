-- Drop invoice_folder_path column (replaced by google_drive_folder_id)
DROP INDEX IF EXISTS idx_companies_invoice_folder_path;
ALTER TABLE companies DROP COLUMN IF EXISTS invoice_folder_path;
