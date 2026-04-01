use std::sync::mpsc::Sender;

use crate::app::{InstallConfig, InstallMessage, InstallMode};
use crate::theme;

use super::{command_ok, run_command, run_sudo};

pub fn setup(config: &InstallConfig, mode: InstallMode, tx: &Sender<InstallMessage>) -> Result<(), String> {
    match mode {
        InstallMode::Development => build_from_source(config, tx),
        _ => download_binary(config, tx),
    }
}

fn download_binary(config: &InstallConfig, tx: &Sender<InstallMessage>) -> Result<(), String> {
    let _ = config;
    let url = format!(
        "https://github.com/{}/releases/latest/download/rustbill-server-x86_64-linux-musl",
        theme::REPO
    );

    tx.send(InstallMessage::Progress("Downloading RustBill server binary...".to_string())).ok();

    let out = run_command("curl", &[
        "-fsSL", &url,
        "-o", "/tmp/rustbill-server",
    ])?;
    if !command_ok(&out) {
        return Err(format!("Failed to download backend binary from {}", url));
    }

    // Install binary
    tx.send(InstallMessage::Progress("Installing binary to /usr/local/bin/...".to_string())).ok();
    run_sudo("mv", &["/tmp/rustbill-server", "/usr/local/bin/rustbill-server"])?;
    run_sudo("chmod", &["755", "/usr/local/bin/rustbill-server"])?;

    // Copy default config
    tx.send(InstallMessage::Progress("Setting up config directory...".to_string())).ok();
    run_sudo("mkdir", &["-p", &format!("{}/config", config.install_dir)])?;

    // Download default.toml
    let config_url = format!(
        "https://raw.githubusercontent.com/{}/main/rustbill/config/default.toml",
        theme::REPO
    );
    let out = run_command("curl", &[
        "-fsSL", &config_url,
        "-o", "/tmp/rustbill-default.toml",
    ])?;
    if command_ok(&out) {
        run_sudo("mv", &["/tmp/rustbill-default.toml", &format!("{}/config/default.toml", config.install_dir)])?;
    }

    Ok(())
}

fn build_from_source(config: &InstallConfig, tx: &Sender<InstallMessage>) -> Result<(), String> {
    let repo_url = format!("https://github.com/{}.git", theme::REPO);

    tx.send(InstallMessage::Progress(format!("Cloning repository to {}...", config.install_dir))).ok();
    run_sudo("mkdir", &["-p", &config.install_dir])?;

    let out = run_command("git", &["clone", &repo_url, &config.install_dir])?;
    if !command_ok(&out) {
        // May already exist
        tx.send(InstallMessage::Progress("Repository may already exist, pulling latest...".to_string())).ok();
        let _ = run_command("git", &["-C", &config.install_dir, "pull"]);
    }

    tx.send(InstallMessage::Progress("Building Rust backend (this may take a while)...".to_string())).ok();
    let out = run_command("cargo", &[
        "build", "--release",
        "--manifest-path", &format!("{}/rustbill/Cargo.toml", config.install_dir),
    ])?;
    if !command_ok(&out) {
        return Err(format!("Failed to build backend: {}", super::stderr_string(&out)));
    }

    tx.send(InstallMessage::Progress("Backend build complete".to_string())).ok();
    Ok(())
}
