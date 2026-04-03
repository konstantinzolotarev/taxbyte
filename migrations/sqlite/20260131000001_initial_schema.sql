-- Consolidated SQLite schema for TaxByte
-- Equivalent to all PostgreSQL migrations combined

-- Users table
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY NOT NULL,
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    full_name TEXT NOT NULL,
    is_email_verified INTEGER NOT NULL DEFAULT 0,
    email_verification_token TEXT,
    email_verification_token_expires_at TEXT,
    password_reset_token TEXT,
    password_reset_token_expires_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_email_verification_token ON users(email_verification_token) WHERE email_verification_token IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_users_password_reset_token ON users(password_reset_token) WHERE password_reset_token IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_users_deleted_at ON users(deleted_at) WHERE deleted_at IS NOT NULL;

-- Sessions table
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_token TEXT UNIQUE NOT NULL,
    ip_address TEXT,
    user_agent TEXT,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sessions_session_token ON sessions(session_token);
CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at);

-- Login attempts table
CREATE TABLE IF NOT EXISTS login_attempts (
    id TEXT PRIMARY KEY NOT NULL,
    email TEXT NOT NULL,
    ip_address TEXT NOT NULL,
    success INTEGER NOT NULL,
    attempted_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_login_attempts_email_attempted_at ON login_attempts(email, attempted_at);
CREATE INDEX IF NOT EXISTS idx_login_attempts_ip_attempted_at ON login_attempts(ip_address, attempted_at);

-- Companies table
CREATE TABLE IF NOT EXISTS companies (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    email TEXT,
    phone TEXT,
    address TEXT,
    tax_id TEXT,
    vat_number TEXT,
    invoice_folder_path TEXT,
    google_drive_folder_id TEXT,
    storage_provider TEXT DEFAULT 'none',
    storage_config TEXT,
    oauth_access_token TEXT,
    oauth_refresh_token TEXT,
    oauth_token_expires_at TEXT,
    oauth_connected_by TEXT REFERENCES users(id) ON DELETE SET NULL,
    oauth_connected_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_companies_name ON companies(name);
CREATE INDEX IF NOT EXISTS idx_companies_tax_id ON companies(tax_id) WHERE tax_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_companies_vat_number ON companies(vat_number) WHERE vat_number IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_companies_storage_provider ON companies(storage_provider);
CREATE INDEX IF NOT EXISTS idx_companies_invoice_folder_path ON companies(invoice_folder_path);
CREATE INDEX IF NOT EXISTS idx_companies_oauth_tokens ON companies(oauth_refresh_token) WHERE oauth_refresh_token IS NOT NULL;

-- Company members table
CREATE TABLE IF NOT EXISTS company_members (
    id TEXT PRIMARY KEY NOT NULL,
    company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(company_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_company_members_company_id ON company_members(company_id);
CREATE INDEX IF NOT EXISTS idx_company_members_user_id ON company_members(user_id);
CREATE INDEX IF NOT EXISTS idx_company_members_role ON company_members(role);

-- Active companies table
CREATE TABLE IF NOT EXISTS active_companies (
    user_id TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    set_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_active_companies_company_id ON active_companies(company_id);

-- Bank accounts table
CREATE TABLE IF NOT EXISTS bank_accounts (
    id TEXT PRIMARY KEY NOT NULL,
    company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    iban TEXT NOT NULL,
    bank_details TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    archived_at TEXT,
    CONSTRAINT bank_accounts_company_iban_unique UNIQUE (company_id, iban)
);

CREATE INDEX IF NOT EXISTS idx_bank_accounts_company_id ON bank_accounts(company_id);
CREATE INDEX IF NOT EXISTS idx_bank_accounts_archived_at ON bank_accounts(archived_at) WHERE archived_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_bank_accounts_iban ON bank_accounts(iban);

-- Active bank accounts table
CREATE TABLE IF NOT EXISTS active_bank_accounts (
    company_id TEXT PRIMARY KEY REFERENCES companies(id) ON DELETE CASCADE,
    bank_account_id TEXT NOT NULL REFERENCES bank_accounts(id) ON DELETE CASCADE,
    set_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_active_bank_accounts_account_id ON active_bank_accounts(bank_account_id);

-- Customers table
CREATE TABLE IF NOT EXISTS customers (
    id TEXT PRIMARY KEY NOT NULL,
    company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    address TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    archived_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_customers_company_id ON customers(company_id);
CREATE INDEX IF NOT EXISTS idx_customers_archived_at ON customers(archived_at) WHERE archived_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_customers_name ON customers(company_id, name) WHERE archived_at IS NULL;

-- Invoices table
CREATE TABLE IF NOT EXISTS invoices (
    id TEXT PRIMARY KEY NOT NULL,
    company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    customer_id TEXT NOT NULL REFERENCES customers(id) ON DELETE RESTRICT,
    bank_account_id TEXT REFERENCES bank_accounts(id) ON DELETE SET NULL,
    invoice_number TEXT NOT NULL,
    invoice_date TEXT NOT NULL,
    due_date TEXT NOT NULL,
    payment_terms TEXT NOT NULL,
    currency TEXT NOT NULL,
    status TEXT NOT NULL,
    pdf_path TEXT,
    pdf_drive_file_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    archived_at TEXT,
    CONSTRAINT invoices_company_number_unique UNIQUE (company_id, invoice_number)
);

CREATE INDEX IF NOT EXISTS idx_invoices_company_id ON invoices(company_id);
CREATE INDEX IF NOT EXISTS idx_invoices_customer_id ON invoices(customer_id);
CREATE INDEX IF NOT EXISTS idx_invoices_invoice_number ON invoices(company_id, invoice_number);
CREATE INDEX IF NOT EXISTS idx_invoices_status ON invoices(status);
CREATE INDEX IF NOT EXISTS idx_invoices_invoice_date ON invoices(invoice_date);
CREATE INDEX IF NOT EXISTS idx_invoices_due_date ON invoices(due_date);
CREATE INDEX IF NOT EXISTS idx_invoices_archived_at ON invoices(archived_at) WHERE archived_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_invoices_overdue ON invoices(company_id, due_date, status) WHERE status = 'sent' AND archived_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_invoices_pdf_drive_file_id ON invoices(pdf_drive_file_id);

-- Invoice line items table
CREATE TABLE IF NOT EXISTS invoice_line_items (
    id TEXT PRIMARY KEY NOT NULL,
    invoice_id TEXT NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
    description TEXT NOT NULL,
    quantity TEXT NOT NULL,
    unit_price_amount TEXT NOT NULL,
    unit_price_currency TEXT NOT NULL,
    vat_rate TEXT NOT NULL,
    line_order INTEGER NOT NULL,
    CONSTRAINT line_items_invoice_order_unique UNIQUE (invoice_id, line_order)
);

CREATE INDEX IF NOT EXISTS idx_line_items_invoice_id ON invoice_line_items(invoice_id);
CREATE INDEX IF NOT EXISTS idx_line_items_order ON invoice_line_items(invoice_id, line_order);

-- Invoice templates table
CREATE TABLE IF NOT EXISTS invoice_templates (
    id TEXT PRIMARY KEY NOT NULL,
    company_id TEXT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    customer_id TEXT NOT NULL REFERENCES customers(id) ON DELETE CASCADE,
    bank_account_id TEXT REFERENCES bank_accounts(id) ON DELETE SET NULL,
    payment_terms TEXT NOT NULL,
    currency TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    archived_at TEXT,
    CONSTRAINT templates_company_name_unique UNIQUE (company_id, name)
);

CREATE INDEX IF NOT EXISTS idx_templates_company_id ON invoice_templates(company_id);
CREATE INDEX IF NOT EXISTS idx_templates_customer_id ON invoice_templates(customer_id);
CREATE INDEX IF NOT EXISTS idx_templates_archived_at ON invoice_templates(archived_at) WHERE archived_at IS NULL;

-- Invoice template line items table
CREATE TABLE IF NOT EXISTS invoice_template_line_items (
    id TEXT PRIMARY KEY NOT NULL,
    template_id TEXT NOT NULL REFERENCES invoice_templates(id) ON DELETE CASCADE,
    description TEXT NOT NULL,
    quantity TEXT NOT NULL,
    unit_price_amount TEXT NOT NULL,
    unit_price_currency TEXT NOT NULL,
    vat_rate TEXT NOT NULL,
    line_order INTEGER NOT NULL,
    CONSTRAINT template_items_order_unique UNIQUE (template_id, line_order)
);

CREATE INDEX IF NOT EXISTS idx_template_items_template_id ON invoice_template_line_items(template_id);
CREATE INDEX IF NOT EXISTS idx_template_items_order ON invoice_template_line_items(template_id, line_order);
