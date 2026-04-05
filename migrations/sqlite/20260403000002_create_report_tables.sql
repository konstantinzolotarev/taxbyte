-- Add reports_folder_id to companies
ALTER TABLE companies ADD COLUMN reports_folder_id TEXT;

-- Monthly reports table
CREATE TABLE IF NOT EXISTS monthly_reports (
    id TEXT PRIMARY KEY NOT NULL,
    company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    month INTEGER NOT NULL,
    year INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'draft',
    bank_account_iban TEXT NOT NULL,
    total_incoming TEXT NOT NULL DEFAULT '0',
    total_outgoing TEXT NOT NULL DEFAULT '0',
    transaction_count INTEGER NOT NULL DEFAULT 0,
    matched_count INTEGER NOT NULL DEFAULT 0,
    drive_folder_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CONSTRAINT monthly_reports_company_period_unique UNIQUE (company_id, month, year)
);

CREATE INDEX IF NOT EXISTS idx_monthly_reports_company_id ON monthly_reports(company_id);
CREATE INDEX IF NOT EXISTS idx_monthly_reports_period ON monthly_reports(year, month);

-- Received invoices table (vendor bills)
CREATE TABLE IF NOT EXISTS received_invoices (
    id TEXT PRIMARY KEY NOT NULL,
    company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    vendor_name TEXT NOT NULL,
    amount TEXT NOT NULL,
    currency TEXT NOT NULL DEFAULT 'EUR',
    invoice_date TEXT,
    invoice_number TEXT,
    pdf_path TEXT NOT NULL,
    pdf_drive_file_id TEXT,
    notes TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_received_invoices_company_id ON received_invoices(company_id);
CREATE INDEX IF NOT EXISTS idx_received_invoices_date ON received_invoices(invoice_date);

-- Bank transactions table
CREATE TABLE IF NOT EXISTS bank_transactions (
    id TEXT PRIMARY KEY NOT NULL,
    report_id TEXT NOT NULL REFERENCES monthly_reports(id) ON DELETE CASCADE,
    row_number INTEGER NOT NULL,
    date TEXT NOT NULL,
    counterparty_name TEXT,
    counterparty_account TEXT,
    direction TEXT NOT NULL,
    amount TEXT NOT NULL,
    reference_number TEXT,
    description TEXT,
    currency TEXT NOT NULL DEFAULT 'EUR',
    registry_code TEXT,
    matched_invoice_id TEXT REFERENCES invoices(id) ON DELETE SET NULL,
    matched_received_invoice_id TEXT REFERENCES received_invoices(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL,
    CONSTRAINT bank_tx_at_most_one_match CHECK (
        NOT (matched_invoice_id IS NOT NULL AND matched_received_invoice_id IS NOT NULL)
    )
);

CREATE INDEX IF NOT EXISTS idx_bank_transactions_report_id ON bank_transactions(report_id);
CREATE INDEX IF NOT EXISTS idx_bank_transactions_date ON bank_transactions(date);
CREATE INDEX IF NOT EXISTS idx_bank_transactions_matched_invoice ON bank_transactions(matched_invoice_id) WHERE matched_invoice_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_bank_transactions_matched_received ON bank_transactions(matched_received_invoice_id) WHERE matched_received_invoice_id IS NOT NULL;
