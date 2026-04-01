use std::sync::mpsc::Sender;

use crate::app::{InstallConfig, InstallMessage, InstallMode};

use super::{command_ok, run_sudo};

pub fn setup(config: &InstallConfig, mode: InstallMode, tx: &Sender<InstallMessage>) -> Result<(), String> {
    if mode.needs_backend() {
        tx.send(InstallMessage::Progress("Creating rustbill-backend.service...".to_string())).ok();
        create_backend_service(config)?;
    }

    if mode.needs_frontend() {
        tx.send(InstallMessage::Progress("Creating rustbill-frontend.service...".to_string())).ok();
        create_frontend_service(config, mode)?;
    }

    // Reload systemd
    tx.send(InstallMessage::Progress("Reloading systemd...".to_string())).ok();
    run_sudo("systemctl", &["daemon-reload"])?;

    // Enable and start services
    if mode.needs_backend() {
        tx.send(InstallMessage::Progress("Enabling and starting backend service...".to_string())).ok();
        run_sudo("systemctl", &["enable", "rustbill-backend"])?;
        let out = run_sudo("systemctl", &["start", "rustbill-backend"])?;
        if !command_ok(&out) {
            return Err("Failed to start rustbill-backend service".to_string());
        }

        // Wait for backend to be healthy before starting frontend
        tx.send(InstallMessage::Progress("Waiting for backend health check...".to_string())).ok();
        wait_for_health(&format!("http://localhost:{}/health", config.api_port), 30)?;
    }

    if mode.needs_frontend() {
        tx.send(InstallMessage::Progress("Enabling and starting frontend service...".to_string())).ok();
        run_sudo("systemctl", &["enable", "rustbill-frontend"])?;
        let out = run_sudo("systemctl", &["start", "rustbill-frontend"])?;
        if !command_ok(&out) {
            return Err("Failed to start rustbill-frontend service".to_string());
        }
    }

    Ok(())
}

fn create_backend_service(config: &InstallConfig) -> Result<(), String> {
    let content = format!(
        r#"[Unit]
Description=RustBill API Server
Documentation=https://github.com/{repo}
After=network-online.target postgresql.service
Wants=network-online.target
Requires=postgresql.service

[Service]
Type=simple
User=rustbill
Group=rustbill
WorkingDirectory={install_dir}
EnvironmentFile={install_dir}/.env
ExecStart=/usr/local/bin/rustbill-server
Restart=on-failure
RestartSec=10
TimeoutStartSec=30
TimeoutStopSec=15

StandardOutput=journal
StandardError=journal
SyslogIdentifier=rustbill-backend

NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
PrivateTmp=true
ReadWritePaths={install_dir} {data_dir}

[Install]
WantedBy=multi-user.target
"#,
        repo = crate::theme::REPO,
        install_dir = config.install_dir,
        data_dir = config.data_dir,
    );

    write_service_file("rustbill-backend.service", &content)
}

fn create_frontend_service(config: &InstallConfig, mode: InstallMode) -> Result<(), String> {
    let (working_dir, exec_start) = if matches!(mode, InstallMode::Development) {
        (
            config.install_dir.clone(),
            "bun start".to_string(),
        )
    } else {
        (
            format!("{}/frontend", config.install_dir),
            format!("{}/bin/bun server.js", config.install_dir),
        )
    };

    let after = if mode.needs_backend() {
        "After=network-online.target rustbill-backend.service\nWants=rustbill-backend.service"
    } else {
        "After=network-online.target"
    };

    let content = format!(
        r#"[Unit]
Description=RustBill Frontend (Next.js)
Documentation=https://github.com/{repo}
{after}

[Service]
Type=simple
User=rustbill
Group=rustbill
WorkingDirectory={working_dir}
EnvironmentFile={install_dir}/.env
ExecStart={exec_start}
Restart=on-failure
RestartSec=10
TimeoutStartSec=60

StandardOutput=journal
StandardError=journal
SyslogIdentifier=rustbill-frontend

NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
PrivateTmp=true
ReadWritePaths={install_dir}

[Install]
WantedBy=multi-user.target
"#,
        repo = crate::theme::REPO,
        after = after,
        working_dir = working_dir,
        install_dir = config.install_dir,
        exec_start = exec_start,
    );

    write_service_file("rustbill-frontend.service", &content)
}

fn write_service_file(name: &str, content: &str) -> Result<(), String> {
    let tmp_path = format!("/tmp/{}", name);
    std::fs::write(&tmp_path, content)
        .map_err(|e| format!("Failed to write temp service file: {}", e))?;
    let out = run_sudo("mv", &[&tmp_path, &format!("/etc/systemd/system/{}", name)])?;
    if !command_ok(&out) {
        return Err(format!("Failed to install service file {}", name));
    }
    Ok(())
}

fn wait_for_health(url: &str, timeout_secs: u32) -> Result<(), String> {
    for _ in 0..timeout_secs {
        if let Ok(out) = super::run_command("curl", &["-sf", url]) {
            if command_ok(&out) {
                return Ok(());
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    Err(format!("Health check at {} did not pass within {} seconds", url, timeout_secs))
}
