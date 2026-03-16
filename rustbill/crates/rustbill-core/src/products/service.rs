use crate::db::models::Product;
use crate::error::{BillingError, Result};
use crate::products::validation::{CreateProductRequest, UpdateProductRequest};
use rust_decimal::Decimal;
use sqlx::PgPool;

pub async fn list_products(pool: &PgPool) -> Result<Vec<serde_json::Value>> {
    // Match existing behavior: compute revenue from deals, MoM change, license counts
    let rows = sqlx::query_as::<_, Product>("SELECT * FROM products ORDER BY created_at DESC")
        .fetch_all(pool)
        .await?;

    let mut results = Vec::with_capacity(rows.len());
    for p in rows {
        // Compute revenue from deals
        let revenue: Option<Decimal> =
            sqlx::query_scalar("SELECT COALESCE(SUM(value), 0) FROM deals WHERE product_id = $1")
                .bind(&p.id)
                .fetch_one(pool)
                .await?;

        // Compute MoM change
        let this_month: Option<Decimal> = sqlx::query_scalar(
            "SELECT COALESCE(SUM(value), 0) FROM deals WHERE product_id = $1 AND created_at >= date_trunc('month', CURRENT_DATE)"
        )
        .bind(&p.id)
        .fetch_one(pool)
        .await?;

        let last_month: Option<Decimal> = sqlx::query_scalar(
            "SELECT COALESCE(SUM(value), 0) FROM deals WHERE product_id = $1 AND created_at >= date_trunc('month', CURRENT_DATE) - interval '1 month' AND created_at < date_trunc('month', CURRENT_DATE)"
        )
        .bind(&p.id)
        .fetch_one(pool)
        .await?;

        let change = if let (Some(tm), Some(lm)) = (this_month, last_month) {
            if lm > Decimal::ZERO {
                ((tm - lm) / lm * Decimal::from(100)).round_dp(2)
            } else if tm > Decimal::ZERO {
                Decimal::from(100)
            } else {
                Decimal::ZERO
            }
        } else {
            Decimal::ZERO
        };

        let mut val = serde_json::to_value(&p).unwrap();
        let obj = val.as_object_mut().unwrap();
        obj.insert(
            "revenue".to_string(),
            serde_json::json!(revenue.unwrap_or_default().to_string()),
        );
        obj.insert("change".to_string(), serde_json::json!(change.to_string()));

        // License counts for licensed products
        if p.product_type == crate::db::models::ProductType::Licensed {
            let active: Option<i64> = sqlx::query_scalar(
                "SELECT COUNT(*) FROM licenses WHERE product_id = $1 AND status = 'active'",
            )
            .bind(&p.id)
            .fetch_one(pool)
            .await?;

            let total: Option<i64> =
                sqlx::query_scalar("SELECT COUNT(*) FROM licenses WHERE product_id = $1")
                    .bind(&p.id)
                    .fetch_one(pool)
                    .await?;

            obj.insert(
                "activeLicenses".to_string(),
                serde_json::json!(active.unwrap_or(0)),
            );
            obj.insert(
                "totalLicenses".to_string(),
                serde_json::json!(total.unwrap_or(0)),
            );
        }

        results.push(val);
    }

    Ok(results)
}

pub async fn get_product(pool: &PgPool, id: &str) -> Result<Product> {
    sqlx::query_as::<_, Product>("SELECT * FROM products WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| BillingError::not_found("product", id))
}

pub async fn create_product(pool: &PgPool, req: CreateProductRequest) -> Result<Product> {
    let row = sqlx::query_as::<_, Product>(
        r#"
        INSERT INTO products (id, name, product_type, revenue, target, change,
            units_sold, active_licenses, total_licenses,
            mau, dau, free_users, paid_users, churn_rate,
            api_calls, active_developers, avg_latency)
        VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5,
            $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
        RETURNING *
        "#,
    )
    .bind(&req.name)
    .bind(&req.product_type)
    .bind(req.revenue.unwrap_or_default())
    .bind(req.target.unwrap_or_default())
    .bind(req.change.unwrap_or_default())
    .bind(req.units_sold)
    .bind(req.active_licenses)
    .bind(req.total_licenses)
    .bind(req.mau)
    .bind(req.dau)
    .bind(req.free_users)
    .bind(req.paid_users)
    .bind(req.churn_rate)
    .bind(req.api_calls)
    .bind(req.active_developers)
    .bind(req.avg_latency)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn update_product(pool: &PgPool, id: &str, req: UpdateProductRequest) -> Result<Product> {
    // Ensure exists
    let _existing = get_product(pool, id).await?;

    let row = sqlx::query_as::<_, Product>(
        r#"
        UPDATE products SET
            name = COALESCE($2, name),
            product_type = COALESCE($3, product_type),
            revenue = COALESCE($4, revenue),
            target = COALESCE($5, target),
            change = COALESCE($6, change),
            units_sold = COALESCE($7, units_sold),
            active_licenses = COALESCE($8, active_licenses),
            total_licenses = COALESCE($9, total_licenses),
            mau = COALESCE($10, mau),
            dau = COALESCE($11, dau),
            free_users = COALESCE($12, free_users),
            paid_users = COALESCE($13, paid_users),
            churn_rate = COALESCE($14, churn_rate),
            api_calls = COALESCE($15, api_calls),
            active_developers = COALESCE($16, active_developers),
            avg_latency = COALESCE($17, avg_latency),
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&req.name)
    .bind(&req.product_type)
    .bind(req.revenue)
    .bind(req.target)
    .bind(req.change)
    .bind(req.units_sold)
    .bind(req.active_licenses)
    .bind(req.total_licenses)
    .bind(req.mau)
    .bind(req.dau)
    .bind(req.free_users)
    .bind(req.paid_users)
    .bind(req.churn_rate)
    .bind(req.api_calls)
    .bind(req.active_developers)
    .bind(req.avg_latency)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn delete_product(pool: &PgPool, id: &str) -> Result<()> {
    let result = sqlx::query("DELETE FROM products WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(BillingError::not_found("product", id));
    }
    Ok(())
}
