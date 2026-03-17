-- New enums
ALTER TYPE billing_event_type ADD VALUE IF NOT EXISTS 'credit.deposited';
ALTER TYPE billing_event_type ADD VALUE IF NOT EXISTS 'credit.applied';
ALTER TYPE billing_event_type ADD VALUE IF NOT EXISTS 'payment_method.added';
ALTER TYPE billing_event_type ADD VALUE IF NOT EXISTS 'payment_method.removed';
ALTER TYPE billing_event_type ADD VALUE IF NOT EXISTS 'payment_method.failed';
ALTER TYPE billing_event_type ADD VALUE IF NOT EXISTS 'subscription.plan_changed';

CREATE TYPE credit_reason AS ENUM ('proration', 'credit_note', 'manual', 'overpayment', 'refund');
CREATE TYPE saved_payment_method_status AS ENUM ('active', 'expired', 'failed');
CREATE TYPE saved_payment_method_type AS ENUM ('card', 'bank_account', 'ewallet', 'va');
CREATE TYPE payment_provider AS ENUM ('stripe', 'xendit', 'lemonsqueezy');

-- Customer credit balance (source of truth for current balance)
CREATE TABLE customer_credit_balances (
    customer_id TEXT NOT NULL REFERENCES customers(id),
    currency VARCHAR(3) NOT NULL DEFAULT 'USD',
    balance NUMERIC(12,2) NOT NULL DEFAULT 0 CHECK (balance >= 0),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    PRIMARY KEY (customer_id, currency)
);

-- Customer credits audit log
CREATE TABLE customer_credits (
    id TEXT PRIMARY KEY,
    customer_id TEXT NOT NULL REFERENCES customers(id),
    currency VARCHAR(3) NOT NULL,
    amount NUMERIC(12,2) NOT NULL,
    balance_after NUMERIC(12,2) NOT NULL,
    reason credit_reason NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    invoice_id TEXT REFERENCES invoices(id),
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_customer_credits_customer ON customer_credits (customer_id, currency);

-- Tax rules (immutable — close old, create new)
CREATE TABLE tax_rules (
    id TEXT PRIMARY KEY,
    country VARCHAR(2) NOT NULL,
    region VARCHAR(100),
    tax_name TEXT NOT NULL,
    rate NUMERIC(6,4) NOT NULL,
    inclusive BOOLEAN NOT NULL DEFAULT FALSE,
    product_category TEXT,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    effective_from DATE NOT NULL DEFAULT CURRENT_DATE,
    effective_to DATE,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_tax_rules_lookup ON tax_rules (country, region, active, effective_from);

-- Saved payment methods
CREATE TABLE saved_payment_methods (
    id TEXT PRIMARY KEY,
    customer_id TEXT NOT NULL REFERENCES customers(id),
    provider payment_provider NOT NULL,
    provider_token TEXT NOT NULL,
    method_type saved_payment_method_type NOT NULL,
    label TEXT NOT NULL DEFAULT '',
    last_four VARCHAR(4),
    expiry_month INTEGER,
    expiry_year INTEGER,
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    status saved_payment_method_status NOT NULL DEFAULT 'active',
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);
CREATE UNIQUE INDEX idx_one_default_per_customer
    ON saved_payment_methods (customer_id) WHERE is_default = TRUE;
CREATE INDEX idx_saved_pm_customer ON saved_payment_methods (customer_id);

-- New columns on invoices
ALTER TABLE invoices ADD COLUMN IF NOT EXISTS tax_name VARCHAR(50);
ALTER TABLE invoices ADD COLUMN IF NOT EXISTS tax_rate NUMERIC(6,4);
ALTER TABLE invoices ADD COLUMN IF NOT EXISTS tax_inclusive BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE invoices ADD COLUMN IF NOT EXISTS credits_applied NUMERIC(12,2) NOT NULL DEFAULT 0;
ALTER TABLE invoices ADD COLUMN IF NOT EXISTS amount_due NUMERIC(12,2);
ALTER TABLE invoices ADD COLUMN IF NOT EXISTS auto_charge_attempts INTEGER NOT NULL DEFAULT 0;
ALTER TABLE invoices ADD COLUMN IF NOT EXISTS idempotency_key VARCHAR(255);
CREATE UNIQUE INDEX idx_invoice_idempotency ON invoices (idempotency_key) WHERE idempotency_key IS NOT NULL;

-- Backfill amount_due = total for existing invoices
UPDATE invoices SET amount_due = total WHERE amount_due IS NULL;
ALTER TABLE invoices ALTER COLUMN amount_due SET DEFAULT 0;

-- New columns on subscriptions
ALTER TABLE subscriptions ADD COLUMN IF NOT EXISTS managed_by VARCHAR(20);

-- Seed tax rules
INSERT INTO tax_rules (id, country, region, tax_name, rate, inclusive, effective_from) VALUES
    (gen_random_uuid()::text, 'ID', NULL, 'PPN', 0.1100, TRUE, '2025-01-01'),
    (gen_random_uuid()::text, 'SG', NULL, 'GST', 0.0900, FALSE, '2025-01-01'),
    (gen_random_uuid()::text, 'US', 'CA', 'Sales Tax', 0.0725, FALSE, '2025-01-01'),
    (gen_random_uuid()::text, 'US', 'TX', 'Sales Tax', 0.0625, FALSE, '2025-01-01'),
    (gen_random_uuid()::text, 'DE', NULL, 'VAT', 0.1900, TRUE, '2025-01-01'),
    (gen_random_uuid()::text, 'GB', NULL, 'VAT', 0.2000, TRUE, '2025-01-01');
