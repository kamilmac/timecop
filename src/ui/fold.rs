use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct FoldState {
    pub collapsed: HashSet<String>,
    pub description_collapsed: bool,
}

impl FoldState {
    pub fn new(all_paths: &[String]) -> Self {
        Self {
            collapsed: all_paths.iter().cloned().collect(),
            description_collapsed: true,
        }
    }

    pub fn is_collapsed(&self, path: &str) -> bool {
        self.collapsed.contains(path)
    }

    pub fn toggle(&mut self, path: &str) {
        if self.collapsed.contains(path) {
            self.collapsed.remove(path);
        } else {
            self.collapsed.insert(path.to_string());
        }
    }

    pub fn collapse_all(&mut self, all_paths: &[String]) {
        self.collapsed = all_paths.iter().cloned().collect();
    }

    pub fn expand_all(&mut self) {
        self.collapsed.clear();
    }

    pub fn any_expanded(&self, all_paths: &[String]) -> bool {
        all_paths.iter().any(|p| !self.collapsed.contains(p))
    }
}

impl Default for FoldState {
    fn default() -> Self {
        Self {
            collapsed: HashSet::new(),
            description_collapsed: true,
        }
    }
}
