use std::sync::mpsc::Sender;

use crate::app::{InstallConfig, InstallMessage, InstallMode};
use crate::theme;

use super::{command_ok, run_command, run_sudo};

pub fn setup(config: &InstallConfig, mode: InstallMode, tx: &Sender<InstallMessage>) -> Result<(), String> {
    match mode {
        InstallMode::Development => build_from_source(config, tx),
        _ => download_tarball(config, tx),
    }
}

fn download_tarball(config: &InstallConfig, tx: &Sender<InstallMessage>) -> Result<(), String> {
    let url = format!(
        "https://github.com/{}/releases/latest/download/rustbill-frontend-standalone.tar.gz",
        theme::REPO
    );

    tx.send(InstallMessage::Progress("Downloading frontend tarball...".to_string())).ok();
    let out = run_command("curl", &[
        "-fsSL", &url,
        "-o", "/tmp/rustbill-frontend.tar.gz",
    ])?;
    if !command_ok(&out) {
        return Err(format!("Failed to download frontend tarball from {}", url));
    }

    // Extract
    let frontend_dir = format!("{}/frontend", config.install_dir);
    tx.send(InstallMessage::Progress("Extracting frontend...".to_string())).ok();
    run_sudo("mkdir", &["-p", &frontend_dir])?;
    let out = run_sudo("tar", &["xzf", "/tmp/rustbill-frontend.tar.gz", "-C", &frontend_dir])?;
    if !command_ok(&out) {
        return Err("Failed to extract frontend tarball".to_string());
    }

    // Download Bun binary for the frontend service
    tx.send(InstallMessage::Progress("Downloading Bun runtime...".to_string())).ok();
    let bin_dir = format!("{}/bin", config.install_dir);
    run_sudo("mkdir", &["-p", &bin_dir])?;

    let bun_url = "https://github.com/oven-sh/bun/releases/latest/download/bun-linux-x64.zip";
    let out = run_command("curl", &["-fsSL", bun_url, "-o", "/tmp/bun.zip"])?;
    if command_ok(&out) {
        let _ = run_command("bash", &["-c", "cd /tmp && unzip -o bun.zip"]);
        let _ = run_sudo("mv", &["/tmp/bun-linux-x64/bun", &format!("{}/bun", bin_dir)]);
        let _ = run_sudo("chmod", &["755", &format!("{}/bun", bin_dir)]);
    } else {
        tx.send(InstallMessage::Progress("Warning: Could not download Bun. Frontend may need manual Bun installation.".to_string())).ok();
    }

    // Cleanup
    let _ = run_command("rm", &["-f", "/tmp/rustbill-frontend.tar.gz", "/tmp/bun.zip"]);
    let _ = run_command("rm", &["-rf", "/tmp/bun-linux-x64"]);

    Ok(())
}

fn build_from_source(config: &InstallConfig, tx: &Sender<InstallMessage>) -> Result<(), String> {
    // Assumes repo is already cloned in backend phase
    tx.send(InstallMessage::Progress("Installing frontend dependencies...".to_string())).ok();
    let out = run_command("bash", &["-c", &format!("cd {} && bun install --frozen-lockfile", config.install_dir)])?;
    if !command_ok(&out) {
        // Try without frozen lockfile
        let out = run_command("bash", &["-c", &format!("cd {} && bun install", config.install_dir)])?;
        if !command_ok(&out) {
            return Err("Failed to install frontend dependencies".to_string());
        }
    }

    tx.send(InstallMessage::Progress("Building frontend (this may take a while)...".to_string())).ok();
    let out = run_command("bash", &["-c", &format!("cd {} && bun run build", config.install_dir)])?;
    if !command_ok(&out) {
        return Err(format!("Failed to build frontend: {}", super::stderr_string(&out)));
    }

    tx.send(InstallMessage::Progress("Frontend build complete".to_string())).ok();
    Ok(())
}
