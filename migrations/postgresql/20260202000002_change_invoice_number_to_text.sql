-- Migration: Change invoice_number from INTEGER to VARCHAR for manual editing

-- Step 1: Add new text column
ALTER TABLE invoices ADD COLUMN IF NOT EXISTS invoice_number_text VARCHAR(100);

-- Step 2: Copy existing invoice numbers (format as INV-XXXX)
UPDATE invoices
SET invoice_number_text = 'INV-' || LPAD(invoice_number::text, 4, '0')
WHERE invoice_number_text IS NULL;

-- Step 3: Make the new column NOT NULL
ALTER TABLE invoices ALTER COLUMN invoice_number_text SET NOT NULL;

-- Step 4: Drop old constraints that reference invoice_number
ALTER TABLE invoices DROP CONSTRAINT IF EXISTS invoices_company_number_unique;
ALTER TABLE invoices DROP CONSTRAINT IF EXISTS invoices_invoice_number_positive;

-- Step 5: Drop old column (this will automatically drop idx_invoices_invoice_number)
ALTER TABLE invoices DROP COLUMN IF EXISTS invoice_number;

-- Step 6: Rename new column to invoice_number
ALTER TABLE invoices RENAME COLUMN invoice_number_text TO invoice_number;

-- Step 7: Add unique constraint back (company_id + invoice_number)
ALTER TABLE invoices ADD CONSTRAINT invoices_company_number_unique UNIQUE (company_id, invoice_number);

-- Step 8: Recreate index (no need to drop first since it was auto-dropped with column)
CREATE INDEX IF NOT EXISTS idx_invoices_invoice_number ON invoices(company_id, invoice_number);
