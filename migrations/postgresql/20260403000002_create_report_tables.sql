-- Add reports_folder_id to companies
ALTER TABLE companies ADD COLUMN IF NOT EXISTS reports_folder_id TEXT;

-- Monthly reports table
CREATE TABLE IF NOT EXISTS monthly_reports (
    id UUID PRIMARY KEY,
    company_id UUID NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    month INTEGER NOT NULL,
    year INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'draft',
    bank_account_iban TEXT NOT NULL,
    total_incoming DECIMAL(12,2) NOT NULL DEFAULT 0,
    total_outgoing DECIMAL(12,2) NOT NULL DEFAULT 0,
    transaction_count INTEGER NOT NULL DEFAULT 0,
    matched_count INTEGER NOT NULL DEFAULT 0,
    drive_folder_id TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT monthly_reports_company_period_unique UNIQUE (company_id, month, year)
);

CREATE INDEX IF NOT EXISTS idx_monthly_reports_company_id ON monthly_reports(company_id);
CREATE INDEX IF NOT EXISTS idx_monthly_reports_period ON monthly_reports(year, month);

-- Received invoices table (vendor bills)
CREATE TABLE IF NOT EXISTS received_invoices (
    id UUID PRIMARY KEY,
    company_id UUID NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    vendor_name TEXT NOT NULL,
    amount DECIMAL(12,2) NOT NULL,
    currency TEXT NOT NULL DEFAULT 'EUR',
    invoice_date DATE,
    invoice_number TEXT,
    pdf_path TEXT NOT NULL,
    pdf_drive_file_id TEXT,
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_received_invoices_company_id ON received_invoices(company_id);
CREATE INDEX IF NOT EXISTS idx_received_invoices_date ON received_invoices(invoice_date);

-- Bank transactions table
CREATE TABLE IF NOT EXISTS bank_transactions (
    id UUID PRIMARY KEY,
    report_id UUID NOT NULL REFERENCES monthly_reports(id) ON DELETE CASCADE,
    row_number INTEGER NOT NULL,
    date DATE NOT NULL,
    counterparty_name TEXT,
    counterparty_account TEXT,
    direction TEXT NOT NULL,
    amount DECIMAL(12,2) NOT NULL,
    reference_number TEXT,
    description TEXT,
    currency TEXT NOT NULL DEFAULT 'EUR',
    registry_code TEXT,
    matched_invoice_id UUID REFERENCES invoices(id) ON DELETE SET NULL,
    matched_received_invoice_id UUID REFERENCES received_invoices(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT bank_tx_at_most_one_match CHECK (
        NOT (matched_invoice_id IS NOT NULL AND matched_received_invoice_id IS NOT NULL)
    )
);

CREATE INDEX IF NOT EXISTS idx_bank_transactions_report_id ON bank_transactions(report_id);
CREATE INDEX IF NOT EXISTS idx_bank_transactions_date ON bank_transactions(date);
CREATE INDEX IF NOT EXISTS idx_bank_transactions_matched_invoice ON bank_transactions(matched_invoice_id) WHERE matched_invoice_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_bank_transactions_matched_received ON bank_transactions(matched_received_invoice_id) WHERE matched_received_invoice_id IS NOT NULL;
