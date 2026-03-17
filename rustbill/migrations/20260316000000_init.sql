-- RantAI Billing: full schema migration
-- Generated from Drizzle ORM schema (lib/db/schema.ts)

-- ============================================================
-- ENUM TYPES
-- ============================================================

CREATE TYPE product_type AS ENUM ('licensed', 'saas', 'api');
CREATE TYPE license_status AS ENUM ('active', 'expired', 'revoked', 'suspended');
CREATE TYPE trend AS ENUM ('up', 'down', 'stable');
CREATE TYPE deal_type AS ENUM ('sale', 'trial', 'partner');
CREATE TYPE api_key_status AS ENUM ('active', 'revoked');
CREATE TYPE user_role AS ENUM ('admin', 'customer');
CREATE TYPE billing_cycle AS ENUM ('monthly', 'quarterly', 'yearly');
CREATE TYPE pricing_model AS ENUM ('flat', 'per_unit', 'tiered', 'usage_based');
CREATE TYPE subscription_status AS ENUM ('active', 'paused', 'canceled', 'past_due', 'trialing');
CREATE TYPE invoice_status AS ENUM ('draft', 'issued', 'paid', 'overdue', 'void');
CREATE TYPE payment_method AS ENUM ('manual', 'stripe', 'xendit', 'lemonsqueezy', 'bank_transfer', 'check');
CREATE TYPE credit_note_status AS ENUM ('draft', 'issued', 'void');
CREATE TYPE discount_type AS ENUM ('percentage', 'fixed_amount');
CREATE TYPE refund_status AS ENUM ('pending', 'completed', 'failed');
CREATE TYPE dunning_step AS ENUM ('reminder', 'warning', 'final_notice', 'suspension');
CREATE TYPE webhook_status AS ENUM ('active', 'inactive');
CREATE TYPE billing_event_type AS ENUM (
    'invoice.created', 'invoice.issued', 'invoice.paid', 'invoice.overdue', 'invoice.voided',
    'payment.received', 'payment.refunded',
    'subscription.created', 'subscription.renewed', 'subscription.canceled', 'subscription.paused',
    'dunning.reminder', 'dunning.warning', 'dunning.final_notice', 'dunning.suspension'
);

-- ============================================================
-- SEQUENCE
-- ============================================================

CREATE SEQUENCE IF NOT EXISTS invoice_number_seq;
CREATE SEQUENCE IF NOT EXISTS credit_note_number_seq;

-- ============================================================
-- TABLES (ordered by foreign-key dependencies)
-- ============================================================

-- products (no FK deps)
CREATE TABLE products (
    id TEXT PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    product_type product_type NOT NULL,
    revenue NUMERIC(12,2) NOT NULL DEFAULT 0,
    target NUMERIC(12,2) NOT NULL DEFAULT 0,
    change NUMERIC(12,2) NOT NULL DEFAULT 0,
    units_sold INTEGER,
    active_licenses INTEGER,
    total_licenses INTEGER,
    mau INTEGER,
    dau INTEGER,
    free_users INTEGER,
    paid_users INTEGER,
    churn_rate NUMERIC(12,4),
    api_calls INTEGER,
    active_developers INTEGER,
    avg_latency NUMERIC(12,4),
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- customers (no FK deps)
CREATE TABLE customers (
    id TEXT PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    industry VARCHAR(255) NOT NULL,
    tier VARCHAR(50) NOT NULL,
    location VARCHAR(255) NOT NULL,
    contact VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL,
    phone VARCHAR(50) NOT NULL,
    total_revenue NUMERIC(12,2) NOT NULL DEFAULT 0,
    health_score INTEGER NOT NULL DEFAULT 50,
    trend trend NOT NULL DEFAULT 'stable',
    last_contact VARCHAR(100) NOT NULL,
    billing_email VARCHAR(255),
    billing_address TEXT,
    billing_city VARCHAR(100),
    billing_state VARCHAR(100),
    billing_zip VARCHAR(20),
    billing_country VARCHAR(100),
    tax_id VARCHAR(50),
    default_payment_method payment_method,
    stripe_customer_id VARCHAR(255),
    xendit_customer_id VARCHAR(255),
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- customer_products (depends on: customers, products)
CREATE TABLE customer_products (
    id TEXT PRIMARY KEY,
    customer_id TEXT NOT NULL,
    product_id TEXT NOT NULL,
    license_keys JSONB,
    mau INTEGER,
    api_calls INTEGER
);

-- deals (depends on: customers, products)
CREATE TABLE deals (
    id TEXT PRIMARY KEY,
    customer_id TEXT,
    company VARCHAR(255) NOT NULL,
    contact VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL,
    value NUMERIC(12,2) NOT NULL,
    product_id TEXT,
    product_name VARCHAR(255) NOT NULL,
    product_type product_type NOT NULL,
    deal_type deal_type NOT NULL DEFAULT 'sale',
    date VARCHAR(20) NOT NULL,
    license_key VARCHAR(50),
    notes TEXT,
    usage_metric_label VARCHAR(50),
    usage_metric_value INTEGER,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- licenses (depends on: customers, products)
CREATE TABLE licenses (
    key VARCHAR(50) PRIMARY KEY,
    customer_id TEXT,
    customer_name VARCHAR(255) NOT NULL,
    product_id TEXT,
    product_name VARCHAR(255) NOT NULL,
    status license_status NOT NULL DEFAULT 'active',
    created_at VARCHAR(20) NOT NULL,
    expires_at VARCHAR(20) NOT NULL,
    license_type VARCHAR(10) NOT NULL DEFAULT 'simple',
    signed_payload TEXT,
    signature TEXT,
    features JSONB,
    max_activations INTEGER
);

-- license_activations (depends on: licenses)
CREATE TABLE license_activations (
    id TEXT PRIMARY KEY,
    license_key VARCHAR(50) NOT NULL,
    device_id VARCHAR(255) NOT NULL,
    device_name VARCHAR(255),
    ip_address VARCHAR(45),
    activated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMP NOT NULL DEFAULT NOW(),
    CONSTRAINT license_activations_key_device_unique UNIQUE (license_key, device_id)
);

CREATE INDEX license_activations_license_key_idx ON license_activations (license_key);

-- api_keys (no FK deps)
CREATE TABLE api_keys (
    id TEXT PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    key_hash VARCHAR(128) NOT NULL,
    key_prefix VARCHAR(12) NOT NULL,
    status api_key_status NOT NULL DEFAULT 'active',
    last_used_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- users (depends on: customers)
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    email VARCHAR(255) NOT NULL UNIQUE,
    name VARCHAR(255) NOT NULL,
    password_hash VARCHAR(255),
    role user_role NOT NULL DEFAULT 'customer',
    auth_provider VARCHAR(20) NOT NULL DEFAULT 'default',
    customer_id TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- sessions (depends on: users)
CREATE TABLE sessions (
    id VARCHAR(64) PRIMARY KEY,
    user_id TEXT NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- pricing_plans (depends on: products)
CREATE TABLE pricing_plans (
    id TEXT PRIMARY KEY,
    product_id TEXT,
    name VARCHAR(255) NOT NULL,
    pricing_model pricing_model NOT NULL,
    billing_cycle billing_cycle NOT NULL,
    base_price NUMERIC(12,2) NOT NULL DEFAULT 0,
    unit_price NUMERIC(12,2),
    tiers JSONB,
    usage_metric_name VARCHAR(100),
    trial_days INTEGER NOT NULL DEFAULT 0,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- subscriptions (depends on: customers, pricing_plans)
CREATE TABLE subscriptions (
    id TEXT PRIMARY KEY,
    customer_id TEXT NOT NULL,
    plan_id TEXT NOT NULL,
    status subscription_status NOT NULL DEFAULT 'active',
    current_period_start TIMESTAMP NOT NULL,
    current_period_end TIMESTAMP NOT NULL,
    canceled_at TIMESTAMP,
    cancel_at_period_end BOOLEAN NOT NULL DEFAULT FALSE,
    trial_end TIMESTAMP,
    quantity INTEGER NOT NULL DEFAULT 1,
    metadata JSONB,
    stripe_subscription_id VARCHAR(255),
    version INTEGER NOT NULL DEFAULT 1,
    deleted_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX subscriptions_customer_id_idx ON subscriptions (customer_id);
CREATE INDEX subscriptions_status_idx ON subscriptions (status);

-- invoices (depends on: customers, subscriptions)
CREATE TABLE invoices (
    id TEXT PRIMARY KEY,
    invoice_number VARCHAR(20) NOT NULL UNIQUE,
    customer_id TEXT NOT NULL,
    subscription_id TEXT,
    status invoice_status NOT NULL DEFAULT 'draft',
    issued_at TIMESTAMP,
    due_at TIMESTAMP,
    paid_at TIMESTAMP,
    subtotal NUMERIC(12,2) NOT NULL DEFAULT 0,
    tax NUMERIC(12,2) NOT NULL DEFAULT 0,
    total NUMERIC(12,2) NOT NULL DEFAULT 0,
    currency VARCHAR(3) NOT NULL DEFAULT 'USD',
    notes TEXT,
    stripe_invoice_id VARCHAR(255),
    xendit_invoice_id VARCHAR(255),
    lemonsqueezy_order_id VARCHAR(255),
    version INTEGER NOT NULL DEFAULT 1,
    deleted_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX invoices_customer_id_idx ON invoices (customer_id);
CREATE INDEX invoices_status_idx ON invoices (status);

-- invoice_items (depends on: invoices)
CREATE TABLE invoice_items (
    id TEXT PRIMARY KEY,
    invoice_id TEXT NOT NULL,
    description VARCHAR(500) NOT NULL,
    quantity NUMERIC(12,2) NOT NULL,
    unit_price NUMERIC(12,2) NOT NULL,
    amount NUMERIC(12,2) NOT NULL,
    period_start TIMESTAMP,
    period_end TIMESTAMP
);

-- payments (depends on: invoices)
CREATE TABLE payments (
    id TEXT PRIMARY KEY,
    invoice_id TEXT NOT NULL,
    amount NUMERIC(12,2) NOT NULL,
    method payment_method NOT NULL,
    reference VARCHAR(255),
    paid_at TIMESTAMP NOT NULL,
    notes TEXT,
    stripe_payment_intent_id VARCHAR(255),
    xendit_payment_id VARCHAR(255),
    lemonsqueezy_order_id VARCHAR(255),
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX payments_invoice_id_idx ON payments (invoice_id);

-- usage_events (depends on: subscriptions)
CREATE TABLE usage_events (
    id TEXT PRIMARY KEY,
    subscription_id TEXT NOT NULL,
    metric_name VARCHAR(100) NOT NULL,
    value NUMERIC(12,4) NOT NULL,
    timestamp TIMESTAMP NOT NULL DEFAULT NOW(),
    idempotency_key VARCHAR(255),
    properties JSONB,
    CONSTRAINT usage_events_idempotency_key_unique UNIQUE (idempotency_key)
);

CREATE INDEX usage_events_subscription_id_idx ON usage_events (subscription_id);

-- credit_notes (depends on: invoices, customers)
CREATE TABLE credit_notes (
    id TEXT PRIMARY KEY,
    credit_note_number VARCHAR(20) NOT NULL UNIQUE,
    invoice_id TEXT NOT NULL,
    customer_id TEXT NOT NULL,
    reason VARCHAR(500) NOT NULL,
    amount NUMERIC(12,2) NOT NULL,
    status credit_note_status NOT NULL DEFAULT 'draft',
    issued_at TIMESTAMP,
    deleted_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- credit_note_items (depends on: credit_notes)
CREATE TABLE credit_note_items (
    id TEXT PRIMARY KEY,
    credit_note_id TEXT NOT NULL,
    description VARCHAR(500) NOT NULL,
    quantity NUMERIC(12,2) NOT NULL,
    unit_price NUMERIC(12,2) NOT NULL,
    amount NUMERIC(12,2) NOT NULL
);

-- coupons (no FK deps)
CREATE TABLE coupons (
    id TEXT PRIMARY KEY,
    code VARCHAR(50) NOT NULL UNIQUE,
    name VARCHAR(255) NOT NULL,
    discount_type discount_type NOT NULL,
    discount_value NUMERIC(12,2) NOT NULL,
    currency VARCHAR(3) NOT NULL DEFAULT 'USD',
    max_redemptions INTEGER,
    times_redeemed INTEGER NOT NULL DEFAULT 0,
    valid_from TIMESTAMP NOT NULL DEFAULT NOW(),
    valid_until TIMESTAMP,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    applies_to JSONB,
    deleted_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- subscription_discounts (depends on: subscriptions, coupons)
CREATE TABLE subscription_discounts (
    id TEXT PRIMARY KEY,
    subscription_id TEXT NOT NULL,
    coupon_id TEXT NOT NULL,
    applied_at TIMESTAMP NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMP
);

-- refunds (depends on: payments, invoices)
CREATE TABLE refunds (
    id TEXT PRIMARY KEY,
    payment_id TEXT NOT NULL,
    invoice_id TEXT NOT NULL,
    amount NUMERIC(12,2) NOT NULL,
    reason VARCHAR(500) NOT NULL,
    status refund_status NOT NULL DEFAULT 'pending',
    stripe_refund_id VARCHAR(255),
    processed_at TIMESTAMP,
    deleted_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX refunds_payment_id_idx ON refunds (payment_id);

-- dunning_log (depends on: invoices, subscriptions)
CREATE TABLE dunning_log (
    id TEXT PRIMARY KEY,
    invoice_id TEXT NOT NULL,
    subscription_id TEXT,
    step dunning_step NOT NULL,
    scheduled_at TIMESTAMP NOT NULL,
    executed_at TIMESTAMP,
    notes TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX dunning_log_invoice_id_idx ON dunning_log (invoice_id);

-- billing_events (depends on: customers)
CREATE TABLE billing_events (
    id TEXT PRIMARY KEY,
    event_type billing_event_type NOT NULL,
    resource_type VARCHAR(50) NOT NULL,
    resource_id TEXT NOT NULL,
    customer_id TEXT,
    data JSONB,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX billing_events_customer_id_idx ON billing_events (customer_id);

-- webhook_endpoints (no FK deps)
CREATE TABLE webhook_endpoints (
    id TEXT PRIMARY KEY,
    url VARCHAR(500) NOT NULL,
    description VARCHAR(255),
    secret VARCHAR(255) NOT NULL,
    events JSONB NOT NULL,
    status webhook_status NOT NULL DEFAULT 'active',
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- system_settings (no FK deps)
CREATE TABLE system_settings (
    key VARCHAR(100) PRIMARY KEY,
    value TEXT NOT NULL,
    sensitive BOOLEAN NOT NULL DEFAULT FALSE,
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- webhook_deliveries (depends on: webhook_endpoints, billing_events)
CREATE TABLE webhook_deliveries (
    id TEXT PRIMARY KEY,
    endpoint_id TEXT NOT NULL,
    event_id TEXT NOT NULL,
    payload JSONB NOT NULL,
    response_code INTEGER,
    response_body TEXT,
    attempts INTEGER NOT NULL DEFAULT 0,
    delivered_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);
