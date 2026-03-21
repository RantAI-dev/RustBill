use super::schema::{ActivationOutcome, CreateLicenseDraft, UpdateLicenseDraft};
use crate::db::models::{License, LicenseActivation};
use crate::error::Result;
use async_trait::async_trait;
use sqlx::PgPool;

#[async_trait]
pub trait LicensesRepository {
    async fn list_licenses(&self) -> Result<Vec<License>>;
    async fn activation_count(&self, license_key: &str) -> Result<i64>;
    async fn get_license(&self, key: &str) -> Result<Option<License>>;
    async fn customer_name(&self, customer_id: &str) -> Result<Option<String>>;
    async fn product_name(&self, product_id: &str) -> Result<Option<String>>;
    async fn create_license(&self, draft: CreateLicenseDraft) -> Result<License>;
    async fn update_license(&self, key: &str, draft: UpdateLicenseDraft) -> Result<License>;
    async fn delete_license(&self, key: &str) -> Result<u64>;
    async fn list_activations(&self, license_key: &str) -> Result<Vec<LicenseActivation>>;
    async fn deactivate_device(&self, license_key: &str, device_id: &str) -> Result<u64>;
    async fn get_keypair(&self) -> Result<Option<(String, String)>>;
    async fn store_public_key(&self, public_pem: &str) -> Result<()>;
    async fn store_private_key(&self, private_pem: &str) -> Result<()>;
    async fn record_activation(
        &self,
        license_key: &str,
        device_id: &str,
        device_name: Option<&str>,
        ip_address: Option<&str>,
        max_activations: Option<i32>,
    ) -> Result<ActivationOutcome>;
    async fn update_signed_license(
        &self,
        key: &str,
        signed_payload: &str,
        signature: &str,
    ) -> Result<License>;
}

pub struct PgLicensesRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> PgLicensesRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl LicensesRepository for PgLicensesRepository<'_> {
    async fn list_licenses(&self) -> Result<Vec<License>> {
        let rows = sqlx::query_as::<_, License>("SELECT * FROM licenses ORDER BY created_at DESC")
            .fetch_all(self.pool)
            .await?;
        Ok(rows)
    }

    async fn activation_count(&self, license_key: &str) -> Result<i64> {
        let count: Option<i64> =
            sqlx::query_scalar("SELECT COUNT(*) FROM license_activations WHERE license_key = $1")
                .bind(license_key)
                .fetch_one(self.pool)
                .await?;
        Ok(count.unwrap_or(0))
    }

    async fn get_license(&self, key: &str) -> Result<Option<License>> {
        let row = sqlx::query_as::<_, License>("SELECT * FROM licenses WHERE key = $1")
            .bind(key)
            .fetch_optional(self.pool)
            .await?;
        Ok(row)
    }

    async fn customer_name(&self, customer_id: &str) -> Result<Option<String>> {
        let name: Option<String> = sqlx::query_scalar("SELECT name FROM customers WHERE id = $1")
            .bind(customer_id)
            .fetch_optional(self.pool)
            .await?;
        Ok(name)
    }

    async fn product_name(&self, product_id: &str) -> Result<Option<String>> {
        let name: Option<String> = sqlx::query_scalar("SELECT name FROM products WHERE id = $1")
            .bind(product_id)
            .fetch_optional(self.pool)
            .await?;
        Ok(name)
    }

    async fn create_license(&self, draft: CreateLicenseDraft) -> Result<License> {
        let row = sqlx::query_as::<_, License>(
            r#"
            INSERT INTO licenses (
                key, customer_id, customer_name, product_id, product_name,
                status, created_at, expires_at, license_type,
                features, max_activations
            )
            VALUES (
                gen_random_uuid()::text, $1, $2, $3, $4,
                $5, $6, $7, $8,
                $9, $10
            )
            RETURNING *
            "#,
        )
        .bind(&draft.customer_id)
        .bind(&draft.customer_name)
        .bind(&draft.product_id)
        .bind(&draft.product_name)
        .bind(&draft.status)
        .bind(&draft.created_at)
        .bind(&draft.expires_at)
        .bind(&draft.license_type)
        .bind(&draft.features)
        .bind(draft.max_activations)
        .fetch_one(self.pool)
        .await?;

        Ok(row)
    }

    async fn update_license(&self, key: &str, draft: UpdateLicenseDraft) -> Result<License> {
        let row = sqlx::query_as::<_, License>(
            r#"
            UPDATE licenses SET
                customer_id = COALESCE($2, customer_id),
                customer_name = COALESCE($3, customer_name),
                product_id = COALESCE($4, product_id),
                product_name = COALESCE($5, product_name),
                status = COALESCE($6, status),
                expires_at = COALESCE($7, expires_at),
                license_type = COALESCE($8, license_type),
                features = COALESCE($9, features),
                max_activations = COALESCE($10, max_activations)
            WHERE key = $1
            RETURNING *
            "#,
        )
        .bind(key)
        .bind(&draft.customer_id)
        .bind(&draft.customer_name)
        .bind(&draft.product_id)
        .bind(&draft.product_name)
        .bind(&draft.status)
        .bind(&draft.expires_at)
        .bind(&draft.license_type)
        .bind(&draft.features)
        .bind(draft.max_activations)
        .fetch_one(self.pool)
        .await?;

        Ok(row)
    }

    async fn delete_license(&self, key: &str) -> Result<u64> {
        let result = sqlx::query("DELETE FROM licenses WHERE key = $1")
            .bind(key)
            .execute(self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    async fn list_activations(&self, license_key: &str) -> Result<Vec<LicenseActivation>> {
        let rows = sqlx::query_as::<_, LicenseActivation>(
            "SELECT * FROM license_activations WHERE license_key = $1 ORDER BY activated_at DESC",
        )
        .bind(license_key)
        .fetch_all(self.pool)
        .await?;

        Ok(rows)
    }

    async fn deactivate_device(&self, license_key: &str, device_id: &str) -> Result<u64> {
        let result = sqlx::query(
            "DELETE FROM license_activations WHERE license_key = $1 AND device_id = $2",
        )
        .bind(license_key)
        .bind(device_id)
        .execute(self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    async fn get_keypair(&self) -> Result<Option<(String, String)>> {
        let public_pem: Option<String> = sqlx::query_scalar(
            "SELECT value FROM system_settings WHERE key = 'license_public_key'",
        )
        .fetch_optional(self.pool)
        .await?;

        let private_pem: Option<String> = sqlx::query_scalar(
            "SELECT value FROM system_settings WHERE key = 'license_private_key'",
        )
        .fetch_optional(self.pool)
        .await?;

        match (public_pem, private_pem) {
            (Some(public_pem), Some(private_pem)) => Ok(Some((public_pem, private_pem))),
            _ => Ok(None),
        }
    }

    async fn store_public_key(&self, public_pem: &str) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO system_settings (key, value, sensitive, updated_at)
            VALUES ('license_public_key', $1, false, NOW())
            ON CONFLICT (key) DO UPDATE SET value = $1, updated_at = NOW()
            "#,
        )
        .bind(public_pem)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    async fn store_private_key(&self, private_pem: &str) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO system_settings (key, value, sensitive, updated_at)
            VALUES ('license_private_key', $1, true, NOW())
            ON CONFLICT (key) DO UPDATE SET value = $1, updated_at = NOW()
            "#,
        )
        .bind(private_pem)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    async fn record_activation(
        &self,
        license_key: &str,
        device_id: &str,
        device_name: Option<&str>,
        ip_address: Option<&str>,
        max_activations: Option<i32>,
    ) -> Result<ActivationOutcome> {
        let mut tx = self.pool.begin().await?;

        let existing: Option<LicenseActivation> = sqlx::query_as(
            "SELECT * FROM license_activations WHERE license_key = $1 AND device_id = $2",
        )
        .bind(license_key)
        .bind(device_id)
        .fetch_optional(&mut *tx)
        .await?;

        if existing.is_some() {
            sqlx::query(
                "UPDATE license_activations SET last_seen_at = NOW() WHERE license_key = $1 AND device_id = $2",
            )
            .bind(license_key)
            .bind(device_id)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            return Ok(ActivationOutcome::Updated);
        }

        if let Some(max) = max_activations {
            let current_count: Option<i64> = sqlx::query_scalar(
                "SELECT COUNT(*) FROM license_activations WHERE license_key = $1",
            )
            .bind(license_key)
            .fetch_one(&mut *tx)
            .await?;

            if current_count.unwrap_or(0) >= max as i64 {
                tx.rollback().await?;
                return Ok(ActivationOutcome::LimitReached);
            }
        }

        sqlx::query(
            r#"
            INSERT INTO license_activations (id, license_key, device_id, device_name, ip_address, activated_at, last_seen_at)
            VALUES (gen_random_uuid()::text, $1, $2, $3, $4, NOW(), NOW())
            "#,
        )
        .bind(license_key)
        .bind(device_id)
        .bind(device_name)
        .bind(ip_address)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(ActivationOutcome::Inserted)
    }

    async fn update_signed_license(
        &self,
        key: &str,
        signed_payload: &str,
        signature: &str,
    ) -> Result<License> {
        let row = sqlx::query_as::<_, License>(
            r#"
            UPDATE licenses
            SET signed_payload = $2, signature = $3
            WHERE key = $1
            RETURNING *
            "#,
        )
        .bind(key)
        .bind(signed_payload)
        .bind(signature)
        .fetch_one(self.pool)
        .await?;

        Ok(row)
    }
}
