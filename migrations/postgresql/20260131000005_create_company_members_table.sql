-- Create company_members table
CREATE TABLE company_members (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    company_id UUID NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role VARCHAR(50) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    UNIQUE(company_id, user_id)
);

-- Create index on company_id for faster company member lookups
CREATE INDEX idx_company_members_company_id ON company_members(company_id);

-- Create index on user_id for faster user company lookups
CREATE INDEX idx_company_members_user_id ON company_members(user_id);

-- Create index on role for faster role-based queries
CREATE INDEX idx_company_members_role ON company_members(role);
