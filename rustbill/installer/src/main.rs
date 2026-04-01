mod app;
mod installer;
mod theme;
mod ui;

use std::io::{self, stdout};
use std::sync::mpsc;
use std::time::Duration;

use anyhow::Result;
use clap::{Parser, Subcommand};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use crossterm::{execute, cursor};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use app::{App, InstallMode, InstallMessage, LogLevel, Screen, Status};

#[derive(Parser)]
#[command(name = "rustbill-installer", about = "TUI installer for RustBill billing platform")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)]
enum Command {
    /// Install RustBill
    Install {
        /// Installation mode
        #[arg(long, default_value = "full")]
        mode: String,

        /// Run without TUI (headless)
        #[arg(long)]
        non_interactive: bool,

        /// Install directory
        #[arg(long, default_value = "/opt/rustbill")]
        install_dir: String,

        /// Database host
        #[arg(long, default_value = "localhost")]
        db_host: String,

        /// Database port
        #[arg(long, default_value = "5432")]
        db_port: String,

        /// Database name
        #[arg(long, default_value = "rantai_billing")]
        db_name: String,

        /// Database user
        #[arg(long, default_value = "rantai_billing")]
        db_user: String,

        /// Database password (auto-generated if not provided)
        #[arg(long)]
        db_password: Option<String>,

        /// API server port
        #[arg(long, default_value = "3001")]
        api_port: String,

        /// Frontend port
        #[arg(long, default_value = "3000")]
        frontend_port: String,
    },

    /// Uninstall RustBill
    Uninstall {
        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Install {
            mode,
            non_interactive,
            install_dir,
            db_host,
            db_port,
            db_name,
            db_user,
            db_password,
            api_port,
            frontend_port,
        }) => {
            let install_mode = match mode.as_str() {
                "full" => InstallMode::Full,
                "backend" | "backend-only" => InstallMode::BackendOnly,
                "frontend" | "frontend-only" => InstallMode::FrontendOnly,
                "dev" | "development" => InstallMode::Development,
                _ => {
                    eprintln!("Unknown mode: {}. Use: full, backend, frontend, dev", mode);
                    std::process::exit(1);
                }
            };

            if non_interactive {
                run_non_interactive(install_mode, install_dir, db_host, db_port, db_name, db_user, db_password, api_port, frontend_port)
            } else {
                let mut app = App::new();
                app.mode = install_mode;
                app.config.install_dir = install_dir;
                app.config.db_host = db_host;
                app.config.db_port = db_port;
                app.config.db_name = db_name;
                app.config.db_user = db_user;
                if let Some(pw) = db_password {
                    app.config.db_password = pw;
                }
                app.config.api_port = api_port;
                app.config.frontend_port = frontend_port;
                app.rebuild_config_fields();
                run_tui(app)
            }
        }
        Some(Command::Uninstall { force }) => {
            run_uninstall(force)
        }
        None => {
            // Default: launch TUI
            run_tui(App::new())
        }
    }
}

fn run_tui(mut app: App) -> Result<()> {
    // Terminal setup
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen, cursor::Hide)?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let result = run_event_loop(&mut terminal, &mut app);

    // Cleanup
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen, cursor::Show)?;

    result
}

fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|frame| ui::render(frame, app))?;

        // Process install messages if on Progress screen
        if app.screen == Screen::Progress {
            let messages: Vec<_> = app
                .install_rx
                .as_ref()
                .map(|rx| std::iter::from_fn(|| rx.try_recv().ok()).collect())
                .unwrap_or_default();
            for msg in messages {
                handle_install_message(app, msg);
            }
        }

        // Poll for events (100ms timeout for spinner animation)
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // Global quit
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    break;
                }

                match app.screen {
                    Screen::Welcome => handle_welcome_input(app, key.code),
                    Screen::ModeSelect => handle_mode_select_input(app, key.code),
                    Screen::Config => handle_config_input(app, key.code),
                    Screen::Preflight => handle_preflight_input(app, key.code),
                    Screen::Progress => handle_progress_input(app, key.code),
                    Screen::Verify => handle_verify_input(app, key.code),
                    Screen::Complete => handle_complete_input(app, key.code),
                    Screen::Error => handle_error_input(app, key.code),
                }
            }
        } else {
            // Tick spinner on timeout
            app.tick_spinner();
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

fn handle_welcome_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Enter => app.screen = Screen::ModeSelect,
        KeyCode::Char('q') => app.should_quit = true,
        _ => {}
    }
}

fn handle_mode_select_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Up | KeyCode::Char('k') => {
            if app.mode_index > 0 {
                app.mode_index -= 1;
                app.mode = InstallMode::ALL[app.mode_index];
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.mode_index < InstallMode::ALL.len() - 1 {
                app.mode_index += 1;
                app.mode = InstallMode::ALL[app.mode_index];
            }
        }
        KeyCode::Enter => {
            app.rebuild_config_fields();
            app.screen = Screen::Config;
        }
        KeyCode::Esc => app.screen = Screen::Welcome,
        KeyCode::Char('q') => app.should_quit = true,
        _ => {}
    }
}

fn handle_config_input(app: &mut App, key: KeyCode) {
    if app.editing {
        match key {
            KeyCode::Enter => {
                let field_key = app.config_fields[app.config_index].key;
                let value = app.edit_buffer.clone();
                app.config.set_field(field_key, value.clone());
                app.config_fields[app.config_index].value = value;
                app.editing = false;
                app.edit_buffer.clear();
            }
            KeyCode::Esc => {
                app.editing = false;
                app.edit_buffer.clear();
            }
            KeyCode::Backspace => {
                app.edit_buffer.pop();
            }
            KeyCode::Char(c) => {
                app.edit_buffer.push(c);
            }
            _ => {}
        }
        return;
    }

    match key {
        KeyCode::Up | KeyCode::Char('k') => {
            if app.config_index > 0 {
                app.config_index -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.config_index < app.config_fields.len().saturating_sub(1) {
                app.config_index += 1;
            }
        }
        KeyCode::Char('e') | KeyCode::Tab => {
            app.editing = true;
            app.edit_buffer = app.config_fields[app.config_index].value.clone();
        }
        KeyCode::Enter => {
            // Start preflight
            app.screen = Screen::Preflight;
            run_preflight(app);
        }
        KeyCode::Esc => app.screen = Screen::ModeSelect,
        KeyCode::Char('q') => app.should_quit = true,
        _ => {}
    }
}

fn handle_preflight_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Enter => {
            // Check if any preflight check failed
            let has_error = app
                .preflight_checks
                .iter()
                .any(|c| c.status == Status::Error);
            if has_error {
                app.error_message = "Preflight checks failed. Fix issues and retry.".to_string();
                app.screen = Screen::Error;
            } else {
                app.screen = Screen::Progress;
                start_installation(app);
            }
        }
        KeyCode::Char('r') => run_preflight(app),
        KeyCode::Esc => app.screen = Screen::Config,
        KeyCode::Char('q') => app.should_quit = true,
        _ => {}
    }
}

fn handle_progress_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Up | KeyCode::Char('k') => {
            app.log_scroll = app.log_scroll.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.log_scroll = app.log_scroll.saturating_add(1);
        }
        _ => {}
    }
}

fn handle_verify_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Enter => app.screen = Screen::Complete,
        KeyCode::Char('q') => app.should_quit = true,
        _ => {}
    }
}

fn handle_complete_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Enter | KeyCode::Char('q') => app.should_quit = true,
        _ => {}
    }
}

fn handle_error_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => app.screen = Screen::Config,
        KeyCode::Char('q') => app.should_quit = true,
        _ => {}
    }
}

fn run_preflight(app: &mut App) {
    app.preflight_checks = installer::preflight::run_checks(&app.config, app.mode);
}

fn start_installation(app: &mut App) {
    let (tx, rx) = mpsc::channel();
    app.install_rx = Some(rx);

    let config = app.config.clone();
    let mode = app.mode;

    std::thread::spawn(move || {
        installer::executor::run_installation(config, mode, tx);
    });
}

fn handle_install_message(app: &mut App, msg: InstallMessage) {
    match msg {
        InstallMessage::PhaseStart(phase) => {
            app.set_phase_status(phase, Status::InProgress);
            app.add_log(LogLevel::Info, format!("Starting: {}", phase.name()));
        }
        InstallMessage::PhaseComplete(phase) => {
            app.set_phase_status(phase, Status::Success);
            app.add_log(LogLevel::Success, format!("Completed: {}", phase.name()));
        }
        InstallMessage::PhaseSkipped(phase) => {
            app.set_phase_status(phase, Status::Skipped);
            app.add_log(LogLevel::Info, format!("Skipped: {}", phase.name()));
        }
        InstallMessage::Progress(msg) => {
            app.add_log(LogLevel::Info, msg);
            // Auto-scroll
            if app.logs.len() > 5 {
                app.log_scroll = app.logs.len().saturating_sub(5);
            }
        }
        InstallMessage::Log(level, msg) => {
            app.add_log(level, msg);
            if app.logs.len() > 5 {
                app.log_scroll = app.logs.len().saturating_sub(5);
            }
        }
        InstallMessage::PreflightResult(checks) => {
            app.preflight_checks = checks;
        }
        InstallMessage::VerifyResult(checks) => {
            app.verify_checks = checks;
            app.screen = Screen::Verify;
        }
        InstallMessage::Error(phase, msg) => {
            app.set_phase_status(phase, Status::Error);
            app.add_log(LogLevel::Error, msg.clone());
            app.error_message = msg;
            app.screen = Screen::Error;
        }
        InstallMessage::Done(success) => {
            if success {
                app.screen = Screen::Verify;
                // Run verification
                app.verify_checks = installer::verify::run_checks(&app.config, app.mode);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run_non_interactive(
    mode: InstallMode,
    install_dir: String,
    db_host: String,
    db_port: String,
    db_name: String,
    db_user: String,
    db_password: Option<String>,
    api_port: String,
    frontend_port: String,
) -> Result<()> {
    println!("╔════════════════════════════════════════════════╗");
    println!("║  RustBill Installer (non-interactive)          ║");
    println!("╚════════════════════════════════════════════════╝");
    println!();

    let config = app::InstallConfig {
        install_dir,
        db_host,
        db_port,
        db_name,
        db_user,
        db_password: db_password.unwrap_or_else(|| app::InstallConfig::default().db_password),
        api_port,
        frontend_port,
        ..Default::default()
    };

    // Run preflight
    println!("[1/8] Running preflight checks...");
    let checks = installer::preflight::run_checks(&config, mode);
    let mut has_error = false;
    for check in &checks {
        let sym = theme::status_symbol(check.status);
        println!("  {} {} {}", sym, check.name, check.message);
        if check.status == Status::Error {
            has_error = true;
        }
    }
    if has_error {
        eprintln!("\nPreflight checks failed. Aborting.");
        std::process::exit(1);
    }
    println!();

    // Run installation via channel
    let (tx, rx) = mpsc::channel();
    let config_clone = config.clone();
    let handle = std::thread::spawn(move || {
        installer::executor::run_installation(config_clone, mode, tx);
    });

    // Print messages as they arrive
    let mut success = true;
    loop {
        match rx.recv_timeout(Duration::from_secs(300)) {
            Ok(InstallMessage::PhaseStart(phase)) => {
                println!("[{}/8] {}...", phase.number(), phase.name());
            }
            Ok(InstallMessage::PhaseComplete(phase)) => {
                println!("  {} {}", theme::SYM_SUCCESS, phase.name());
            }
            Ok(InstallMessage::PhaseSkipped(phase)) => {
                println!("  {} {} (skipped)", theme::SYM_SKIPPED, phase.name());
            }
            Ok(InstallMessage::Progress(msg)) | Ok(InstallMessage::Log(_, msg)) => {
                println!("  {}", msg);
            }
            Ok(InstallMessage::Error(phase, msg)) => {
                eprintln!("  {} {} FAILED: {}", theme::SYM_ERROR, phase.name(), msg);
                success = false;
            }
            Ok(InstallMessage::Done(_)) => break,
            Ok(_) => {}
            Err(mpsc::RecvTimeoutError::Timeout) => {
                eprintln!("Installation timed out.");
                success = false;
                break;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    let _ = handle.join();

    if success {
        // Run verification
        println!("\nRunning verification...");
        let checks = installer::verify::run_checks(&config, mode);
        for check in &checks {
            let sym = theme::status_symbol(check.status);
            println!("  {} {} {}", sym, check.name, check.message);
        }
        println!("\n{} Installation complete!", theme::SYM_SUCCESS);
        if mode.needs_backend() {
            println!("  API: http://localhost:{}", config.api_port);
        }
        if mode.needs_frontend() {
            println!("  Dashboard: http://localhost:{}", config.frontend_port);
        }
        println!("  Admin: {}", config.admin_email);
    } else {
        eprintln!("\nInstallation failed.");
        std::process::exit(1);
    }

    Ok(())
}

fn run_uninstall(force: bool) -> Result<()> {
    if !force {
        println!("This will remove RustBill and all associated data.");
        println!("Are you sure? (y/N)");
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return Ok(());
        }
    }

    println!("Stopping services...");
    let _ = std::process::Command::new("sudo")
        .args(["systemctl", "stop", "rustbill-frontend", "rustbill-backend"])
        .status();
    let _ = std::process::Command::new("sudo")
        .args(["systemctl", "disable", "rustbill-frontend", "rustbill-backend"])
        .status();

    println!("Removing service files...");
    let _ = std::process::Command::new("sudo")
        .args(["rm", "-f", "/etc/systemd/system/rustbill-backend.service", "/etc/systemd/system/rustbill-frontend.service"])
        .status();
    let _ = std::process::Command::new("sudo")
        .args(["systemctl", "daemon-reload"])
        .status();

    println!("Removing binary...");
    let _ = std::process::Command::new("sudo")
        .args(["rm", "-f", "/usr/local/bin/rustbill-server"])
        .status();

    println!("Removing installation directory...");
    let _ = std::process::Command::new("sudo")
        .args(["rm", "-rf", "/opt/rustbill"])
        .status();

    println!("Removing system user...");
    let _ = std::process::Command::new("sudo")
        .args(["userdel", "-r", "rustbill"])
        .status();

    println!("{} RustBill uninstalled.", theme::SYM_SUCCESS);
    println!("Note: PostgreSQL database was preserved. Drop it manually if needed:");
    println!("  sudo -u postgres dropdb rantai_billing");
    println!("  sudo -u postgres dropuser rantai_billing");

    Ok(())
}
