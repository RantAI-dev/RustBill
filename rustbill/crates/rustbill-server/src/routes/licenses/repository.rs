use async_trait::async_trait;
use rustbill_core::db::models::{License, LicenseActivation};
use rustbill_core::error::BillingError;
use sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct NewLicenseRecord {
    pub key: String,
    pub customer_id: Option<String>,
    pub customer_name: Option<String>,
    pub product_id: Option<String>,
    pub product_name: Option<String>,
    pub created_at: Option<String>,
    pub expires_at: Option<String>,
    pub license_type: Option<String>,
    pub features: Option<serde_json::Value>,
    pub max_activations: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct LicensePatch {
    pub status: Option<String>,
    pub customer_name: Option<String>,
    pub product_name: Option<String>,
    pub max_activations: Option<i32>,
    pub created_at: Option<String>,
    pub expires_at: Option<String>,
    pub license_type: Option<String>,
    pub features: Option<serde_json::Value>,
}

#[async_trait]
pub trait LicensesRepository: Send + Sync {
    async fn list_licenses(&self, status: Option<&str>) -> Result<Vec<License>, BillingError>;
    async fn get_license(&self, key: &str) -> Result<Option<License>, BillingError>;
    async fn insert_license(&self, record: &NewLicenseRecord) -> Result<License, BillingError>;
    async fn update_license(
        &self,
        key: &str,
        patch: &LicensePatch,
    ) -> Result<Option<License>, BillingError>;
    async fn delete_license(&self, key: &str) -> Result<u64, BillingError>;
    async fn list_activations(&self, key: &str) -> Result<Vec<LicenseActivation>, BillingError>;
    async fn delete_activation(&self, key: &str, device_id: &str) -> Result<u64, BillingError>;
    async fn get_keypair(&self) -> Result<Option<(String, String)>, BillingError>;
    async fn store_keypair(&self, public_pem: &str, private_pem: &str) -> Result<(), BillingError>;
    async fn count_activations(&self, key: &str) -> Result<i64, BillingError>;
    async fn find_activation(
        &self,
        key: &str,
        device_id: &str,
    ) -> Result<Option<LicenseActivation>, BillingError>;
    async fn insert_activation(
        &self,
        key: &str,
        device_id: &str,
        device_name: Option<&str>,
        ip_address: Option<&str>,
    ) -> Result<(), BillingError>;
    async fn update_activation_last_seen(
        &self,
        key: &str,
        device_id: &str,
    ) -> Result<(), BillingError>;
    async fn store_signed_license(
        &self,
        key: &str,
        signed_payload: &str,
        signature: &str,
    ) -> Result<License, BillingError>;
}

#[derive(Clone)]
pub struct SqlxLicensesRepository {
    pool: PgPool,
}

impl SqlxLicensesRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl LicensesRepository for SqlxLicensesRepository {
    async fn list_licenses(&self, status: Option<&str>) -> Result<Vec<License>, BillingError> {
        sqlx::query_as::<_, License>(
            r#"SELECT *
               FROM licenses
               WHERE ($1::text IS NULL OR status::text = $1)
               ORDER BY created_at DESC"#,
        )
        .bind(status)
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn get_license(&self, key: &str) -> Result<Option<License>, BillingError> {
        sqlx::query_as::<_, License>("SELECT * FROM licenses WHERE key = $1")
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .map_err(BillingError::from)
    }

    async fn insert_license(&self, record: &NewLicenseRecord) -> Result<License, BillingError> {
        sqlx::query_as::<_, License>(
            r#"INSERT INTO licenses
               (key, customer_id, customer_name, product_id, product_name, status, created_at, expires_at, license_type, features, max_activations)
               VALUES ($1, $2, COALESCE($3, ''), $4, COALESCE($5, ''), 'active', COALESCE($6, to_char(now(), 'YYYY-MM-DD')), COALESCE($7, ''), COALESCE($8, 'simple'), $9, $10)
               RETURNING *"#,
        )
        .bind(&record.key)
        .bind(record.customer_id.as_deref())
        .bind(record.customer_name.as_deref())
        .bind(record.product_id.as_deref())
        .bind(record.product_name.as_deref())
        .bind(record.created_at.as_deref())
        .bind(record.expires_at.as_deref())
        .bind(record.license_type.as_deref())
        .bind(record.features.as_ref())
        .bind(record.max_activations)
        .fetch_one(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn update_license(
        &self,
        key: &str,
        patch: &LicensePatch,
    ) -> Result<Option<License>, BillingError> {
        sqlx::query_as::<_, License>(
            r#"UPDATE licenses SET
                 status = COALESCE($2::license_status, status),
                 customer_name = COALESCE($3, customer_name),
                 product_name = COALESCE($4, product_name),
                 max_activations = COALESCE($5, max_activations),
                 created_at = COALESCE($6, created_at),
                 expires_at = COALESCE($7, expires_at),
                 license_type = COALESCE($8, license_type),
                 features = COALESCE($9, features)
               WHERE key = $1
               RETURNING *"#,
        )
        .bind(key)
        .bind(patch.status.as_deref())
        .bind(patch.customer_name.as_deref())
        .bind(patch.product_name.as_deref())
        .bind(patch.max_activations)
        .bind(patch.created_at.as_deref())
        .bind(patch.expires_at.as_deref())
        .bind(patch.license_type.as_deref())
        .bind(patch.features.as_ref())
        .fetch_optional(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn delete_license(&self, key: &str) -> Result<u64, BillingError> {
        let result = sqlx::query("DELETE FROM licenses WHERE key = $1")
            .bind(key)
            .execute(&self.pool)
            .await
            .map_err(BillingError::from)?;

        Ok(result.rows_affected())
    }

    async fn list_activations(&self, key: &str) -> Result<Vec<LicenseActivation>, BillingError> {
        sqlx::query_as::<_, LicenseActivation>(
            r#"SELECT *
               FROM license_activations
               WHERE license_key = $1
               ORDER BY activated_at DESC"#,
        )
        .bind(key)
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn delete_activation(&self, key: &str, device_id: &str) -> Result<u64, BillingError> {
        let result = sqlx::query(
            r#"DELETE FROM license_activations
               WHERE license_key = $1
                 AND device_id = $2"#,
        )
        .bind(key)
        .bind(device_id)
        .execute(&self.pool)
        .await
        .map_err(BillingError::from)?;

        Ok(result.rows_affected())
    }

    async fn get_keypair(&self) -> Result<Option<(String, String)>, BillingError> {
        let public_pem: Option<String> = sqlx::query_scalar(
            "SELECT value FROM system_settings WHERE key = 'license_public_key'",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(BillingError::from)?;

        let private_pem: Option<String> = sqlx::query_scalar(
            "SELECT value FROM system_settings WHERE key = 'license_private_key'",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(BillingError::from)?;

        match (public_pem, private_pem) {
            (Some(public_pem), Some(private_pem)) => Ok(Some((public_pem, private_pem))),
            _ => Ok(None),
        }
    }

    async fn store_keypair(&self, public_pem: &str, private_pem: &str) -> Result<(), BillingError> {
        sqlx::query(
            r#"INSERT INTO system_settings (key, value, sensitive, updated_at)
               VALUES ('license_public_key', $1, false, NOW())
               ON CONFLICT (key) DO UPDATE SET value = $1, updated_at = NOW()"#,
        )
        .bind(public_pem)
        .execute(&self.pool)
        .await
        .map_err(BillingError::from)?;

        sqlx::query(
            r#"INSERT INTO system_settings (key, value, sensitive, updated_at)
               VALUES ('license_private_key', $1, true, NOW())
               ON CONFLICT (key) DO UPDATE SET value = $1, updated_at = NOW()"#,
        )
        .bind(private_pem)
        .execute(&self.pool)
        .await
        .map_err(BillingError::from)?;

        Ok(())
    }

    async fn count_activations(&self, key: &str) -> Result<i64, BillingError> {
        sqlx::query_scalar("SELECT COUNT(*) FROM license_activations WHERE license_key = $1")
            .bind(key)
            .fetch_one(&self.pool)
            .await
            .map_err(BillingError::from)
    }

    async fn find_activation(
        &self,
        key: &str,
        device_id: &str,
    ) -> Result<Option<LicenseActivation>, BillingError> {
        sqlx::query_as::<_, LicenseActivation>(
            "SELECT * FROM license_activations WHERE license_key = $1 AND device_id = $2",
        )
        .bind(key)
        .bind(device_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn insert_activation(
        &self,
        key: &str,
        device_id: &str,
        device_name: Option<&str>,
        ip_address: Option<&str>,
    ) -> Result<(), BillingError> {
        sqlx::query(
            r#"INSERT INTO license_activations
               (id, license_key, device_id, device_name, ip_address, activated_at, last_seen_at)
               VALUES (gen_random_uuid()::text, $1, $2, $3, $4, NOW(), NOW())"#,
        )
        .bind(key)
        .bind(device_id)
        .bind(device_name)
        .bind(ip_address)
        .execute(&self.pool)
        .await
        .map_err(BillingError::from)?;

        Ok(())
    }

    async fn update_activation_last_seen(
        &self,
        key: &str,
        device_id: &str,
    ) -> Result<(), BillingError> {
        sqlx::query(
            "UPDATE license_activations SET last_seen_at = NOW() WHERE license_key = $1 AND device_id = $2",
        )
        .bind(key)
        .bind(device_id)
        .execute(&self.pool)
        .await
        .map_err(BillingError::from)?;

        Ok(())
    }

    async fn store_signed_license(
        &self,
        key: &str,
        signed_payload: &str,
        signature: &str,
    ) -> Result<License, BillingError> {
        sqlx::query_as::<_, License>(
            r#"UPDATE licenses
               SET signed_payload = $2, signature = $3
               WHERE key = $1
               RETURNING *"#,
        )
        .bind(key)
        .bind(signed_payload)
        .bind(signature)
        .fetch_one(&self.pool)
        .await
        .map_err(BillingError::from)
    }
}
