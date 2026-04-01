use crate::app::{CheckItem, InstallConfig, InstallMode, Status};
use std::net::TcpListener;
use sysinfo::System;

pub fn run_checks(config: &InstallConfig, mode: InstallMode) -> Vec<CheckItem> {
    let mut checks = vec![
        check_os(),
        check_arch(),
        check_ram(),
        check_disk(),
        check_root(),
    ];

    if mode.needs_backend() {
        checks.push(check_port(&config.api_port, "API port"));
    }
    if mode.needs_frontend() {
        checks.push(check_port(&config.frontend_port, "Frontend port"));
    }
    if mode.needs_database() {
        checks.push(check_port(&config.db_port, "PostgreSQL port"));
    }

    checks
}

fn check_os() -> CheckItem {
    if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
        let pretty = content
            .lines()
            .find(|l| l.starts_with("PRETTY_NAME="))
            .map(|l| l.trim_start_matches("PRETTY_NAME=").trim_matches('"'))
            .unwrap_or("Unknown");

        let id = content
            .lines()
            .find(|l| l.starts_with("ID="))
            .map(|l| l.trim_start_matches("ID=").trim_matches('"'))
            .unwrap_or("");

        let supported = matches!(id, "ubuntu" | "debian" | "rhel" | "rocky" | "fedora" | "almalinux" | "centos");

        CheckItem {
            name: "Operating System".to_string(),
            status: if supported { Status::Success } else { Status::Warning },
            message: pretty.to_string(),
        }
    } else {
        CheckItem {
            name: "Operating System".to_string(),
            status: Status::Error,
            message: "Cannot detect OS".to_string(),
        }
    }
}

fn check_arch() -> CheckItem {
    let arch = std::env::consts::ARCH;
    let supported = matches!(arch, "x86_64" | "aarch64");
    CheckItem {
        name: "Architecture".to_string(),
        status: if supported { Status::Success } else { Status::Error },
        message: arch.to_string(),
    }
}

fn check_ram() -> CheckItem {
    let mut sys = System::new();
    sys.refresh_memory();
    let total_mb = sys.total_memory() / 1024 / 1024;
    let status = if total_mb >= 1024 {
        Status::Success
    } else if total_mb >= 512 {
        Status::Warning
    } else {
        Status::Error
    };
    CheckItem {
        name: "Memory".to_string(),
        status,
        message: format!("{} MB (minimum 1024 MB)", total_mb),
    }
}

fn check_disk() -> CheckItem {
    let available = fs_available_mb("/");
    let status = if available >= 2048 {
        Status::Success
    } else if available >= 1024 {
        Status::Warning
    } else {
        Status::Error
    };
    CheckItem {
        name: "Disk Space".to_string(),
        status,
        message: format!("{} MB available (recommended 2048 MB)", available),
    }
}

fn check_root() -> CheckItem {
    let is_root = nix::unistd::geteuid().is_root();
    let can_sudo = if !is_root {
        std::process::Command::new("sudo")
            .args(["-n", "true"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    } else {
        true
    };
    CheckItem {
        name: "Privileges".to_string(),
        status: if is_root || can_sudo { Status::Success } else { Status::Warning },
        message: if is_root {
            "Running as root".to_string()
        } else if can_sudo {
            "Passwordless sudo available".to_string()
        } else {
            "May need sudo password".to_string()
        },
    }
}

fn check_port(port: &str, label: &str) -> CheckItem {
    let port_num: u16 = port.parse().unwrap_or(0);
    if port_num == 0 {
        return CheckItem {
            name: format!("{} ({})", label, port),
            status: Status::Error,
            message: "Invalid port number".to_string(),
        };
    }

    let available = TcpListener::bind(format!("0.0.0.0:{}", port_num)).is_ok();
    CheckItem {
        name: format!("{} ({})", label, port),
        status: if available { Status::Success } else { Status::Warning },
        message: if available {
            "Available".to_string()
        } else {
            "Port in use (may be an existing service)".to_string()
        },
    }
}

fn fs_available_mb(path: &str) -> u64 {
    if let Ok(stat) = nix::sys::statvfs::statvfs(path) {
        (stat.blocks_available() * stat.fragment_size()) / 1024 / 1024
    } else {
        0
    }
}
