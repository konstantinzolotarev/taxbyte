-- Active company tracking table
CREATE TABLE active_companies (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    company_id UUID NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    set_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for faster lookups
CREATE INDEX idx_active_companies_company_id ON active_companies(company_id);

-- Comment
COMMENT ON TABLE active_companies IS 'Tracks the currently active company for each user session';
