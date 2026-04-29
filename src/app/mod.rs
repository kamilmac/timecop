pub mod action;
pub mod format;
pub mod state;

use crate::session::{Session, branch, pr};
use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use state::State;
use std::env;
use std::io::{self, Stdout};
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

type Term = Terminal<CrosstermBackend<Stdout>>;

pub fn run(target: Option<String>) -> Result<()> {
    let repo_path = env::current_dir()?;
    let session = open_session(target, &repo_path)?;
    let mut state = State::new(session, repo_path);

    let mut terminal = setup_terminal()?;
    let result = main_loop(&mut terminal, &mut state);
    teardown_terminal(&mut terminal)?;
    result
}

fn open_session(target: Option<String>, repo_path: &PathBuf) -> Result<Session> {
    match target {
        None => Ok(Session::Branch(branch::open(repo_path.clone(), None)?)),
        Some(s) => {
            if let Ok(num) = s.parse::<u32>() {
                return Ok(Session::Pr(pr::open(num)?));
            }
            Ok(Session::Branch(branch::open(repo_path.clone(), Some(s))?))
        }
    }
}

fn setup_terminal() -> Result<Term> {
    enable_raw_mode().context("enable_raw_mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend).context("create terminal")
}

fn teardown_terminal(terminal: &mut Term) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn main_loop(terminal: &mut Term, state: &mut State) -> Result<()> {
    while !state.quit {
        terminal.draw(|f| crate::ui::render::render(state, f))?;
        if event::poll(Duration::from_millis(200))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    action::handle_key(state, key)?;
                }
                _ => {}
            }
        }
        if let Some((path, line)) = state.pending_editor.take() {
            run_editor(terminal, state, &path, line)?;
        }
    }
    Ok(())
}

fn run_editor(terminal: &mut Term, state: &mut State, path: &str, line: u32) -> Result<()> {
    teardown_terminal(terminal)?;
    let editor = env::var("EDITOR").unwrap_or_else(|_| "hx".to_string());
    let target_path = state.repo_path.join(path);
    let target = target_path.to_string_lossy().to_string();
    let result = launch_editor(&editor, &target, line);
    *terminal = setup_terminal()?;
    terminal.clear()?;
    if let Err(e) = result {
        state.status_message = Some(format!("editor failed: {e}"));
    }
    Ok(())
}

fn launch_editor(editor: &str, path: &str, line: u32) -> Result<()> {
    let editor_name = std::path::Path::new(editor)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(editor);
    let status = match editor_name {
        "vim" | "nvim" | "vi" => Command::new(editor)
            .arg(format!("+{line}"))
            .arg(path)
            .status()?,
        "code" | "cursor" => Command::new(editor)
            .arg("-g")
            .arg(format!("{path}:{line}"))
            .status()?,
        _ => Command::new(editor)
            .arg(format!("{path}:{line}"))
            .status()?,
    };
    if !status.success() {
        anyhow::bail!("editor exited with status {status}");
    }
    Ok(())
}
