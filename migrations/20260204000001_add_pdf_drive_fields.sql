-- Add Google Drive file ID field to invoices
ALTER TABLE invoices
ADD COLUMN pdf_drive_file_id VARCHAR(255);

-- Add invoice folder path configuration to companies table
-- Allows each company to have custom invoice subfolder (e.g., "Invoices", "Documents/Invoices")
-- NULL means use default from config (default: "Invoices")
ALTER TABLE companies
ADD COLUMN invoice_folder_path VARCHAR(255);

-- Optional: Cache Drive folder IDs to avoid repeated API calls
-- This is the ID of the final folder where invoices are uploaded
ALTER TABLE companies
ADD COLUMN google_drive_folder_id VARCHAR(255);

-- Indexes
CREATE INDEX idx_invoices_pdf_drive_file_id ON invoices(pdf_drive_file_id);
CREATE INDEX idx_companies_invoice_folder_path ON companies(invoice_folder_path);
