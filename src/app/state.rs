use crate::session::Session;
use crate::ui::fold::FoldState;
use crate::ui::input::InputState;
use crate::ui::scroll::ScrollState;
use crate::ui::syntax::Highlighter;
use std::collections::HashSet;
use std::path::PathBuf;

pub struct State {
    pub session: Session,
    pub repo_path: PathBuf,
    pub scroll: ScrollState,
    pub fold: FoldState,
    pub thread_overrides: HashSet<usize>,
    pub input: Option<InputState>,
    pub show_help: bool,
    pub show_verdict: bool,
    pub pending_g: bool,
    pub pending_editor: Option<(String, u32)>,
    pub quit: bool,
    pub status_message: Option<String>,
    pub highlighter: Highlighter,
}

impl State {
    pub fn new(session: Session, repo_path: PathBuf) -> Self {
        let paths: Vec<String> = session.diff().files.iter().map(|f| f.path.clone()).collect();
        Self {
            session,
            repo_path,
            scroll: ScrollState::new(),
            fold: FoldState::new(&paths),
            thread_overrides: HashSet::new(),
            input: None,
            show_help: false,
            show_verdict: false,
            pending_g: false,
            pending_editor: None,
            quit: false,
            status_message: None,
            highlighter: Highlighter::new(),
        }
    }

    pub fn all_paths(&self) -> Vec<String> {
        self.session.diff().files.iter().map(|f| f.path.clone()).collect()
    }
}
