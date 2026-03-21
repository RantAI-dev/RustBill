use super::repository::CustomersRepository;
use super::schema::{CreateCustomerRequest, UpdateCustomerRequest};
use crate::db::models::Customer;
use crate::error::{BillingError, Result};

pub async fn list_customers<R: CustomersRepository + ?Sized>(
    repo: &R,
) -> Result<Vec<serde_json::Value>> {
    let customers = repo.list_customers().await?;
    let mut results = Vec::with_capacity(customers.len());

    for customer in customers {
        let metrics = repo.customer_metrics(&customer.id).await?;

        let mut value = serde_json::to_value(&customer)
            .map_err(|err| BillingError::Internal(anyhow::anyhow!(err)))?;
        let object = value.as_object_mut().ok_or_else(|| {
            BillingError::bad_request("customer serialization returned non-object")
        })?;

        object.insert(
            "totalRevenue".to_string(),
            serde_json::json!(metrics.total_revenue.to_string()),
        );
        object.insert(
            "healthScore".to_string(),
            serde_json::json!(metrics.health_score),
        );
        object.insert("trend".to_string(), serde_json::json!(metrics.trend));
        if let Some(last_contact) = metrics.last_contact {
            object.insert("lastContact".to_string(), serde_json::json!(last_contact));
        }

        results.push(value);
    }

    Ok(results)
}

pub async fn get_customer<R: CustomersRepository + ?Sized>(repo: &R, id: &str) -> Result<Customer> {
    repo.get_customer(id).await
}

pub async fn create_customer<R: CustomersRepository + ?Sized>(
    repo: &R,
    req: CreateCustomerRequest,
) -> Result<Customer> {
    repo.create_customer(req).await
}

pub async fn update_customer<R: CustomersRepository + ?Sized>(
    repo: &R,
    id: &str,
    req: UpdateCustomerRequest,
) -> Result<Customer> {
    // Preserve existing semantics: explicit existence check before update.
    let _ = repo.get_customer(id).await?;
    repo.update_customer(id, req).await
}

pub async fn delete_customer<R: CustomersRepository + ?Sized>(repo: &R, id: &str) -> Result<()> {
    let affected = repo.delete_customer(id).await?;
    if affected == 0 {
        return Err(BillingError::not_found("customer", id));
    }

    Ok(())
}
