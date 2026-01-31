-- Create companies table
CREATE TABLE companies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    tax_id VARCHAR(50),
    address TEXT,
    phone VARCHAR(50),
    email VARCHAR(255),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create index on name for faster company lookups
CREATE INDEX idx_companies_name ON companies(name);

-- Create index on tax_id for faster tax ID lookups
CREATE INDEX idx_companies_tax_id ON companies(tax_id) WHERE tax_id IS NOT NULL;
