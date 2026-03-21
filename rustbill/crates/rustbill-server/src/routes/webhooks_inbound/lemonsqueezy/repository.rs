pub trait LemonSqueezyWebhookRepository: Send + Sync {}

#[derive(Clone, Default)]
pub struct SqlxLemonSqueezyWebhookRepository;

impl LemonSqueezyWebhookRepository for SqlxLemonSqueezyWebhookRepository {}
