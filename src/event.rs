use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
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
    /// Terminal resize
    Resize(u16, u16),
    /// File system change detected
    FileChanged,
    /// Tick for periodic updates
    Tick,
    /// PR data loaded
    PrLoaded,
}

/// Event handler that runs in a separate thread
pub struct EventHandler {
    rx: mpsc::Receiver<AppEvent>,
    _tx: mpsc::Sender<AppEvent>,
    paused: Arc<AtomicBool>,
    _watcher: Option<notify_debouncer_mini::Debouncer<RecommendedWatcher>>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        Self::with_watcher(tick_rate, None)
    }

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
                        let app_event = match event {
                            Event::Key(key) => Some(AppEvent::Key(key)),
                            Event::Resize(w, h) => Some(AppEvent::Resize(w, h)),
                            _ => None,
                        };

                        if let Some(e) = app_event {
                            if event_tx.send(e).is_err() {
                                break;
                            }
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
        git_dir: &Path,
        tx: mpsc::Sender<AppEvent>,
    ) -> Option<notify_debouncer_mini::Debouncer<RecommendedWatcher>> {
        let git_index = git_dir.join(".git").join("index");
        if !git_index.exists() {
            return None;
        }

        let debouncer = new_debouncer(Duration::from_millis(500), move |res: DebounceEventResult| {
            if let Ok(events) = res {
                for event in events {
                    if matches!(event.kind, DebouncedEventKind::Any) {
                        let _ = tx.send(AppEvent::FileChanged);
                        break;
                    }
                }
            }
        });

        match debouncer {
            Ok(mut watcher) => {
                if watcher
                    .watcher()
                    .watch(&git_index, RecursiveMode::NonRecursive)
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

    fn with_watcher(tick_rate: Duration, watcher: Option<notify_debouncer_mini::Debouncer<RecommendedWatcher>>) -> Self {
        let (tx, rx) = mpsc::channel();
        let event_tx = tx.clone();
        let paused = Arc::new(AtomicBool::new(false));
        let paused_clone = paused.clone();

        // Spawn event polling thread
        thread::spawn(move || {
            loop {
                // Check if paused
                if paused_clone.load(Ordering::Relaxed) {
                    thread::sleep(Duration::from_millis(50));
                    continue;
                }

                // Poll for events with timeout
                if event::poll(tick_rate).unwrap_or(false) {
                    // Double-check we're not paused before reading
                    if paused_clone.load(Ordering::Relaxed) {
                        continue;
                    }

                    if let Ok(event) = event::read() {
                        let app_event = match event {
                            Event::Key(key) => Some(AppEvent::Key(key)),
                            Event::Resize(w, h) => Some(AppEvent::Resize(w, h)),
                            _ => None,
                        };

                        if let Some(e) = app_event {
                            if event_tx.send(e).is_err() {
                                break;
                            }
                        }
                    }
                } else {
                    // Send tick on timeout (only if not paused)
                    if !paused_clone.load(Ordering::Relaxed) {
                        if event_tx.send(AppEvent::Tick).is_err() {
                            break;
                        }
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

    /// Get the next event (blocking)
    pub fn next(&self) -> Result<AppEvent> {
        Ok(self.rx.recv()?)
    }

    /// Try to get the next event (non-blocking)
    pub fn try_next(&self) -> Option<AppEvent> {
        self.rx.try_recv().ok()
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

    pub fn is_mode_cycle(key: &KeyEvent) -> bool {
        key.code == KeyCode::Char('m') && key.modifiers == KeyModifiers::NONE
    }

    pub fn get_mode_number(key: &KeyEvent) -> Option<u8> {
        if key.modifiers != KeyModifiers::NONE {
            return None;
        }
        match key.code {
            KeyCode::Char('1') => Some(1),
            KeyCode::Char('2') => Some(2),
            KeyCode::Char('3') => Some(3),
            KeyCode::Char('4') => Some(4),
            _ => None,
        }
    }

    pub fn is_yank(key: &KeyEvent) -> bool {
        key.code == KeyCode::Char('y') && key.modifiers == KeyModifiers::NONE
    }

    pub fn is_open(key: &KeyEvent) -> bool {
        key.code == KeyCode::Char('o') && key.modifiers == KeyModifiers::NONE
    }

    pub fn is_checkout(key: &KeyEvent) -> bool {
        key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::NONE
    }

    pub fn is_refresh(key: &KeyEvent) -> bool {
        key.code == KeyCode::Char('r') && key.modifiers == KeyModifiers::NONE
    }

    pub fn is_pr_list(key: &KeyEvent) -> bool {
        key.code == KeyCode::Char('p') && key.modifiers == KeyModifiers::NONE
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
}
