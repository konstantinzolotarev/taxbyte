-- Drop invoice_folder_path column (replaced by google_drive_folder_id)
-- Must drop index first since it references the column
DROP INDEX IF EXISTS idx_companies_invoice_folder_path;
ALTER TABLE companies DROP COLUMN invoice_folder_path;
