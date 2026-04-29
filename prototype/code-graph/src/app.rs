use crate::graph::{CodeGraph, ModuleInfo};
use crate::layout::{ego_layout, LayoutNode};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ZoomLevel {
    Heatmap,  // coupling matrix
    Entities, // entities within a module (or between two modules)
    Ego,      // single entity focus
}

pub struct App {
    pub graph: CodeGraph,
    pub zoom: ZoomLevel,

    // Heatmap state
    pub modules: Vec<ModuleInfo>,
    pub matrix: Vec<Vec<usize>>,  // matrix[row][col] = relation count
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub max_cell_value: usize,

    // Entity level state
    pub entity_list_title: String,
    pub module_entities: Vec<usize>,
    pub selected_entity_idx: usize,

    // Ego level state
    pub layout_nodes: Vec<LayoutNode>,
    pub center_entity: usize,
    pub visible_neighbors: Vec<usize>,
    pub selected_neighbor: usize,
    pub ego_history: Vec<usize>,

    // Search
    pub search_mode: bool,
    pub search_query: String,
    pub search_results: Vec<usize>,
    pub search_selected: usize,
}

impl App {
    pub fn new(graph: CodeGraph) -> Self {
        let modules = graph.compute_modules();
        let module_relations = graph.module_relations(&modules);

        // Build the coupling matrix
        let n = modules.len();
        let mut matrix = vec![vec![0usize; n]; n];

        // Diagonal = internal entity count
        for (i, module) in modules.iter().enumerate() {
            matrix[i][i] = module.entity_count;
        }

        // Off-diagonal = cross-module relation count
        let mut max_off_diag = 0usize;
        for &(a, b, count) in &module_relations {
            matrix[a][b] = count;
            matrix[b][a] = count;
            max_off_diag = max_off_diag.max(count);
        }

        App {
            graph,
            zoom: ZoomLevel::Heatmap,
            modules,
            matrix,
            cursor_row: 0,
            cursor_col: 0,
            max_cell_value: max_off_diag,
            entity_list_title: String::new(),
            module_entities: Vec::new(),
            selected_entity_idx: 0,
            layout_nodes: Vec::new(),
            center_entity: 0,
            visible_neighbors: Vec::new(),
            selected_neighbor: 0,
            ego_history: Vec::new(),
            search_mode: false,
            search_query: String::new(),
            search_results: Vec::new(),
            search_selected: 0,
        }
    }

    // --- Navigation ---

    pub fn move_vertical(&mut self, delta: i32) {
        if self.search_mode {
            if !self.search_results.is_empty() {
                let len = self.search_results.len() as i32;
                self.search_selected =
                    ((self.search_selected as i32 + delta).rem_euclid(len)) as usize;
            }
            return;
        }
        match self.zoom {
            ZoomLevel::Heatmap => {
                let len = self.modules.len() as i32;
                if len > 0 {
                    self.cursor_row =
                        ((self.cursor_row as i32 + delta).rem_euclid(len)) as usize;
                }
            }
            ZoomLevel::Entities => {
                if !self.module_entities.is_empty() {
                    let len = self.module_entities.len() as i32;
                    self.selected_entity_idx =
                        ((self.selected_entity_idx as i32 + delta).rem_euclid(len)) as usize;
                }
            }
            ZoomLevel::Ego => {
                if !self.visible_neighbors.is_empty() {
                    let len = self.visible_neighbors.len() as i32;
                    self.selected_neighbor =
                        ((self.selected_neighbor as i32 + delta).rem_euclid(len)) as usize;
                }
            }
        }
    }

    pub fn move_horizontal(&mut self, delta: i32) {
        if self.zoom == ZoomLevel::Heatmap {
            let len = self.modules.len() as i32;
            if len > 0 {
                self.cursor_col =
                    ((self.cursor_col as i32 + delta).rem_euclid(len)) as usize;
            }
        }
    }

    pub fn zoom_in(&mut self) {
        if self.search_mode {
            if let Some(&eid) = self.search_results.get(self.search_selected) {
                self.search_mode = false;
                self.search_query.clear();
                self.enter_ego(eid);
            }
            return;
        }
        match self.zoom {
            ZoomLevel::Heatmap => {
                if self.cursor_row == self.cursor_col {
                    // Diagonal: drill into module
                    self.enter_module(self.cursor_row);
                } else {
                    // Off-diagonal: show cross-module entities
                    self.enter_cross_module(self.cursor_row, self.cursor_col);
                }
            }
            ZoomLevel::Entities => {
                if let Some(&eid) = self.module_entities.get(self.selected_entity_idx) {
                    self.enter_ego(eid);
                }
            }
            ZoomLevel::Ego => {
                if let Some(&target) = self.visible_neighbors.get(self.selected_neighbor) {
                    self.ego_history.push(self.center_entity);
                    self.center_entity = target;
                    self.recompute_ego();
                }
            }
        }
    }

    pub fn zoom_out(&mut self) {
        if self.search_mode {
            self.search_mode = false;
            self.search_query.clear();
            return;
        }
        match self.zoom {
            ZoomLevel::Heatmap => {}
            ZoomLevel::Entities => {
                self.zoom = ZoomLevel::Heatmap;
            }
            ZoomLevel::Ego => {
                if let Some(prev) = self.ego_history.pop() {
                    self.center_entity = prev;
                    self.recompute_ego();
                } else {
                    self.zoom = ZoomLevel::Entities;
                }
            }
        }
    }

    fn enter_module(&mut self, module_idx: usize) {
        let module = &self.modules[module_idx];
        self.entity_list_title = format!("{} — {} entities", module.name, module.entity_count);
        self.module_entities = module.entity_ids.clone();
        self.module_entities.sort_by_key(|&id| {
            let e = &self.graph.entities[id];
            (e.file.clone(), e.line)
        });
        self.selected_entity_idx = 0;
        self.zoom = ZoomLevel::Entities;
    }

    fn enter_cross_module(&mut self, row: usize, col: usize) {
        let row_mod = &self.modules[row];
        let col_mod = &self.modules[col];
        let count = self.matrix[row][col];
        self.entity_list_title = format!(
            "{} ↔ {} — {} relations",
            row_mod.name, col_mod.name, count
        );

        // Collect entities from both modules that participate in cross-module relations
        let row_ids: std::collections::HashSet<usize> =
            row_mod.entity_ids.iter().copied().collect();
        let col_ids: std::collections::HashSet<usize> =
            col_mod.entity_ids.iter().copied().collect();

        let mut participating: Vec<usize> = Vec::new();
        for rel in &self.graph.relations {
            let from_in_row = row_ids.contains(&rel.from);
            let from_in_col = col_ids.contains(&rel.from);
            let to_in_row = row_ids.contains(&rel.to);
            let to_in_col = col_ids.contains(&rel.to);

            if (from_in_row && to_in_col) || (from_in_col && to_in_row) {
                participating.push(rel.from);
                participating.push(rel.to);
            }
        }
        participating.sort();
        participating.dedup();

        self.module_entities = participating;
        self.selected_entity_idx = 0;
        self.zoom = ZoomLevel::Entities;
    }

    fn enter_ego(&mut self, entity_id: usize) {
        self.center_entity = entity_id;
        self.ego_history.clear();
        self.recompute_ego();
        self.zoom = ZoomLevel::Ego;
    }

    fn recompute_ego(&mut self) {
        self.layout_nodes = ego_layout(&self.graph, self.center_entity);
        self.visible_neighbors = self
            .layout_nodes
            .iter()
            .filter(|n| n.entity_id != self.center_entity)
            .map(|n| n.entity_id)
            .collect();
        self.selected_neighbor = 0;
    }

    pub fn selected_entity_id(&self) -> Option<usize> {
        match self.zoom {
            ZoomLevel::Heatmap => None,
            ZoomLevel::Entities => self.module_entities.get(self.selected_entity_idx).copied(),
            ZoomLevel::Ego => {
                if self.visible_neighbors.is_empty() {
                    Some(self.center_entity)
                } else {
                    self.visible_neighbors.get(self.selected_neighbor).copied()
                }
            }
        }
    }

    // --- Cell info for details panel ---

    pub fn selected_cell_info(&self) -> CellInfo<'_> {
        let row = self.cursor_row;
        let col = self.cursor_col;
        let value = self.matrix[row][col];

        if row == col {
            CellInfo::Diagonal {
                module: &self.modules[row],
                entity_count: value,
            }
        } else {
            // Find the actual relations between these modules
            let row_ids: std::collections::HashSet<usize> =
                self.modules[row].entity_ids.iter().copied().collect();
            let col_ids: std::collections::HashSet<usize> =
                self.modules[col].entity_ids.iter().copied().collect();

            let mut relations: Vec<(usize, usize, crate::graph::RelationKind)> = Vec::new();
            for rel in &self.graph.relations {
                if (row_ids.contains(&rel.from) && col_ids.contains(&rel.to))
                    || (col_ids.contains(&rel.from) && row_ids.contains(&rel.to))
                {
                    relations.push((rel.from, rel.to, rel.kind));
                }
            }

            CellInfo::CrossModule {
                row_module: &self.modules[row],
                col_module: &self.modules[col],
                count: value,
                sample_relations: relations.into_iter().take(3).collect(),
            }
        }
    }

    // --- Search ---

    pub fn enter_search(&mut self) {
        self.search_mode = true;
        self.search_query.clear();
        self.search_results.clear();
        self.search_selected = 0;
    }

    pub fn search_input(&mut self, ch: char) {
        self.search_query.push(ch);
        self.update_search();
    }

    pub fn search_backspace(&mut self) {
        self.search_query.pop();
        self.update_search();
    }

    fn update_search(&mut self) {
        let query = self.search_query.to_lowercase();
        self.search_results = self
            .graph
            .entities
            .iter()
            .filter(|e| {
                e.name.to_lowercase().contains(&query)
                    || e.display_name().to_lowercase().contains(&query)
            })
            .map(|e| e.id)
            .take(20)
            .collect();
        self.search_selected = 0;
    }
}

pub enum CellInfo<'a> {
    Diagonal {
        module: &'a ModuleInfo,
        entity_count: usize,
    },
    CrossModule {
        row_module: &'a ModuleInfo,
        col_module: &'a ModuleInfo,
        count: usize,
        sample_relations: Vec<(usize, usize, crate::graph::RelationKind)>,
    },
}
