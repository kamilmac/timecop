mod app;
mod async_loader;
mod config;
mod event;
mod git;
mod github;
mod theme;
mod ui;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::fs::OpenOptions;
use std::io;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use app::{App, AppCommand};
use event::{AppEvent, EventHandler};

/// TimeCop - AI-native code review TUI
#[derive(Parser, Debug)]
#[command(name = "timecop")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to git repository
    #[arg(default_value = ".")]
    path: PathBuf,
}

fn main() -> Result<()> {
    // Initialize logging to file (avoids corrupting TUI output on stderr)
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/timecop.log")
        .expect("Failed to open log file");
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"))
        .target(env_logger::Target::Pipe(Box::new(log_file)))
        .init();

    let args = Args::parse();

    // Resolve path
    let path = args.path.canonicalize().unwrap_or(args.path);

    // Create app first (fails early if not a git repo)
    let mut app = App::new(path.to_str().unwrap_or(".")).map_err(|_| {
        anyhow::anyhow!(
            "Not a git repository: {}\n\nTimeCop must be run inside a git repository.",
            path.display()
        )
    })?;

    // Initialize terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create event handler with git file watcher
    let events = EventHandler::with_git_watcher(Duration::from_millis(100), &path);

    // Main loop
    let result = run_app(&mut terminal, &mut app, &events);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

fn run_app<B: Backend + io::Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    events: &EventHandler,
) -> Result<()> {
    while app.running {
        // Draw
        terminal.draw(|frame| {
            app.render(frame);
        })?;

        // Handle events
        match events.next()? {
            AppEvent::Key(key) => {
                app.handle_key(key)?;
            }
            AppEvent::Mouse(mouse) => {
                app.handle_mouse(mouse)?;
            }
            AppEvent::Tick => {
                app.handle_tick();
            }
            AppEvent::FileChanged => {
                // Drain queued FileChanged events to coalesce rapid saves
                let mut pending = Vec::new();
                while let Some(evt) = events.try_next() {
                    if !matches!(evt, AppEvent::FileChanged) {
                        pending.push(evt);
                    }
                }
                app.refresh()?;
                for evt in pending {
                    match evt {
                        AppEvent::Key(key) => app.handle_key(key)?,
                        AppEvent::Mouse(mouse) => app.handle_mouse(mouse)?,
                        AppEvent::Tick => app.handle_tick(),
                        AppEvent::FileChanged => {}
                    }
                }
            }
        }

        // Handle pending commands
        match app.take_command() {
            AppCommand::None => {}
            AppCommand::OpenEditor { path, line } => {
                // Pause event polling first
                events.pause();

                // Suspend terminal
                disable_raw_mode()?;
                execute!(
                    terminal.backend_mut(),
                    LeaveAlternateScreen,
                    DisableMouseCapture
                )?;
                terminal.show_cursor()?;

                // Run editor
                let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
                let mut cmd = Command::new(&editor);

                // Add line number argument if available (works for vim, nvim, hx, etc.)
                if let Some(line_num) = line {
                    if editor.contains("vim") || editor.contains("nvim") {
                        cmd.arg(format!("+{}", line_num));
                    } else if editor.contains("hx") || editor.contains("helix") {
                        // Helix uses file:line format
                        cmd.arg(format!("{}:{}", path, line_num));
                    } else {
                        cmd.arg(&path);
                    }
                    if !editor.contains("hx") && !editor.contains("helix") {
                        cmd.arg(&path);
                    }
                } else {
                    cmd.arg(&path);
                }

                // Run with proper stdio inheritance
                let _ = cmd
                    .stdin(std::process::Stdio::inherit())
                    .stdout(std::process::Stdio::inherit())
                    .stderr(std::process::Stdio::inherit())
                    .status();

                // Resume terminal
                enable_raw_mode()?;
                execute!(
                    terminal.backend_mut(),
                    EnterAlternateScreen,
                    EnableMouseCapture
                )?;
                terminal.hide_cursor()?;
                terminal.clear()?;

                // Resume event polling
                events.resume();

                // Refresh after returning from editor
                app.refresh()?;
            }
        }
    }

    Ok(())
}
