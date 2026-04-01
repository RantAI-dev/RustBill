#![allow(dead_code)]
use std::sync::mpsc;

/// TUI screens in navigation order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Welcome,
    ModeSelect,
    Config,
    Preflight,
    Progress,
    Verify,
    Complete,
    Error,
}

/// Installation modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallMode {
    /// PostgreSQL + Backend + Frontend + systemd
    Full,
    /// PostgreSQL + Backend only + systemd
    BackendOnly,
    /// Frontend only + systemd (expects external backend)
    FrontendOnly,
    /// Clone + build from source, no systemd
    Development,
}

impl InstallMode {
    pub const ALL: [InstallMode; 4] = [
        InstallMode::Full,
        InstallMode::BackendOnly,
        InstallMode::FrontendOnly,
        InstallMode::Development,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            InstallMode::Full => "Full Installation",
            InstallMode::BackendOnly => "Backend Only",
            InstallMode::FrontendOnly => "Frontend Only",
            InstallMode::Development => "Development",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            InstallMode::Full => "Install PostgreSQL, Rust API server, Next.js frontend, and systemd services. Recommended for production.",
            InstallMode::BackendOnly => "Install PostgreSQL and Rust API server only. Use when the frontend is hosted separately.",
            InstallMode::FrontendOnly => "Install Next.js frontend only. Connects to an existing RustBill backend.",
            InstallMode::Development => "Clone repository and build from source. For local development — no systemd services.",
        }
    }

    pub fn needs_database(&self) -> bool {
        matches!(self, InstallMode::Full | InstallMode::BackendOnly | InstallMode::Development)
    }

    pub fn needs_backend(&self) -> bool {
        matches!(self, InstallMode::Full | InstallMode::BackendOnly | InstallMode::Development)
    }

    pub fn needs_frontend(&self) -> bool {
        matches!(self, InstallMode::Full | InstallMode::FrontendOnly | InstallMode::Development)
    }

    pub fn needs_services(&self) -> bool {
        matches!(self, InstallMode::Full | InstallMode::BackendOnly | InstallMode::FrontendOnly)
    }
}

/// Installation phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Preflight,
    Dependencies,
    Database,
    Backend,
    Frontend,
    Configuration,
    Services,
    Verification,
}

impl Phase {
    pub const ALL: [Phase; 8] = [
        Phase::Preflight,
        Phase::Dependencies,
        Phase::Database,
        Phase::Backend,
        Phase::Frontend,
        Phase::Configuration,
        Phase::Services,
        Phase::Verification,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Phase::Preflight => "Preflight Checks",
            Phase::Dependencies => "Dependencies",
            Phase::Database => "Database Setup",
            Phase::Backend => "Backend Setup",
            Phase::Frontend => "Frontend Setup",
            Phase::Configuration => "Configuration",
            Phase::Services => "Systemd Services",
            Phase::Verification => "Verification",
        }
    }

    pub fn number(&self) -> usize {
        match self {
            Phase::Preflight => 1,
            Phase::Dependencies => 2,
            Phase::Database => 3,
            Phase::Backend => 4,
            Phase::Frontend => 5,
            Phase::Configuration => 6,
            Phase::Services => 7,
            Phase::Verification => 8,
        }
    }
}

/// Status for checks and phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Pending,
    InProgress,
    Success,
    Warning,
    Error,
    Skipped,
}

/// Log level for installer messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Success,
    Warning,
    Error,
}

/// A single check item (preflight or verification).
#[derive(Debug, Clone)]
pub struct CheckItem {
    pub name: String,
    pub status: Status,
    pub message: String,
}

/// A log entry.
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: LogLevel,
    pub message: String,
}

/// Messages from background installation thread to TUI.
#[derive(Debug, Clone)]
pub enum InstallMessage {
    PhaseStart(Phase),
    PhaseComplete(Phase),
    PhaseSkipped(Phase),
    Progress(String),
    Log(LogLevel, String),
    PreflightResult(Vec<CheckItem>),
    VerifyResult(Vec<CheckItem>),
    Error(Phase, String),
    Done(bool),
}

/// A config field editable in the TUI.
#[derive(Debug, Clone)]
pub struct ConfigField {
    pub key: &'static str,
    pub label: &'static str,
    pub value: String,
    pub is_secret: bool,
}

/// Installation configuration with all user-editable parameters.
#[derive(Debug, Clone)]
pub struct InstallConfig {
    pub install_dir: String,
    pub data_dir: String,
    pub db_host: String,
    pub db_port: String,
    pub db_name: String,
    pub db_user: String,
    pub db_password: String,
    pub admin_email: String,
    pub admin_password: String,
    pub api_port: String,
    pub frontend_port: String,
    pub rust_backend_url: String,
    pub cron_secret: String,
}

impl Default for InstallConfig {
    fn default() -> Self {
        Self {
            install_dir: "/opt/rustbill".to_string(),
            data_dir: "/var/lib/rustbill".to_string(),
            db_host: "localhost".to_string(),
            db_port: "5432".to_string(),
            db_name: "rantai_billing".to_string(),
            db_user: "rantai_billing".to_string(),
            db_password: generate_random_string(24),
            admin_email: "admin@rustbill.local".to_string(),
            admin_password: "admin123".to_string(),
            api_port: "3001".to_string(),
            frontend_port: "3000".to_string(),
            rust_backend_url: "http://localhost:3001".to_string(),
            cron_secret: generate_random_string(32),
        }
    }
}

impl InstallConfig {
    /// Build the list of editable fields based on install mode.
    pub fn fields(&self, mode: InstallMode) -> Vec<ConfigField> {
        let mut fields = vec![
            ConfigField { key: "install_dir", label: "Install Directory", value: self.install_dir.clone(), is_secret: false },
            ConfigField { key: "data_dir", label: "Data Directory", value: self.data_dir.clone(), is_secret: false },
        ];

        if mode.needs_database() {
            fields.extend([
                ConfigField { key: "db_host", label: "DB Host", value: self.db_host.clone(), is_secret: false },
                ConfigField { key: "db_port", label: "DB Port", value: self.db_port.clone(), is_secret: false },
                ConfigField { key: "db_name", label: "DB Name", value: self.db_name.clone(), is_secret: false },
                ConfigField { key: "db_user", label: "DB User", value: self.db_user.clone(), is_secret: false },
                ConfigField { key: "db_password", label: "DB Password", value: self.db_password.clone(), is_secret: true },
            ]);
        }

        fields.extend([
            ConfigField { key: "admin_email", label: "Admin Email", value: self.admin_email.clone(), is_secret: false },
            ConfigField { key: "admin_password", label: "Admin Password", value: self.admin_password.clone(), is_secret: true },
        ]);

        if mode.needs_backend() {
            fields.push(ConfigField { key: "api_port", label: "API Port", value: self.api_port.clone(), is_secret: false });
            fields.push(ConfigField { key: "cron_secret", label: "CRON Secret", value: self.cron_secret.clone(), is_secret: true });
        }

        if mode.needs_frontend() {
            fields.push(ConfigField { key: "frontend_port", label: "Frontend Port", value: self.frontend_port.clone(), is_secret: false });
        }

        if matches!(mode, InstallMode::FrontendOnly) {
            fields.push(ConfigField { key: "rust_backend_url", label: "Backend URL", value: self.rust_backend_url.clone(), is_secret: false });
        }

        fields
    }

    /// Apply an edited field value back to the config.
    pub fn set_field(&mut self, key: &str, value: String) {
        match key {
            "install_dir" => self.install_dir = value,
            "data_dir" => self.data_dir = value,
            "db_host" => self.db_host = value,
            "db_port" => self.db_port = value,
            "db_name" => self.db_name = value,
            "db_user" => self.db_user = value,
            "db_password" => self.db_password = value,
            "admin_email" => self.admin_email = value,
            "admin_password" => self.admin_password = value,
            "api_port" => self.api_port = value,
            "frontend_port" => self.frontend_port = value,
            "rust_backend_url" => self.rust_backend_url = value,
            "cron_secret" => self.cron_secret = value,
            _ => {}
        }
    }

    pub fn database_url(&self) -> String {
        format!(
            "postgresql://{}:{}@{}:{}/{}",
            self.db_user, self.db_password, self.db_host, self.db_port, self.db_name
        )
    }
}

/// Main application state.
pub struct App {
    pub screen: Screen,
    pub mode: InstallMode,
    pub mode_index: usize,
    pub config: InstallConfig,
    pub config_fields: Vec<ConfigField>,
    pub config_index: usize,
    pub editing: bool,
    pub edit_buffer: String,

    pub preflight_checks: Vec<CheckItem>,
    pub verify_checks: Vec<CheckItem>,

    pub phase_statuses: Vec<(Phase, Status)>,
    pub logs: Vec<LogEntry>,
    pub log_scroll: usize,

    pub spinner_frame: usize,
    pub error_message: String,

    pub install_tx: Option<mpsc::Sender<InstallMessage>>,
    pub install_rx: Option<mpsc::Receiver<InstallMessage>>,

    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        let config = InstallConfig::default();
        let mode = InstallMode::Full;
        let config_fields = config.fields(mode);
        let phase_statuses = Phase::ALL.iter().map(|p| (*p, Status::Pending)).collect();

        Self {
            screen: Screen::Welcome,
            mode,
            mode_index: 0,
            config,
            config_fields,
            config_index: 0,
            editing: false,
            edit_buffer: String::new(),

            preflight_checks: Vec::new(),
            verify_checks: Vec::new(),

            phase_statuses,
            logs: Vec::new(),
            log_scroll: 0,

            spinner_frame: 0,
            error_message: String::new(),

            install_tx: None,
            install_rx: None,

            should_quit: false,
        }
    }

    pub fn rebuild_config_fields(&mut self) {
        self.config_fields = self.config.fields(self.mode);
        self.config_index = 0;
    }

    pub fn set_phase_status(&mut self, phase: Phase, status: Status) {
        for (p, s) in &mut self.phase_statuses {
            if *p == phase {
                *s = status;
                break;
            }
        }
    }

    pub fn add_log(&mut self, level: LogLevel, message: String) {
        let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
        self.logs.push(LogEntry {
            timestamp,
            level,
            message,
        });
    }

    pub fn tick_spinner(&mut self) {
        self.spinner_frame = (self.spinner_frame + 1) % 10;
    }
}

fn generate_random_string(len: usize) -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| {
            let idx = rng.gen_range(0..36);
            if idx < 10 {
                (b'0' + idx) as char
            } else {
                (b'a' + idx - 10) as char
            }
        })
        .collect()
}
