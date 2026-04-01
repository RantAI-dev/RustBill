use std::sync::mpsc::Sender;

use crate::app::{InstallConfig, InstallMessage};

use super::{command_ok, run_sudo, stdout_string};

pub fn setup(config: &InstallConfig, tx: &Sender<InstallMessage>) -> Result<(), String> {
    // Wait for PostgreSQL to be ready
    tx.send(InstallMessage::Progress("Waiting for PostgreSQL...".to_string())).ok();
    wait_for_postgres()?;

    // Check if user exists
    tx.send(InstallMessage::Progress(format!("Creating database user '{}'...", config.db_user))).ok();
    let check = run_sudo("sudo", &[
        "-u", "postgres", "psql", "-tAc",
        &format!("SELECT 1 FROM pg_roles WHERE rolname='{}'", config.db_user),
    ])?;

    if stdout_string(&check) != "1" {
        let out = run_sudo("sudo", &[
            "-u", "postgres", "psql", "-c",
            &format!("CREATE USER {} WITH PASSWORD '{}' CREATEDB;", config.db_user, config.db_password),
        ])?;
        if !command_ok(&out) {
            return Err(format!("Failed to create database user: {}", super::stderr_string(&out)));
        }
    } else {
        tx.send(InstallMessage::Progress("Database user already exists".to_string())).ok();
    }

    // Check if database exists
    tx.send(InstallMessage::Progress(format!("Creating database '{}'...", config.db_name))).ok();
    let check = run_sudo("sudo", &[
        "-u", "postgres", "psql", "-tAc",
        &format!("SELECT 1 FROM pg_database WHERE datname='{}'", config.db_name),
    ])?;

    if stdout_string(&check) != "1" {
        let out = run_sudo("sudo", &[
            "-u", "postgres", "psql", "-c",
            &format!("CREATE DATABASE {} OWNER {};", config.db_name, config.db_user),
        ])?;
        if !command_ok(&out) {
            return Err(format!("Failed to create database: {}", super::stderr_string(&out)));
        }
    } else {
        tx.send(InstallMessage::Progress("Database already exists".to_string())).ok();
    }

    // Grant privileges
    let _ = run_sudo("sudo", &[
        "-u", "postgres", "psql", "-c",
        &format!("GRANT ALL PRIVILEGES ON DATABASE {} TO {};", config.db_name, config.db_user),
    ]);

    tx.send(InstallMessage::Progress("Database setup complete".to_string())).ok();
    Ok(())
}

fn wait_for_postgres() -> Result<(), String> {
    for i in 0..30 {
        let out = run_sudo("sudo", &["-u", "postgres", "pg_isready"]);
        if let Ok(o) = out {
            if command_ok(&o) {
                return Ok(());
            }
        }
        if i < 29 {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
    Err("PostgreSQL did not become ready within 30 seconds".to_string())
}
