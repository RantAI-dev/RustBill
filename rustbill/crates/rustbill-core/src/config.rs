use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    pub cron: CronConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub cors_origins: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    #[serde(default)]
    pub url: String,
    #[serde(default = "default_max_conn")]
    pub max_connections: u32,
    #[serde(default = "default_min_conn")]
    pub min_connections: u32,
}

fn default_max_conn() -> u32 { 20 }
fn default_min_conn() -> u32 { 5 }

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default = "default_session_expiry")]
    pub session_expiry_days: u32,
    pub keycloak: Option<KeycloakConfig>,
    pub cron_secret: Option<String>,
}

fn default_provider() -> String { "default".to_string() }
fn default_session_expiry() -> u32 { 7 }

#[derive(Debug, Clone, Deserialize)]
pub struct KeycloakConfig {
    pub realm_url: String,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub admin_role: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CronConfig {
    #[serde(default = "default_lifecycle_schedule")]
    pub subscription_lifecycle: String,
    #[serde(default = "default_dunning_schedule")]
    pub dunning: String,
    #[serde(default)]
    pub enabled: bool,
}

fn default_lifecycle_schedule() -> String { "0 0 * * * *".to_string() }
fn default_dunning_schedule() -> String { "0 0 */6 * * *".to_string() }

impl AppConfig {
    /// Load config from TOML files + environment variables.
    /// Priority: env vars > production/development.toml > default.toml
    pub fn load() -> anyhow::Result<Self> {
        let run_mode = std::env::var("RUN_MODE").unwrap_or_else(|_| "development".to_string());

        let mut builder = config::Config::builder()
            .add_source(config::File::with_name("config/default").required(false))
            .add_source(config::File::with_name(&format!("config/{run_mode}")).required(false))
            .add_source(
                config::Environment::with_prefix("BILLING")
                    .separator("__")
                    .try_parsing(true),
            );

        // Map DATABASE_URL env var to database.url
        if let Ok(url) = std::env::var("DATABASE_URL") {
            builder = builder.set_override("database.url", url)?;
        }

        // Map CRON_SECRET env var to auth.cron_secret
        if let Ok(secret) = std::env::var("CRON_SECRET") {
            builder = builder.set_override("auth.cron_secret", secret)?;
        }

        let config = builder.build()?;
        Ok(config.try_deserialize()?)
    }
}
