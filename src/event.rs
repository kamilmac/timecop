use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use ignore::gitignore::GitignoreBuilder;
use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, DebouncedEventKind};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

/// Application events
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// Terminal key press
    Key(KeyEvent),
    /// Mouse event
    Mouse(MouseEvent),
    /// File system change detected
    FileChanged,
    /// Tick for periodic updates
    Tick,
}

/// Event handler that runs in a separate thread
pub struct EventHandler {
    rx: mpsc::Receiver<AppEvent>,
    _tx: mpsc::Sender<AppEvent>,
    paused: Arc<AtomicBool>,
    _watcher: Option<notify_debouncer_mini::Debouncer<RecommendedWatcher>>,
}

impl EventHandler {
    pub fn with_git_watcher(tick_rate: Duration, git_dir: &Path) -> Self {
        let (tx, rx) = mpsc::channel();
        let event_tx = tx.clone();
        let paused = Arc::new(AtomicBool::new(false));
        let paused_clone = paused.clone();

        // Set up file watcher for .git/index
        let watcher_tx = tx.clone();
        let watcher = Self::setup_watcher(git_dir, watcher_tx);

        // Spawn event polling thread
        thread::spawn(move || {
            loop {
                if paused_clone.load(Ordering::Relaxed) {
                    thread::sleep(Duration::from_millis(50));
                    continue;
                }

                if event::poll(tick_rate).unwrap_or(false) {
                    if paused_clone.load(Ordering::Relaxed) {
                        continue;
                    }

                    if let Ok(event) = event::read() {
                        match event {
                            Event::Key(key) => {
                                if event_tx.send(AppEvent::Key(key)).is_err() {
                                    break;
                                }
                            }
                            Event::Mouse(mouse) => {
                                if event_tx.send(AppEvent::Mouse(mouse)).is_err() {
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                } else if !paused_clone.load(Ordering::Relaxed) {
                    if event_tx.send(AppEvent::Tick).is_err() {
                        break;
                    }
                }
            }
        });

        Self {
            rx,
            _tx: tx,
            paused,
            _watcher: watcher,
        }
    }

    fn setup_watcher(
        repo_dir: &Path,
        tx: mpsc::Sender<AppEvent>,
    ) -> Option<notify_debouncer_mini::Debouncer<RecommendedWatcher>> {
        let repo_path = repo_dir.to_path_buf();

        // Build gitignore matcher
        let mut builder = GitignoreBuilder::new(&repo_path);
        let gitignore_path = repo_path.join(".gitignore");
        if gitignore_path.exists() {
            let _ = builder.add(&gitignore_path);
        }
        let gitignore = builder.build().ok();

        let debouncer = new_debouncer(Duration::from_millis(300), move |res: DebounceEventResult| {
            if let Ok(events) = res {
                for event in events {
                    if matches!(event.kind, DebouncedEventKind::Any) {
                        // Filter out .git internal changes (except index)
                        let rel_path = event.path.strip_prefix(&repo_path).ok();

                        let dominated_by_git = rel_path
                            .map(|p| {
                                let p_str = p.to_string_lossy();
                                p_str.starts_with(".git/") && !p_str.starts_with(".git/index")
                            })
                            .unwrap_or(false);

                        // Check gitignore
                        let is_ignored = gitignore.as_ref()
                            .and_then(|gi| rel_path.map(|p| gi.matched(p, event.path.is_dir()).is_ignore()))
                            .unwrap_or(false);

                        if !dominated_by_git && !is_ignored {
                            let _ = tx.send(AppEvent::FileChanged);
                            break;
                        }
                    }
                }
            }
        });

        match debouncer {
            Ok(mut watcher) => {
                // Watch the entire repo directory
                if watcher
                    .watcher()
                    .watch(repo_dir, RecursiveMode::Recursive)
                    .is_ok()
                {
                    Some(watcher)
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }

    /// Get the next event (blocking)
    pub fn next(&self) -> Result<AppEvent> {
        Ok(self.rx.recv()?)
    }

    /// Pause event polling (for spawning external processes)
    pub fn pause(&self) {
        self.paused.store(true, Ordering::Relaxed);
        // Give the polling thread time to stop
        thread::sleep(Duration::from_millis(150));
    }

    /// Resume event polling
    pub fn resume(&self) {
        self.paused.store(false, Ordering::Relaxed);
    }
}

/// Key input helper
pub struct KeyInput;

impl KeyInput {
    pub fn is_quit(key: &KeyEvent) -> bool {
        matches!(
            key,
            KeyEvent {
                code: KeyCode::Char('q'),
                modifiers: KeyModifiers::NONE,
                ..
            } | KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
                ..
            }
        )
    }

    pub fn is_down(key: &KeyEvent) -> bool {
        matches!(
            key.code,
            KeyCode::Char('j') | KeyCode::Down
        ) && key.modifiers == KeyModifiers::NONE
    }

    pub fn is_up(key: &KeyEvent) -> bool {
        matches!(
            key.code,
            KeyCode::Char('k') | KeyCode::Up
        ) && key.modifiers == KeyModifiers::NONE
    }

    pub fn is_fast_down(key: &KeyEvent) -> bool {
        key.code == KeyCode::Char('J') && key.modifiers == KeyModifiers::SHIFT
    }

    pub fn is_fast_up(key: &KeyEvent) -> bool {
        key.code == KeyCode::Char('K') && key.modifiers == KeyModifiers::SHIFT
    }

    pub fn is_left(key: &KeyEvent) -> bool {
        key.code == KeyCode::Char('h') && key.modifiers == KeyModifiers::NONE
    }

    pub fn is_right(key: &KeyEvent) -> bool {
        key.code == KeyCode::Char('l') && key.modifiers == KeyModifiers::NONE
    }

    pub fn is_tab(key: &KeyEvent) -> bool {
        key.code == KeyCode::Tab && key.modifiers == KeyModifiers::NONE
    }

    pub fn is_shift_tab(key: &KeyEvent) -> bool {
        key.code == KeyCode::BackTab
            || (key.code == KeyCode::Tab && key.modifiers == KeyModifiers::SHIFT)
    }

    pub fn is_page_down(key: &KeyEvent) -> bool {
        key.code == KeyCode::Char('d') && key.modifiers == KeyModifiers::CONTROL
    }

    pub fn is_page_up(key: &KeyEvent) -> bool {
        key.code == KeyCode::Char('u') && key.modifiers == KeyModifiers::CONTROL
    }

    pub fn is_top(key: &KeyEvent) -> bool {
        key.code == KeyCode::Char('g') && key.modifiers == KeyModifiers::NONE
    }

    pub fn is_bottom(key: &KeyEvent) -> bool {
        key.code == KeyCode::Char('G') && key.modifiers == KeyModifiers::SHIFT
    }

    pub fn is_enter(key: &KeyEvent) -> bool {
        key.code == KeyCode::Enter
    }

    pub fn is_escape(key: &KeyEvent) -> bool {
        key.code == KeyCode::Esc
    }

    pub fn is_help(key: &KeyEvent) -> bool {
        key.code == KeyCode::Char('?')
    }

    pub fn is_yank(key: &KeyEvent) -> bool {
        key.code == KeyCode::Char('y') && key.modifiers == KeyModifiers::NONE
    }

    pub fn is_open(key: &KeyEvent) -> bool {
        key.code == KeyCode::Char('o') && key.modifiers == KeyModifiers::NONE
    }

    pub fn is_refresh(key: &KeyEvent) -> bool {
        key.code == KeyCode::Char('r') && key.modifiers == KeyModifiers::NONE
    }

    pub fn is_approve(key: &KeyEvent) -> bool {
        key.code == KeyCode::Char('a') && key.modifiers == KeyModifiers::NONE
    }

    pub fn is_request_changes(key: &KeyEvent) -> bool {
        key.code == KeyCode::Char('x') && key.modifiers == KeyModifiers::NONE
    }

    pub fn is_comment(key: &KeyEvent) -> bool {
        key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::NONE
    }

    pub fn is_timeline_next(key: &KeyEvent) -> bool {
        key.code == KeyCode::Char(',') && key.modifiers == KeyModifiers::NONE
    }

    pub fn is_timeline_prev(key: &KeyEvent) -> bool {
        key.code == KeyCode::Char('.') && key.modifiers == KeyModifiers::NONE
    }

    pub fn is_toggle_view_mode(key: &KeyEvent) -> bool {
        key.code == KeyCode::Char('s') && key.modifiers == KeyModifiers::NONE
    }
}
