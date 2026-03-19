CREATE TABLE IF NOT EXISTS sales_events (
    id text PRIMARY KEY DEFAULT gen_random_uuid()::text,
    occurred_at timestamptz NOT NULL,
    event_type text NOT NULL,
    classification text NOT NULL CHECK (classification IN ('bookings', 'billings', 'collections', 'adjustments', 'recurring')),
    amount_subtotal numeric(20,6) NOT NULL DEFAULT 0,
    amount_tax numeric(20,6) NOT NULL DEFAULT 0,
    amount_total numeric(20,6) NOT NULL DEFAULT 0,
    currency text NOT NULL DEFAULT 'USD',
    customer_id text,
    subscription_id text,
    product_id text,
    invoice_id text,
    payment_id text,
    source_table text NOT NULL,
    source_id text NOT NULL,
    metadata jsonb,
    created_at timestamptz NOT NULL DEFAULT NOW(),
    CONSTRAINT sales_events_idem UNIQUE (source_table, source_id, event_type)
);

CREATE INDEX IF NOT EXISTS idx_sales_events_occurred_at ON sales_events (occurred_at);
CREATE INDEX IF NOT EXISTS idx_sales_events_classification_occurred_at ON sales_events (classification, occurred_at);
CREATE INDEX IF NOT EXISTS idx_sales_events_customer_occurred_at ON sales_events (customer_id, occurred_at);
CREATE INDEX IF NOT EXISTS idx_sales_events_event_type_occurred_at ON sales_events (event_type, occurred_at);
