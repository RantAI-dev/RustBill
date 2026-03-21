use super::repository::ProductsRepository;
use super::schema::{CreateProductRequest, UpdateProductRequest};
use crate::db::models::Product;
use crate::error::{BillingError, Result};

pub async fn list_products<R: ProductsRepository + ?Sized>(
    repo: &R,
) -> Result<Vec<serde_json::Value>> {
    let products = repo.list_products().await?;
    let mut results = Vec::with_capacity(products.len());

    for product in products {
        let metrics = repo
            .product_metrics(&product.id, &product.product_type)
            .await?;

        let mut value = serde_json::to_value(&product)
            .map_err(|err| BillingError::Internal(anyhow::anyhow!(err)))?;
        let object = value.as_object_mut().ok_or_else(|| {
            BillingError::bad_request("product serialization returned non-object")
        })?;

        object.insert(
            "revenue".to_string(),
            serde_json::json!(metrics.revenue.to_string()),
        );
        object.insert(
            "change".to_string(),
            serde_json::json!(metrics.change.to_string()),
        );

        if let Some(active) = metrics.active_licenses {
            object.insert("activeLicenses".to_string(), serde_json::json!(active));
        }
        if let Some(total) = metrics.total_licenses {
            object.insert("totalLicenses".to_string(), serde_json::json!(total));
        }

        results.push(value);
    }

    Ok(results)
}

pub async fn get_product<R: ProductsRepository + ?Sized>(repo: &R, id: &str) -> Result<Product> {
    repo.get_product(id).await
}

pub async fn create_product<R: ProductsRepository + ?Sized>(
    repo: &R,
    req: CreateProductRequest,
) -> Result<Product> {
    repo.create_product(req).await
}

pub async fn update_product<R: ProductsRepository + ?Sized>(
    repo: &R,
    id: &str,
    req: UpdateProductRequest,
) -> Result<Product> {
    let _ = repo.get_product(id).await?;
    repo.update_product(id, req).await
}

pub async fn delete_product<R: ProductsRepository + ?Sized>(repo: &R, id: &str) -> Result<()> {
    let affected = repo.delete_product(id).await?;
    if affected == 0 {
        return Err(BillingError::not_found("product", id));
    }

    Ok(())
}
