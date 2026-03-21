use crate::settings::provider_settings::ProviderSettings;

pub trait ProviderSettingsRepository {
    fn get_setting(&self, key: &str) -> Option<String>;
}

impl ProviderSettingsRepository for ProviderSettings {
    fn get_setting(&self, key: &str) -> Option<String> {
        self.get(key)
    }
}
