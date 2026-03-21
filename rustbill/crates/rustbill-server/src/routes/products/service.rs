use super::repository::ProductsRepository;
use super::schema::{CreateProductRequest, ProductListItem, UpdateProductRequest};
use rustbill_core::db::models::Product;
use rustbill_core::error::BillingError;

pub async fn list<R: ProductsRepository>(repo: &R) -> Result<Vec<ProductListItem>, BillingError> {
    repo.list().await
}

pub async fn get<R: ProductsRepository>(repo: &R, id: &str) -> Result<Product, BillingError> {
    repo.get(id).await
}

pub async fn create<R: ProductsRepository>(
    repo: &R,
    body: &CreateProductRequest,
) -> Result<Product, BillingError> {
    repo.create(body).await
}

pub async fn update<R: ProductsRepository>(
    repo: &R,
    id: &str,
    body: &UpdateProductRequest,
) -> Result<Product, BillingError> {
    repo.update(id, body).await
}

pub async fn delete<R: ProductsRepository>(
    repo: &R,
    id: &str,
) -> Result<serde_json::Value, BillingError> {
    let affected = repo.delete(id).await?;
    if affected == 0 {
        return Err(BillingError::not_found("product", id));
    }

    Ok(serde_json::json!({ "success": true }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routes::products::repository::ProductsRepository;
    use async_trait::async_trait;
    use chrono::NaiveDate;
    use rust_decimal::Decimal;
    use rustbill_core::db::models::{Product, ProductType};

    struct MockRepo {
        list_rows: Vec<ProductListItem>,
        product: Product,
        delete_rows: u64,
    }

    impl MockRepo {
        fn new(list_rows: Vec<ProductListItem>, delete_rows: u64) -> Self {
            Self {
                list_rows,
                product: sample_product("prod-1"),
                delete_rows,
            }
        }
    }

    #[async_trait]
    impl ProductsRepository for MockRepo {
        async fn list(&self) -> Result<Vec<ProductListItem>, BillingError> {
            Ok(self.list_rows.clone())
        }

        async fn get(&self, _id: &str) -> Result<Product, BillingError> {
            Ok(self.product.clone())
        }

        async fn create(&self, _body: &CreateProductRequest) -> Result<Product, BillingError> {
            Ok(self.product.clone())
        }

        async fn update(
            &self,
            _id: &str,
            _body: &UpdateProductRequest,
        ) -> Result<Product, BillingError> {
            Ok(self.product.clone())
        }

        async fn delete(&self, _id: &str) -> Result<u64, BillingError> {
            Ok(self.delete_rows)
        }
    }

    #[tokio::test]
    async fn list_forwards_repository_rows() {
        let row = ProductListItem {
            product: sample_product("prod-1"),
            revenue: "25".to_string(),
            change: "10".to_string(),
            active_licenses: Some(3),
            total_licenses: Some(4),
        };
        let repo = MockRepo::new(vec![row.clone()], 1);

        let rows = list(&repo).await;

        assert!(rows.is_ok());
        let rows = match rows {
            Ok(rows) => rows,
            Err(err) => panic!("unexpected error: {err}"),
        };
        assert_eq!(rows.len(), 1);
        let got = &rows[0];
        assert_eq!(got.product.id, row.product.id);
        assert_eq!(got.revenue, row.revenue);
        assert_eq!(got.change, row.change);
        assert_eq!(got.active_licenses, row.active_licenses);
        assert_eq!(got.total_licenses, row.total_licenses);
    }

    #[tokio::test]
    async fn delete_maps_zero_rows_to_not_found() {
        let repo = MockRepo::new(vec![], 0);
        let result = delete(&repo, "prod-1").await;

        assert!(
            matches!(result, Err(BillingError::NotFound { entity: "product", id }) if id == "prod-1")
        );
    }

    fn sample_product(id: &str) -> Product {
        let timestamp = NaiveDate::from_ymd_opt(2025, 1, 1)
            .and_then(|date| date.and_hms_opt(0, 0, 0))
            .expect("valid timestamp");

        Product {
            id: id.to_string(),
            name: "Product".to_string(),
            product_type: ProductType::Licensed,
            revenue: Decimal::new(100, 0),
            target: Decimal::new(50, 0),
            change: Decimal::new(10, 0),
            units_sold: Some(3),
            active_licenses: Some(4),
            total_licenses: Some(5),
            mau: None,
            dau: None,
            free_users: None,
            paid_users: None,
            churn_rate: None,
            api_calls: None,
            active_developers: None,
            avg_latency: None,
            created_at: timestamp,
            updated_at: timestamp,
        }
    }
}
