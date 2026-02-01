-- Add VAT number field to companies table
ALTER TABLE companies ADD COLUMN vat_number VARCHAR(50);

-- Create index for VAT number lookups
CREATE INDEX idx_companies_vat_number ON companies(vat_number)
WHERE vat_number IS NOT NULL;
