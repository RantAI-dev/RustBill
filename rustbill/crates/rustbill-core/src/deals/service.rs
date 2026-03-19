use crate::analytics::sales_ledger::{emit_sales_event, NewSalesEvent, SalesClassification};
use crate::db::models::{Deal, ProductType};
use crate::deals::validation::{CreateDealRequest, UpdateDealRequest};
use crate::error::{BillingError, Result};
use rand::Rng;
use rust_decimal::Decimal;
use sqlx::PgPool;

/// Generate a license key in format NQ-XXXX-XXXX-XXXX-XXXX
fn generate_license_key() -> String {
    let mut rng = rand::thread_rng();
    let chars: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let segment = |rng: &mut rand::rngs::ThreadRng| -> String {
        (0..4)
            .map(|_| chars[rng.gen_range(0..chars.len())] as char)
            .collect()
    };
    format!(
        "NQ-{}-{}-{}-{}",
        segment(&mut rng),
        segment(&mut rng),
        segment(&mut rng),
        segment(&mut rng)
    )
}

pub async fn list_deals(
    pool: &PgPool,
    product_type: Option<&str>,
    deal_type: Option<&str>,
) -> Result<Vec<Deal>> {
    let mut query = String::from("SELECT * FROM deals WHERE 1=1");
    let mut binds: Vec<String> = Vec::new();

    if let Some(pt) = product_type {
        binds.push(pt.to_string());
        query.push_str(&format!(
            " AND product_type = ${}::product_type",
            binds.len()
        ));
    }
    if let Some(dt) = deal_type {
        binds.push(dt.to_string());
        query.push_str(&format!(" AND deal_type = ${}::deal_type", binds.len()));
    }

    query.push_str(" ORDER BY created_at DESC");

    // Build the query dynamically
    let mut q = sqlx::query_as::<_, Deal>(&query);
    for b in &binds {
        q = q.bind(b);
    }

    let rows = q.fetch_all(pool).await?;
    Ok(rows)
}

pub async fn get_deal(pool: &PgPool, id: &str) -> Result<Deal> {
    sqlx::query_as::<_, Deal>("SELECT * FROM deals WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| BillingError::not_found("deal", id))
}

pub async fn create_deal(pool: &PgPool, req: CreateDealRequest) -> Result<Deal> {
    let auto_create_invoice = req.auto_create_invoice;

    // Auto-populate from customer FK if provided
    let (company, contact, email) = if let Some(ref cid) = req.customer_id {
        let cust: Option<(String, String, String)> =
            sqlx::query_as("SELECT name, contact, email FROM customers WHERE id = $1")
                .bind(cid)
                .fetch_optional(pool)
                .await?;

        match cust {
            Some((name, ct, em)) => (
                req.company.unwrap_or(name),
                req.contact.unwrap_or(ct),
                req.email.unwrap_or(em),
            ),
            None => return Err(BillingError::not_found("customer", cid.as_str())),
        }
    } else {
        (
            req.company.unwrap_or_default(),
            req.contact.unwrap_or_default(),
            req.email.unwrap_or_default(),
        )
    };

    // Auto-populate from product FK if provided
    let (product_name, product_type) = if let Some(ref pid) = req.product_id {
        let prod: Option<(String, ProductType)> =
            sqlx::query_as("SELECT name, product_type FROM products WHERE id = $1")
                .bind(pid)
                .fetch_optional(pool)
                .await?;

        match prod {
            Some((name, pt)) => (
                req.product_name.unwrap_or(name),
                req.product_type.unwrap_or(pt),
            ),
            None => return Err(BillingError::not_found("product", pid.as_str())),
        }
    } else {
        (
            req.product_name.unwrap_or_default(),
            req.product_type.unwrap_or(ProductType::Licensed),
        )
    };

    // Auto-generate license key for licensed products if not provided
    let license_key = if product_type == ProductType::Licensed && req.license_key.is_none() {
        Some(generate_license_key())
    } else {
        req.license_key
    };

    let date = req
        .date
        .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string());

    let row = sqlx::query_as::<_, Deal>(
        r#"
        INSERT INTO deals (id, customer_id, company, contact, email, value,
            product_id, product_name, product_type, deal_type, date,
            license_key, notes, usage_metric_label, usage_metric_value)
        VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5,
            $6, $7, $8, $9, $10, $11, $12, $13, $14)
        RETURNING *
        "#,
    )
    .bind(&req.customer_id)
    .bind(&company)
    .bind(&contact)
    .bind(&email)
    .bind(&req.value)
    .bind(&req.product_id)
    .bind(&product_name)
    .bind(&product_type)
    .bind(&req.deal_type)
    .bind(&date)
    .bind(&license_key)
    .bind(&req.notes)
    .bind(&req.usage_metric_label)
    .bind(&req.usage_metric_value)
    .fetch_one(pool)
    .await?;

    emit_sales_event(
        pool,
        NewSalesEvent {
            occurred_at: chrono::Utc::now(),
            event_type: "deal.created",
            classification: SalesClassification::Bookings,
            amount_subtotal: row.value,
            amount_tax: Decimal::ZERO,
            amount_total: row.value,
            currency: "USD",
            customer_id: row.customer_id.as_deref(),
            subscription_id: None,
            product_id: row.product_id.as_deref(),
            invoice_id: None,
            payment_id: None,
            source_table: "deals",
            source_id: &row.id,
            metadata: Some(serde_json::json!({
                "deal_id": row.id,
                "deal_type": row.deal_type,
                "product_type": row.product_type,
                "auto_create_invoice": auto_create_invoice,
            })),
        },
    )
    .await?;

    if auto_create_invoice {
        let customer_id = row.customer_id.as_deref().ok_or_else(|| {
            BillingError::bad_request("customerId is required to auto-create invoice")
        })?;

        if row.value <= Decimal::ZERO {
            return Err(BillingError::bad_request(
                "deal value must be greater than 0 to auto-create invoice",
            ));
        }

        create_invoice_from_deal(pool, &row, customer_id, &product_name, &date, row.value).await?;
    }

    // If licensed product and we generated a license key, insert into licenses table
    if product_type == ProductType::Licensed {
        if let Some(ref lk) = row.license_key {
            let customer_name = company.clone();
            let expires_at = chrono::Utc::now()
                .checked_add_signed(chrono::Duration::days(365))
                .unwrap()
                .format("%Y-%m-%d")
                .to_string();

            sqlx::query(
                r#"
                INSERT INTO licenses (key, customer_id, customer_name, product_id, product_name,
                    status, created_at, expires_at, license_type)
                VALUES ($1, $2, $3, $4, $5, 'active', $6, $7, 'perpetual')
                ON CONFLICT (key) DO NOTHING
                "#,
            )
            .bind(lk)
            .bind(&req.customer_id)
            .bind(&customer_name)
            .bind(&req.product_id)
            .bind(&product_name)
            .bind(&date)
            .bind(&expires_at)
            .execute(pool)
            .await?;

            // Auto-sign the license if a signing keypair exists
            if let Ok(Some(_keypair)) = crate::licenses::get_keypair(pool).await {
                // Best-effort: don't fail deal creation if signing fails
                let _ = crate::licenses::sign_license_by_key(pool, lk).await;
            }
        }
    }

    Ok(row)
}

async fn create_invoice_from_deal(
    pool: &PgPool,
    deal: &Deal,
    customer_id: &str,
    product_name: &str,
    issued_at: &str,
    amount: Decimal,
) -> Result<()> {
    let invoice_number = generate_invoice_number(pool).await?;
    let due_at = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::days(30))
        .unwrap()
        .format("%Y-%m-%d")
        .to_string();

    let mut tx = pool.begin().await?;

    let invoice_id: String = sqlx::query_scalar(
        r#"
        INSERT INTO invoices (id, invoice_number, customer_id, subscription_id, status, currency,
          subtotal, tax, total, amount_due, due_at, issued_at, notes, created_at, updated_at)
        VALUES (gen_random_uuid()::text, $1, $2, NULL, 'draft', 'USD',
          $3, 0, $3, $3, $4::timestamp, $5::timestamp, $6, now(), now())
        RETURNING id
        "#,
    )
    .bind(invoice_number)
    .bind(customer_id)
    .bind(amount)
    .bind(due_at)
    .bind(issued_at)
    .bind(format!("Auto-created from deal {}", deal.id))
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO invoice_items (id, invoice_id, description, quantity, unit_price, amount, period_start, period_end)
        VALUES (gen_random_uuid()::text, $1, $2, 1, $3, $3, NULL, NULL)
        "#,
    )
    .bind(&invoice_id)
    .bind(format!("Deal charge: {}", product_name))
    .bind(amount)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    emit_sales_event(
        pool,
        NewSalesEvent {
            occurred_at: chrono::Utc::now(),
            event_type: "invoice.created_from_deal",
            classification: SalesClassification::Billings,
            amount_subtotal: amount,
            amount_tax: Decimal::ZERO,
            amount_total: amount,
            currency: "USD",
            customer_id: Some(customer_id),
            subscription_id: None,
            product_id: deal.product_id.as_deref(),
            invoice_id: Some(&invoice_id),
            payment_id: None,
            source_table: "invoices",
            source_id: &invoice_id,
            metadata: Some(serde_json::json!({
                "deal_id": deal.id,
                "origin": "deal_auto_create",
            })),
        },
    )
    .await?;

    Ok(())
}

async fn generate_invoice_number(pool: &PgPool) -> Result<String> {
    let from_sequence = sqlx::query_scalar::<_, String>(
        "SELECT 'INV-' || LPAD(nextval('invoice_number_seq')::text, 8, '0')",
    )
    .fetch_one(pool)
    .await;

    match from_sequence {
        Ok(value) => Ok(value),
        Err(sqlx::Error::Database(db_err)) if db_err.code().as_deref() == Some("42P01") => {
            let next: i64 = sqlx::query_scalar(
                r#"
                SELECT COALESCE(MAX(NULLIF(regexp_replace(invoice_number, '[^0-9]', '', 'g'), '')::bigint), 0) + 1
                FROM invoices
                "#,
            )
            .fetch_one(pool)
            .await?;

            Ok(format!("INV-{next:08}"))
        }
        Err(err) => Err(BillingError::from(err)),
    }
}

pub async fn update_deal(pool: &PgPool, id: &str, req: UpdateDealRequest) -> Result<Deal> {
    // Ensure exists
    let existing = get_deal(pool, id).await?;

    let row = sqlx::query_as::<_, Deal>(
        r#"
        UPDATE deals SET
            customer_id = COALESCE($2, customer_id),
            company = COALESCE($3, company),
            contact = COALESCE($4, contact),
            email = COALESCE($5, email),
            value = COALESCE($6, value),
            product_id = COALESCE($7, product_id),
            product_name = COALESCE($8, product_name),
            product_type = COALESCE($9, product_type),
            deal_type = COALESCE($10, deal_type),
            date = COALESCE($11, date),
            license_key = COALESCE($12, license_key),
            notes = COALESCE($13, notes),
            usage_metric_label = COALESCE($14, usage_metric_label),
            usage_metric_value = COALESCE($15, usage_metric_value),
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&req.customer_id)
    .bind(&req.company)
    .bind(&req.contact)
    .bind(&req.email)
    .bind(&req.value)
    .bind(&req.product_id)
    .bind(&req.product_name)
    .bind(&req.product_type)
    .bind(&req.deal_type)
    .bind(&req.date)
    .bind(&req.license_key)
    .bind(&req.notes)
    .bind(&req.usage_metric_label)
    .bind(&req.usage_metric_value)
    .fetch_one(pool)
    .await?;

    if row.value != existing.value
        || row.deal_type != existing.deal_type
        || row.product_type != existing.product_type
    {
        emit_deal_reversal_and_replacement(pool, &existing, &row, "deal_update").await?;
    }

    Ok(row)
}

pub async fn delete_deal(pool: &PgPool, id: &str) -> Result<()> {
    let existing = get_deal(pool, id).await?;

    let result = sqlx::query("DELETE FROM deals WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(BillingError::not_found("deal", id));
    }

    emit_deal_reversal(pool, &existing, "deal_delete", None).await?;

    Ok(())
}

fn revision_source_id(deal: &Deal) -> String {
    format!("{}:{}", deal.id, deal.updated_at.format("%Y%m%d%H%M%S%6f"))
}

async fn find_latest_positive_booking_event(
    pool: &PgPool,
    deal_id: &str,
) -> Result<Option<(String, String)>> {
    let row = sqlx::query_as::<_, (String, String)>(
        r#"
        SELECT id, event_type
        FROM sales_events
        WHERE classification = 'bookings'
          AND amount_total > 0
          AND (
            (source_table = 'deals' AND source_id = $1)
            OR (metadata ->> 'deal_id' = $1)
          )
          AND event_type IN ('deal.created', 'deal.updated')
        ORDER BY occurred_at DESC, created_at DESC
        LIMIT 1
        "#,
    )
    .bind(deal_id)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

async fn find_event_id(
    pool: &PgPool,
    source_table: &str,
    source_id: &str,
    event_type: &str,
) -> Result<Option<String>> {
    let row = sqlx::query_scalar::<_, String>(
        r#"
        SELECT id
        FROM sales_events
        WHERE source_table = $1
          AND source_id = $2
          AND event_type = $3
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(source_table)
    .bind(source_id)
    .bind(event_type)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

async fn emit_deal_reversal(
    pool: &PgPool,
    old: &Deal,
    trigger: &str,
    superseded_by_event_id: Option<&str>,
) -> Result<()> {
    let prior_event = find_latest_positive_booking_event(pool, &old.id).await?;

    let mut metadata = serde_json::json!({
        "deal_id": old.id,
        "deal_type": old.deal_type,
        "product_type": old.product_type,
        "trigger": trigger,
        "reason": "deal_correction",
    });
    if let Some((event_id, event_type)) = prior_event {
        metadata["reversal_of_event_id"] = serde_json::json!(event_id);
        metadata["reversal_of_event_type"] = serde_json::json!(event_type);
    }
    if let Some(replacement_event_id) = superseded_by_event_id {
        metadata["superseded_by_event_id"] = serde_json::json!(replacement_event_id);
    }

    emit_sales_event(
        pool,
        NewSalesEvent {
            occurred_at: chrono::Utc::now(),
            event_type: "deal.reversal",
            classification: SalesClassification::Bookings,
            amount_subtotal: -old.value,
            amount_tax: Decimal::ZERO,
            amount_total: -old.value,
            currency: "USD",
            customer_id: old.customer_id.as_deref(),
            subscription_id: None,
            product_id: old.product_id.as_deref(),
            invoice_id: None,
            payment_id: None,
            source_table: "deal_revisions",
            source_id: &revision_source_id(old),
            metadata: Some(metadata),
        },
    )
    .await
}

async fn emit_deal_reversal_and_replacement(
    pool: &PgPool,
    old: &Deal,
    new: &Deal,
    trigger: &str,
) -> Result<()> {
    let previous_event = find_latest_positive_booking_event(pool, &old.id).await?;
    let replacement_source_id = revision_source_id(new);

    emit_sales_event(
        pool,
        NewSalesEvent {
            occurred_at: chrono::Utc::now(),
            event_type: "deal.updated",
            classification: SalesClassification::Bookings,
            amount_subtotal: new.value,
            amount_tax: Decimal::ZERO,
            amount_total: new.value,
            currency: "USD",
            customer_id: new.customer_id.as_deref(),
            subscription_id: None,
            product_id: new.product_id.as_deref(),
            invoice_id: None,
            payment_id: None,
            source_table: "deal_revisions",
            source_id: &replacement_source_id,
            metadata: Some(serde_json::json!({
                "deal_id": new.id,
                "deal_type": new.deal_type,
                "product_type": new.product_type,
                "trigger": trigger,
                "replaces_event_id": previous_event.as_ref().map(|(event_id, _)| event_id),
                "replaces_event_type": previous_event.as_ref().map(|(_, event_type)| event_type),
            })),
        },
    )
    .await?;

    let replacement_event_id = find_event_id(
        pool,
        "deal_revisions",
        &replacement_source_id,
        "deal.updated",
    )
    .await?;

    emit_deal_reversal(pool, old, trigger, replacement_event_id.as_deref()).await?;

    Ok(())
}
