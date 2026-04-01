use std::sync::mpsc::Sender;

use crate::app::{InstallConfig, InstallMessage, InstallMode};

use super::{command_ok, run_command, run_sudo};

pub fn install(config: &InstallConfig, mode: InstallMode, tx: &Sender<InstallMessage>) -> Result<(), String> {
    let _ = config;
    let pm = detect_package_manager()?;

    // Update package index
    tx.send(InstallMessage::Progress("Updating package index...".to_string())).ok();
    match pm {
        PackageManager::Apt => {
            let out = run_sudo("apt-get", &["update", "-qq"])?;
            if !command_ok(&out) {
                return Err("Failed to update apt package index".to_string());
            }
        }
        PackageManager::Dnf => {
            let out = run_sudo("dnf", &["makecache", "-q"])?;
            if !command_ok(&out) {
                return Err("Failed to update dnf cache".to_string());
            }
        }
    }

    // Install base packages
    tx.send(InstallMessage::Progress("Installing base packages...".to_string())).ok();
    let base_pkgs = match pm {
        PackageManager::Apt => vec!["curl", "git", "ca-certificates", "gnupg"],
        PackageManager::Dnf => vec!["curl", "git", "ca-certificates"],
    };
    install_packages(&pm, &base_pkgs)?;

    // Install PostgreSQL if needed
    if mode.needs_database() {
        tx.send(InstallMessage::Progress("Installing PostgreSQL...".to_string())).ok();
        install_postgresql(&pm)?;
    }

    // Install Bun for development mode
    if matches!(mode, InstallMode::Development) {
        tx.send(InstallMessage::Progress("Installing Bun...".to_string())).ok();
        install_bun()?;
    }

    Ok(())
}

enum PackageManager {
    Apt,
    Dnf,
}

fn detect_package_manager() -> Result<PackageManager, String> {
    if run_command("which", &["apt-get"]).map(|o| command_ok(&o)).unwrap_or(false) {
        Ok(PackageManager::Apt)
    } else if run_command("which", &["dnf"]).map(|o| command_ok(&o)).unwrap_or(false) {
        Ok(PackageManager::Dnf)
    } else {
        Err("No supported package manager found (apt or dnf required)".to_string())
    }
}

fn install_packages(pm: &PackageManager, packages: &[&str]) -> Result<(), String> {
    let out = match pm {
        PackageManager::Apt => {
            let mut args = vec!["apt-get", "install", "-y", "-qq"];
            args.extend(packages.iter().copied());
            run_sudo("bash", &["-c", &format!("DEBIAN_FRONTEND=noninteractive apt-get install -y -qq {}", packages.join(" "))])?
        }
        PackageManager::Dnf => {
            let mut args = vec!["dnf", "install", "-y", "-q"];
            args.extend(packages.iter().copied());
            run_sudo(args[0], &args[1..])?
        }
    };
    if command_ok(&out) {
        Ok(())
    } else {
        Err(format!("Failed to install packages: {}", packages.join(", ")))
    }
}

fn install_postgresql(pm: &PackageManager) -> Result<(), String> {
    match pm {
        PackageManager::Apt => {
            // Add PostgreSQL APT repo for PG17
            let _ = run_sudo("bash", &["-c",
                "curl -fsSL https://www.postgresql.org/media/keys/ACCC4CF8.asc | gpg --dearmor -o /usr/share/keyrings/postgresql-keyring.gpg"
            ]);
            let _ = run_sudo("bash", &["-c",
                "echo 'deb [signed-by=/usr/share/keyrings/postgresql-keyring.gpg] https://apt.postgresql.org/pub/repos/apt/ '$(lsb_release -cs)'-pgdg main' > /etc/apt/sources.list.d/pgdg.list"
            ]);
            let _ = run_sudo("apt-get", &["update", "-qq"]);

            let out = run_sudo("bash", &["-c",
                "DEBIAN_FRONTEND=noninteractive apt-get install -y -qq postgresql-17 || DEBIAN_FRONTEND=noninteractive apt-get install -y -qq postgresql"
            ])?;
            if !command_ok(&out) {
                return Err("Failed to install PostgreSQL".to_string());
            }
        }
        PackageManager::Dnf => {
            let out = run_sudo("dnf", &["install", "-y", "-q", "postgresql-server", "postgresql"])?;
            if !command_ok(&out) {
                return Err("Failed to install PostgreSQL".to_string());
            }
            // Initialize DB if needed
            let _ = run_sudo("postgresql-setup", &["--initdb"]);
        }
    }

    // Enable and start
    let _ = run_sudo("systemctl", &["enable", "postgresql"]);
    let out = run_sudo("systemctl", &["start", "postgresql"])?;
    if !command_ok(&out) {
        return Err("Failed to start PostgreSQL service".to_string());
    }

    Ok(())
}

fn install_bun() -> Result<(), String> {
    // Check if already installed
    if run_command("which", &["bun"]).map(|o| command_ok(&o)).unwrap_or(false) {
        return Ok(());
    }

    let out = run_command("bash", &["-c", "curl -fsSL https://bun.sh/install | bash"])?;
    if !command_ok(&out) {
        return Err("Failed to install Bun".to_string());
    }
    Ok(())
}
