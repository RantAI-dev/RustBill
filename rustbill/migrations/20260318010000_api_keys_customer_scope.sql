ALTER TABLE api_keys
    ADD COLUMN IF NOT EXISTS customer_id TEXT REFERENCES customers(id);

CREATE INDEX IF NOT EXISTS api_keys_customer_id_idx ON api_keys(customer_id);
