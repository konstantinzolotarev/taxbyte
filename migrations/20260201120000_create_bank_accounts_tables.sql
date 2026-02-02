-- Create bank accounts table
CREATE TABLE bank_accounts (
    id UUID PRIMARY KEY,
    company_id UUID NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    name VARCHAR(100) NOT NULL,
    iban VARCHAR(34) NOT NULL,
    bank_details TEXT,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    archived_at TIMESTAMPTZ,
    CONSTRAINT bank_accounts_company_iban_unique UNIQUE (company_id, iban)
);

-- Create active bank accounts table
CREATE TABLE active_bank_accounts (
    company_id UUID PRIMARY KEY REFERENCES companies(id) ON DELETE CASCADE,
    bank_account_id UUID NOT NULL REFERENCES bank_accounts(id) ON DELETE CASCADE,
    set_at TIMESTAMPTZ NOT NULL
);

-- Create indexes
CREATE INDEX idx_bank_accounts_company_id ON bank_accounts(company_id);
CREATE INDEX idx_bank_accounts_archived_at ON bank_accounts(archived_at) WHERE archived_at IS NULL;
CREATE INDEX idx_bank_accounts_iban ON bank_accounts(iban);
CREATE INDEX idx_active_bank_accounts_account_id ON active_bank_accounts(bank_account_id);
