use crate::diff::types::Diff;
use crate::review::types::Draft;
use crate::session::{Overlay, Session};
use crate::ui::fold::FoldState;
use crate::ui::scroll::ScrollState;

pub struct State {
    pub session: Session,
    pub diff: Diff,
    pub overlay: Option<Overlay>,
    pub draft: Draft,
    pub scroll: ScrollState,
    pub fold: FoldState,
}
