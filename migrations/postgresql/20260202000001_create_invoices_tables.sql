-- Customers table (reusable client information)
CREATE TABLE customers (
  id UUID PRIMARY KEY,
  company_id UUID NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
  name VARCHAR(255) NOT NULL,
  address JSONB,
  created_at TIMESTAMPTZ NOT NULL,
  updated_at TIMESTAMPTZ NOT NULL,
  archived_at TIMESTAMPTZ
);

-- Invoices table (main invoice document)
CREATE TABLE invoices (
  id UUID PRIMARY KEY,
  company_id UUID NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
  customer_id UUID NOT NULL REFERENCES customers(id) ON DELETE RESTRICT,
  bank_account_id UUID REFERENCES bank_accounts(id) ON DELETE SET NULL,
  invoice_number INTEGER NOT NULL,
  invoice_date DATE NOT NULL,
  due_date DATE NOT NULL,
  payment_terms VARCHAR(50) NOT NULL,
  currency VARCHAR(3) NOT NULL,
  status VARCHAR(20) NOT NULL,
  pdf_path VARCHAR(500),
  created_at TIMESTAMPTZ NOT NULL,
  updated_at TIMESTAMPTZ NOT NULL,
  archived_at TIMESTAMPTZ,
  CONSTRAINT invoices_company_number_unique UNIQUE (company_id, invoice_number),
  CONSTRAINT invoices_invoice_number_positive CHECK (invoice_number > 0)
);

-- Invoice Line Items table
CREATE TABLE invoice_line_items (
  id UUID PRIMARY KEY,
  invoice_id UUID NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
  description TEXT NOT NULL,
  quantity DECIMAL(12, 4) NOT NULL,
  unit_price_amount DECIMAL(12, 2) NOT NULL,
  unit_price_currency VARCHAR(3) NOT NULL,
  vat_rate DECIMAL(5, 2) NOT NULL,
  line_order INTEGER NOT NULL,
  CONSTRAINT line_items_invoice_order_unique UNIQUE (invoice_id, line_order),
  CONSTRAINT line_items_quantity_positive CHECK (quantity > 0),
  CONSTRAINT line_items_unit_price_positive CHECK (unit_price_amount >= 0),
  CONSTRAINT line_items_vat_rate_valid CHECK (vat_rate >= 0 AND vat_rate <= 100)
);

-- Indexes for Customers
CREATE INDEX idx_customers_company_id ON customers(company_id);
CREATE INDEX idx_customers_archived_at ON customers(archived_at) WHERE archived_at IS NULL;
CREATE INDEX idx_customers_name ON customers(company_id, name) WHERE archived_at IS NULL;

-- Indexes for Invoices
CREATE INDEX idx_invoices_company_id ON invoices(company_id);
CREATE INDEX idx_invoices_customer_id ON invoices(customer_id);
CREATE INDEX idx_invoices_invoice_number ON invoices(company_id, invoice_number);
CREATE INDEX idx_invoices_status ON invoices(status);
CREATE INDEX idx_invoices_invoice_date ON invoices(invoice_date);
CREATE INDEX idx_invoices_due_date ON invoices(due_date);
CREATE INDEX idx_invoices_archived_at ON invoices(archived_at) WHERE archived_at IS NULL;
CREATE INDEX idx_invoices_overdue ON invoices(company_id, due_date, status)
  WHERE status = 'sent' AND archived_at IS NULL;

-- Indexes for Invoice Line Items
CREATE INDEX idx_line_items_invoice_id ON invoice_line_items(invoice_id);
CREATE INDEX idx_line_items_order ON invoice_line_items(invoice_id, line_order);
