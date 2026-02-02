-- Create invoice_templates table
CREATE TABLE invoice_templates (
    id UUID PRIMARY KEY,
    company_id UUID NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    customer_id UUID NOT NULL REFERENCES customers(id) ON DELETE CASCADE,
    bank_account_id UUID REFERENCES bank_accounts(id) ON DELETE SET NULL,
    payment_terms VARCHAR(50) NOT NULL,
    currency VARCHAR(3) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    archived_at TIMESTAMPTZ,
    CONSTRAINT templates_company_name_unique UNIQUE (company_id, name)
);

CREATE INDEX idx_templates_company_id ON invoice_templates(company_id);
CREATE INDEX idx_templates_customer_id ON invoice_templates(customer_id);
CREATE INDEX idx_templates_archived_at ON invoice_templates(archived_at) WHERE archived_at IS NULL;

-- Create invoice_template_line_items table
CREATE TABLE invoice_template_line_items (
    id UUID PRIMARY KEY,
    template_id UUID NOT NULL REFERENCES invoice_templates(id) ON DELETE CASCADE,
    description TEXT NOT NULL,
    quantity DECIMAL(12, 4) NOT NULL,
    unit_price_amount DECIMAL(12, 2) NOT NULL,
    unit_price_currency VARCHAR(3) NOT NULL,
    vat_rate DECIMAL(5, 2) NOT NULL,
    line_order INTEGER NOT NULL,
    CONSTRAINT template_items_order_unique UNIQUE (template_id, line_order),
    CONSTRAINT template_items_quantity_positive CHECK (quantity > 0),
    CONSTRAINT template_items_price_positive CHECK (unit_price_amount >= 0),
    CONSTRAINT template_items_vat_valid CHECK (vat_rate >= 0 AND vat_rate <= 100)
);

CREATE INDEX idx_template_items_template_id ON invoice_template_line_items(template_id);
CREATE INDEX idx_template_items_order ON invoice_template_line_items(template_id, line_order);
