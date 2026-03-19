BEGIN;

ALTER TABLE sales_events RENAME TO sales_events_unpartitioned;

CREATE TABLE sales_events (
    id text NOT NULL DEFAULT gen_random_uuid()::text,
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
    created_at timestamptz NOT NULL DEFAULT NOW()
) PARTITION BY RANGE (occurred_at);

CREATE TABLE sales_events_default PARTITION OF sales_events DEFAULT;

DO $$
DECLARE
    start_month date := date_trunc('month', now())::date - interval '2 months';
    end_month date := date_trunc('month', now())::date + interval '4 months';
    m date;
    part_name text;
BEGIN
    m := start_month;
    WHILE m < end_month LOOP
        part_name := format('sales_events_%s', to_char(m, 'YYYY_MM'));
        EXECUTE format(
            'CREATE TABLE IF NOT EXISTS %I PARTITION OF sales_events FOR VALUES FROM (%L) TO (%L)',
            part_name,
            m::timestamptz,
            (m + interval '1 month')::timestamptz
        );
        m := (m + interval '1 month')::date;
    END LOOP;
END $$;

CREATE INDEX idx_sales_events_part_occurred_at ON sales_events (occurred_at);
CREATE INDEX idx_sales_events_part_classification_occurred_at ON sales_events (classification, occurred_at);
CREATE INDEX idx_sales_events_part_customer_occurred_at ON sales_events (customer_id, occurred_at);
CREATE INDEX idx_sales_events_part_event_type_occurred_at ON sales_events (event_type, occurred_at);

CREATE TABLE sales_event_idempotency_keys (
    source_table text NOT NULL,
    source_id text NOT NULL,
    event_type text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT NOW(),
    PRIMARY KEY (source_table, source_id, event_type)
);

INSERT INTO sales_events (
    id,
    occurred_at,
    event_type,
    classification,
    amount_subtotal,
    amount_tax,
    amount_total,
    currency,
    customer_id,
    subscription_id,
    product_id,
    invoice_id,
    payment_id,
    source_table,
    source_id,
    metadata,
    created_at
)
SELECT
    id,
    occurred_at,
    event_type,
    classification,
    amount_subtotal,
    amount_tax,
    amount_total,
    currency,
    customer_id,
    subscription_id,
    product_id,
    invoice_id,
    payment_id,
    source_table,
    source_id,
    metadata,
    created_at
FROM sales_events_unpartitioned;

INSERT INTO sales_event_idempotency_keys (source_table, source_id, event_type)
SELECT DISTINCT source_table, source_id, event_type
FROM sales_events_unpartitioned
ON CONFLICT (source_table, source_id, event_type) DO NOTHING;

DROP TABLE sales_events_unpartitioned;

COMMIT;
