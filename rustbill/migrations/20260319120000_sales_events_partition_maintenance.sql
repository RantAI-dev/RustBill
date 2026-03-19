CREATE OR REPLACE FUNCTION ensure_sales_events_partitions(
    p_anchor_date date DEFAULT date_trunc('month', now())::date,
    p_months_ahead int DEFAULT 4,
    p_months_back int DEFAULT 1
) RETURNS void AS $$
DECLARE
    start_month date;
    end_month date;
    m date;
    part_name text;
BEGIN
    start_month := (date_trunc('month', p_anchor_date)::date - make_interval(months => p_months_back));
    end_month := (date_trunc('month', p_anchor_date)::date + make_interval(months => p_months_ahead));

    m := start_month;
    WHILE m <= end_month LOOP
        part_name := format('sales_events_%s', to_char(m, 'YYYY_MM'));
        EXECUTE format(
            'CREATE TABLE IF NOT EXISTS %I PARTITION OF sales_events FOR VALUES FROM (%L) TO (%L)',
            part_name,
            m::timestamptz,
            (m + INTERVAL '1 month')::timestamptz
        );
        m := (m + INTERVAL '1 month')::date;
    END LOOP;
END;
$$ LANGUAGE plpgsql;

SELECT ensure_sales_events_partitions();
