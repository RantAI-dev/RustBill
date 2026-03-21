use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DunningConfig {
    /// Days overdue to send a reminder (e.g. 1).
    pub reminder_days: i64,
    /// Days overdue to send a warning (e.g. 7).
    pub warning_days: i64,
    /// Days overdue to send a final notice (e.g. 14).
    pub final_notice_days: i64,
    /// Days overdue to suspend (e.g. 30).
    pub suspension_days: i64,
}

impl Default for DunningConfig {
    fn default() -> Self {
        Self {
            reminder_days: 3,
            warning_days: 7,
            final_notice_days: 14,
            suspension_days: 30,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DunningLogFilter {
    pub invoice_id: Option<String>,
}
