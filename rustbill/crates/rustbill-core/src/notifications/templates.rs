//! HTML email templates for billing notifications.

pub fn invoice_created(
    customer_name: &str,
    invoice_number: &str,
    total: &str,
    currency: &str,
) -> (String, String) {
    let subject = format!("New Invoice {invoice_number}");
    let html = format!(
        r#"<div style="font-family:sans-serif;max-width:600px;margin:0 auto">
        <h2>Invoice {invoice_number}</h2>
        <p>Hi {customer_name},</p>
        <p>A new invoice has been created for your account.</p>
        <p><strong>Amount:</strong> {currency} {total}</p>
        <p>Please log in to your dashboard to view and pay this invoice.</p>
        <p>— RantAI Billing</p>
        </div>"#
    );
    (subject, html)
}

pub fn invoice_paid(
    customer_name: &str,
    invoice_number: &str,
    total: &str,
    currency: &str,
) -> (String, String) {
    let subject = format!("Payment Received — Invoice {invoice_number}");
    let html = format!(
        r#"<div style="font-family:sans-serif;max-width:600px;margin:0 auto">
        <h2>Payment Confirmed</h2>
        <p>Hi {customer_name},</p>
        <p>We've received your payment of <strong>{currency} {total}</strong> for invoice {invoice_number}.</p>
        <p>Thank you for your payment!</p>
        <p>— RantAI Billing</p>
        </div>"#
    );
    (subject, html)
}

pub fn payment_received(customer_name: &str, amount: &str, method: &str) -> (String, String) {
    let subject = "Payment Received".to_string();
    let html = format!(
        r#"<div style="font-family:sans-serif;max-width:600px;margin:0 auto">
        <h2>Payment Received</h2>
        <p>Hi {customer_name},</p>
        <p>We've received a payment of <strong>{amount}</strong> via {method}.</p>
        <p>— RantAI Billing</p>
        </div>"#
    );
    (subject, html)
}

pub fn invoice_overdue(
    customer_name: &str,
    invoice_number: &str,
    total: &str,
    currency: &str,
) -> (String, String) {
    let subject = format!("Invoice {invoice_number} is Overdue");
    let html = format!(
        r#"<div style="font-family:sans-serif;max-width:600px;margin:0 auto">
        <h2>Invoice Overdue</h2>
        <p>Hi {customer_name},</p>
        <p>Invoice {invoice_number} for <strong>{currency} {total}</strong> is now overdue.</p>
        <p>Please make payment at your earliest convenience to avoid service interruption.</p>
        <p>— RantAI Billing</p>
        </div>"#
    );
    (subject, html)
}

pub fn dunning_reminder(customer_name: &str, invoice_number: &str, step: &str) -> (String, String) {
    let subject = format!("Payment Reminder — Invoice {invoice_number}");
    let html = format!(
        r#"<div style="font-family:sans-serif;max-width:600px;margin:0 auto">
        <h2>Payment {step}</h2>
        <p>Hi {customer_name},</p>
        <p>This is a {step} regarding your outstanding invoice {invoice_number}.</p>
        <p>Please log in to your dashboard to make payment.</p>
        <p>— RantAI Billing</p>
        </div>"#
    );
    (subject, html)
}

pub fn subscription_created(customer_name: &str, plan_name: &str) -> (String, String) {
    let subject = format!("Subscription Created — {plan_name}");
    let html = format!(
        r#"<div style="font-family:sans-serif;max-width:600px;margin:0 auto">
        <h2>Subscription Created</h2>
        <p>Hi {customer_name},</p>
        <p>Your subscription to <strong>{plan_name}</strong> has been created.</p>
        <p>— RantAI Billing</p>
        </div>"#
    );
    (subject, html)
}

pub fn subscription_renewed(
    customer_name: &str,
    plan_name: &str,
    invoice_number: &str,
    total: &str,
    currency: &str,
    next_period_end: &str,
) -> (String, String) {
    let subject = format!("Subscription Renewed — {plan_name}");
    let html = format!(
        r#"<div style="font-family:sans-serif;max-width:600px;margin:0 auto">
        <h2>Subscription Renewed</h2>
        <p>Hi {customer_name},</p>
        <p>Your subscription to <strong>{plan_name}</strong> has been renewed.</p>
        <p><strong>Invoice:</strong> {invoice_number}</p>
        <p><strong>Amount:</strong> {currency} {total}</p>
        <p><strong>Next renewal:</strong> {next_period_end}</p>
        <p>— RantAI Billing</p>
        </div>"#
    );
    (subject, html)
}

pub fn invoice_issued(
    customer_name: &str,
    invoice_number: &str,
    total: &str,
    currency: &str,
    due_date: &str,
) -> (String, String) {
    let subject = format!("Invoice {invoice_number} Issued");
    let html = format!(
        r#"<div style="font-family:sans-serif;max-width:600px;margin:0 auto">
        <h2>Invoice Issued</h2>
        <p>Hi {customer_name},</p>
        <p>Invoice <strong>{invoice_number}</strong> for <strong>{currency} {total}</strong> has been issued.</p>
        <p><strong>Due date:</strong> {due_date}</p>
        <p>Please log in to your dashboard to view and pay this invoice.</p>
        <p>— RantAI Billing</p>
        </div>"#
    );
    (subject, html)
}
