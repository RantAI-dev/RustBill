#![allow(dead_code)]
pub mod preflight;
pub mod executor;
pub mod deps;
pub mod database;
pub mod backend;
pub mod frontend;
pub mod config;
pub mod services;
pub mod verify;

use std::process::{Command, Output};

/// Run a shell command and return its output.
pub fn run_command(cmd: &str, args: &[&str]) -> Result<Output, String> {
    Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| format!("Failed to execute {}: {}", cmd, e))
}

/// Run a command with sudo.
pub fn run_sudo(cmd: &str, args: &[&str]) -> Result<Output, String> {
    let mut sudo_args = vec![cmd];
    sudo_args.extend(args);
    Command::new("sudo")
        .args(&sudo_args)
        .output()
        .map_err(|e| format!("Failed to execute sudo {}: {}", cmd, e))
}

/// Check if a command succeeded.
pub fn command_ok(output: &Output) -> bool {
    output.status.success()
}

/// Get stdout as string.
pub fn stdout_string(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// Get stderr as string.
pub fn stderr_string(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).trim().to_string()
}

/// Generate a random alphanumeric string.
pub fn generate_password(len: usize) -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| {
            let idx = rng.gen_range(0..62);
            if idx < 10 {
                (b'0' + idx) as char
            } else if idx < 36 {
                (b'a' + idx - 10) as char
            } else {
                (b'A' + idx - 36) as char
            }
        })
        .collect()
}
