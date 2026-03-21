use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OneTimeSaleItemInput {
    pub description: Option<String>,
    pub quantity: Option<f64>,
    #[serde(alias = "unit_price")]
    pub unit_price: Option<f64>,
    pub amount: Option<f64>,
}

impl OneTimeSaleItemInput {
    pub fn normalized_description(&self) -> Option<&str> {
        self.description
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }

    pub fn normalized_quantity(&self) -> f64 {
        self.quantity.unwrap_or(1.0)
    }

    pub fn normalized_unit_price(&self) -> f64 {
        self.unit_price.unwrap_or(0.0)
    }

    pub fn normalized_amount(&self) -> f64 {
        self.amount
            .unwrap_or_else(|| self.normalized_quantity() * self.normalized_unit_price())
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateOneTimeSaleRequest {
    pub customer_id: String,
    pub status: Option<String>,
    pub currency: Option<String>,
    pub subtotal: Option<f64>,
    pub tax: Option<f64>,
    pub total: Option<f64>,
    pub due_at: Option<String>,
    pub issued_at: Option<String>,
    pub notes: Option<String>,
    pub items: Option<Vec<OneTimeSaleItemInput>>,
}

impl CreateOneTimeSaleRequest {
    pub fn normalized_status(&self) -> &str {
        self.status.as_deref().unwrap_or("issued")
    }

    pub fn normalized_currency(&self) -> &str {
        self.currency.as_deref().unwrap_or("USD")
    }

    pub fn normalized_tax(&self) -> f64 {
        self.tax.unwrap_or(0.0)
    }

    pub fn normalized_items(&self) -> &[OneTimeSaleItemInput] {
        self.items.as_deref().unwrap_or(&[])
    }

    pub fn normalized_subtotal(&self) -> f64 {
        let subtotal = self.subtotal.unwrap_or(0.0);
        if subtotal > 0.0 {
            return subtotal;
        }

        self.normalized_items()
            .iter()
            .map(|item| item.normalized_quantity() * item.normalized_unit_price())
            .sum()
    }

    pub fn normalized_total(&self, subtotal: f64) -> f64 {
        self.total.unwrap_or(subtotal + self.normalized_tax())
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOneTimeSaleRequest {
    pub status: Option<String>,
    pub notes: Option<String>,
    pub due_at: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OneTimeSalesListParams {
    pub status: Option<String>,
    pub customer_id: Option<String>,
}
