use crate::app::{CheckItem, InstallConfig, InstallMode, Status};

use super::{command_ok, run_command};

pub fn run_checks(config: &InstallConfig, mode: InstallMode) -> Vec<CheckItem> {
    let mut checks = Vec::new();

    if mode.needs_backend() {
        // Binary exists
        checks.push(check_file_exists(
            "/usr/local/bin/rustbill-server",
            "Backend binary",
        ));

        // Config exists
        checks.push(check_file_exists(
            &format!("{}/config/production.toml", config.install_dir),
            "Backend config",
        ));

        // Health endpoint
        checks.push(check_http_health(
            &format!("http://localhost:{}/health", config.api_port),
            "Backend health",
        ));

        // Systemd service
        if mode.needs_services() {
            checks.push(check_service_active("rustbill-backend"));
        }
    }

    if mode.needs_frontend() {
        // Frontend dir
        checks.push(check_file_exists(
            &format!("{}/frontend", config.install_dir),
            "Frontend directory",
        ));

        // Systemd service
        if mode.needs_services() {
            checks.push(check_service_active("rustbill-frontend"));
        }
    }

    // Env file
    checks.push(check_file_exists(
        &format!("{}/.env", config.install_dir),
        "Environment file",
    ));

    // Database connection
    if mode.needs_database() {
        checks.push(check_database(&config.database_url()));
    }

    checks
}

fn check_file_exists(path: &str, label: &str) -> CheckItem {
    let exists = std::path::Path::new(path).exists();
    CheckItem {
        name: label.to_string(),
        status: if exists { Status::Success } else { Status::Error },
        message: if exists {
            path.to_string()
        } else {
            format!("{} not found", path)
        },
    }
}

fn check_http_health(url: &str, label: &str) -> CheckItem {
    match run_command("curl", &["-sf", "--max-time", "5", url]) {
        Ok(out) if command_ok(&out) => CheckItem {
            name: label.to_string(),
            status: Status::Success,
            message: "Responding".to_string(),
        },
        _ => CheckItem {
            name: label.to_string(),
            status: Status::Warning,
            message: "Not responding (may still be starting)".to_string(),
        },
    }
}

fn check_service_active(name: &str) -> CheckItem {
    match run_command("systemctl", &["is-active", name]) {
        Ok(out) if command_ok(&out) => CheckItem {
            name: format!("{} service", name),
            status: Status::Success,
            message: "Active".to_string(),
        },
        _ => CheckItem {
            name: format!("{} service", name),
            status: Status::Error,
            message: "Not active".to_string(),
        },
    }
}

fn check_database(url: &str) -> CheckItem {
    // Try pg_isready with the connection info
    match run_command("pg_isready", &["-d", url, "-t", "5"]) {
        Ok(out) if command_ok(&out) => CheckItem {
            name: "Database connection".to_string(),
            status: Status::Success,
            message: "Connected".to_string(),
        },
        _ => CheckItem {
            name: "Database connection".to_string(),
            status: Status::Warning,
            message: "Could not verify (pg_isready not found or connection failed)".to_string(),
        },
    }
}
