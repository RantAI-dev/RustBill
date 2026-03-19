use crate::db::models::{Deal, ProductType};
use crate::deals::validation::{CreateDealRequest, UpdateDealRequest};
use crate::error::{BillingError, Result};
use rand::Rng;
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
        query.push_str(&format!(" AND product_type = ${}::product_type", binds.len()));
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

pub async fn update_deal(pool: &PgPool, id: &str, req: UpdateDealRequest) -> Result<Deal> {
    // Ensure exists
    let _existing = get_deal(pool, id).await?;

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

    Ok(row)
}

pub async fn delete_deal(pool: &PgPool, id: &str) -> Result<()> {
    let result = sqlx::query("DELETE FROM deals WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(BillingError::not_found("deal", id));
    }
    Ok(())
}
