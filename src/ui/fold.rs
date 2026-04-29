use std::collections::HashSet;

pub struct FoldState {
    pub collapsed: HashSet<String>,
}

impl FoldState {
    pub fn new() -> Self {
        Self { collapsed: HashSet::new() }
    }
}

impl Default for FoldState {
    fn default() -> Self {
        Self::new()
    }
}
