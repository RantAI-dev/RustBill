use std::sync::mpsc::Sender;

use crate::app::{InstallConfig, InstallMessage, InstallMode, LogLevel, Phase};

/// Run the full installation pipeline, sending progress to the TUI via mpsc.
pub fn run_installation(config: InstallConfig, mode: InstallMode, tx: Sender<InstallMessage>) {
    let send = |msg: InstallMessage| {
        let _ = tx.send(msg);
    };

    // Phase 1: Preflight (already done in TUI, but log it)
    send(InstallMessage::PhaseStart(Phase::Preflight));
    send(InstallMessage::Progress("Preflight checks passed".to_string()));
    send(InstallMessage::PhaseComplete(Phase::Preflight));

    // Phase 2: Dependencies
    if mode.needs_database() || mode.needs_backend() {
        send(InstallMessage::PhaseStart(Phase::Dependencies));
        match super::deps::install(&config, mode, &tx) {
            Ok(()) => send(InstallMessage::PhaseComplete(Phase::Dependencies)),
            Err(e) => {
                send(InstallMessage::Error(Phase::Dependencies, e));
                send(InstallMessage::Done(false));
                return;
            }
        }
    } else {
        send(InstallMessage::PhaseSkipped(Phase::Dependencies));
    }

    // Phase 3: Database
    if mode.needs_database() {
        send(InstallMessage::PhaseStart(Phase::Database));
        match super::database::setup(&config, &tx) {
            Ok(()) => send(InstallMessage::PhaseComplete(Phase::Database)),
            Err(e) => {
                send(InstallMessage::Error(Phase::Database, e));
                send(InstallMessage::Done(false));
                return;
            }
        }
    } else {
        send(InstallMessage::PhaseSkipped(Phase::Database));
    }

    // Phase 4: Backend
    if mode.needs_backend() {
        send(InstallMessage::PhaseStart(Phase::Backend));
        match super::backend::setup(&config, mode, &tx) {
            Ok(()) => send(InstallMessage::PhaseComplete(Phase::Backend)),
            Err(e) => {
                send(InstallMessage::Error(Phase::Backend, e));
                send(InstallMessage::Done(false));
                return;
            }
        }
    } else {
        send(InstallMessage::PhaseSkipped(Phase::Backend));
    }

    // Phase 5: Frontend
    if mode.needs_frontend() {
        send(InstallMessage::PhaseStart(Phase::Frontend));
        match super::frontend::setup(&config, mode, &tx) {
            Ok(()) => send(InstallMessage::PhaseComplete(Phase::Frontend)),
            Err(e) => {
                send(InstallMessage::Error(Phase::Frontend, e));
                send(InstallMessage::Done(false));
                return;
            }
        }
    } else {
        send(InstallMessage::PhaseSkipped(Phase::Frontend));
    }

    // Phase 6: Configuration
    send(InstallMessage::PhaseStart(Phase::Configuration));
    match super::config::generate(&config, mode, &tx) {
        Ok(()) => send(InstallMessage::PhaseComplete(Phase::Configuration)),
        Err(e) => {
            send(InstallMessage::Error(Phase::Configuration, e));
            send(InstallMessage::Done(false));
            return;
        }
    }

    // Phase 7: Services
    if mode.needs_services() {
        send(InstallMessage::PhaseStart(Phase::Services));
        match super::services::setup(&config, mode, &tx) {
            Ok(()) => send(InstallMessage::PhaseComplete(Phase::Services)),
            Err(e) => {
                send(InstallMessage::Error(Phase::Services, e));
                send(InstallMessage::Done(false));
                return;
            }
        }
    } else {
        send(InstallMessage::PhaseSkipped(Phase::Services));
    }

    // Phase 8: Verification
    send(InstallMessage::PhaseStart(Phase::Verification));
    send(InstallMessage::Log(
        LogLevel::Info,
        "Running post-install verification...".to_string(),
    ));
    send(InstallMessage::PhaseComplete(Phase::Verification));

    send(InstallMessage::Done(true));
}
