mod app;
mod async_loader;
mod config;
mod event;
mod git;
mod github;
mod ui;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use app::{App, AppCommand};
use event::{AppEvent, EventHandler};

/// Kimchi - AI-native code review TUI
#[derive(Parser, Debug)]
#[command(name = "kimchi")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to git repository
    #[arg(default_value = ".")]
    path: PathBuf,
}

fn main() -> Result<()> {
    // Initialize logging (controlled by RUST_LOG env var)
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let args = Args::parse();

    // Resolve path
    let path = args.path.canonicalize().unwrap_or(args.path);

    // Initialize terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new(path.to_str().unwrap_or("."))?;

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
            AppEvent::Resize(_, _) => {
                // Terminal will redraw automatically
            }
            AppEvent::Tick => {
                app.handle_tick();
            }
            AppEvent::FileChanged => {
                app.refresh()?;
            }
            AppEvent::PrLoaded => {
                // PR data updated
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
