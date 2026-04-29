use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntityKind {
    Function,
    Struct,
    Enum,
    Trait,
    Module,
    Component,
    Hook,
    Middleware,
    Route,
    Config,
    Test,
    Other,
}

impl EntityKind {
    pub fn short(&self) -> &str {
        match self {
            EntityKind::Function => "fn",
            EntityKind::Struct => "st",
            EntityKind::Enum => "en",
            EntityKind::Trait => "tr",
            EntityKind::Module => "mod",
            EntityKind::Component => "cmp",
            EntityKind::Hook => "hk",
            EntityKind::Middleware => "mw",
            EntityKind::Route => "rt",
            EntityKind::Config => "cfg",
            EntityKind::Test => "tst",
            EntityKind::Other => "?",
        }
    }

    pub fn label(&self) -> &str {
        match self {
            EntityKind::Function => "function",
            EntityKind::Struct => "struct",
            EntityKind::Enum => "enum",
            EntityKind::Trait => "trait",
            EntityKind::Module => "module",
            EntityKind::Component => "component",
            EntityKind::Hook => "hook",
            EntityKind::Middleware => "middleware",
            EntityKind::Route => "route",
            EntityKind::Config => "config",
            EntityKind::Test => "test",
            EntityKind::Other => "other",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RelationKind {
    Calls,
    UsesType,
    Contains,
    Implements,
    Imports,
    Configures,
    Tests,
    Validates,
    Renders,
    HandlesError,
    DependsOn,
}

impl RelationKind {
    pub fn label(&self) -> &str {
        match self {
            RelationKind::Calls => "calls",
            RelationKind::UsesType => "uses",
            RelationKind::Contains => "contains",
            RelationKind::Implements => "implements",
            RelationKind::Imports => "imports",
            RelationKind::Configures => "configures",
            RelationKind::Tests => "tests",
            RelationKind::Validates => "validates",
            RelationKind::Renders => "renders",
            RelationKind::HandlesError => "handles error",
            RelationKind::DependsOn => "depends on",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: usize,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    pub kind: EntityKind,
    pub file: String,
    pub line: usize,
    #[serde(default)]
    pub end_line: usize,
    #[serde(default)]
    pub modified: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Entity {
    pub fn display_name(&self) -> String {
        if let Some(ref owner) = self.owner {
            format!("{}::{}", owner, self.name)
        } else {
            self.name.clone()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub from: usize,
    pub to: usize,
    pub kind: RelationKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// JSON-serializable graph format for agent-generated maps
#[derive(Debug, Serialize, Deserialize)]
pub struct GraphJson {
    pub entities: Vec<Entity>,
    pub relations: Vec<Relation>,
}

pub struct CodeGraph {
    pub entities: Vec<Entity>,
    pub relations: Vec<Relation>,
    name_index: HashMap<String, Vec<usize>>,
}

impl CodeGraph {
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
            relations: Vec::new(),
            name_index: HashMap::new(),
        }
    }

    pub fn from_json(json: GraphJson) -> Self {
        let mut graph = Self::new();
        for entity in json.entities {
            let name = entity.name.clone();
            let owner = entity.owner.clone();
            let id = graph.entities.len();
            graph.name_index.entry(name.clone()).or_default().push(id);
            if let Some(ref o) = owner {
                let qualified = format!("{}::{}", o, name);
                graph.name_index.entry(qualified).or_default().push(id);
            }
            graph.entities.push(Entity { id, ..entity });
        }
        graph.relations = json.relations;
        graph
    }

    pub fn to_json(&self) -> GraphJson {
        GraphJson {
            entities: self.entities.clone(),
            relations: self.relations.clone(),
        }
    }

    pub fn add_entity(
        &mut self,
        name: String,
        owner: Option<String>,
        kind: EntityKind,
        file: String,
        line: usize,
        end_line: usize,
    ) -> usize {
        let id = self.entities.len();
        self.name_index.entry(name.clone()).or_default().push(id);
        if let Some(ref o) = owner {
            let qualified = format!("{}::{}", o, name);
            self.name_index.entry(qualified).or_default().push(id);
        }
        self.entities.push(Entity {
            id,
            name,
            owner,
            kind,
            file,
            line,
            end_line,
            modified: false,
            description: None,
        });
        id
    }

    pub fn add_relation(&mut self, from: usize, to: usize, kind: RelationKind) {
        if from != to
            && !self
                .relations
                .iter()
                .any(|r| r.from == from && r.to == to && r.kind == kind)
        {
            self.relations.push(Relation {
                from,
                to,
                kind,
                label: None,
            });
        }
    }

    pub fn neighbors(&self, id: usize) -> Vec<(usize, RelationKind, bool)> {
        let mut result = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for rel in &self.relations {
            if rel.from == id && seen.insert(rel.to) {
                result.push((rel.to, rel.kind, true));
            } else if rel.to == id && seen.insert(rel.from) {
                result.push((rel.from, rel.kind, false));
            }
        }
        result
    }

    pub fn find_by_name(&self, name: &str) -> Option<usize> {
        self.name_index
            .get(name)
            .and_then(|ids| ids.first())
            .copied()
    }

    pub fn most_connected(&self) -> Option<usize> {
        if self.entities.is_empty() {
            return None;
        }
        let mut counts = vec![0usize; self.entities.len()];
        for rel in &self.relations {
            counts[rel.from] += 1;
            counts[rel.to] += 1;
        }
        counts
            .iter()
            .enumerate()
            .max_by_key(|(_, c)| *c)
            .map(|(id, _)| id)
    }

    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    pub fn relation_count(&self) -> usize {
        self.relations.len()
    }

    /// Group entities into modules by file path.
    pub fn compute_modules(&self) -> Vec<ModuleInfo> {
        let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
        for entity in &self.entities {
            let module = module_name_from_path(&entity.file);
            groups.entry(module).or_default().push(entity.id);
        }

        let mut modules: Vec<ModuleInfo> = groups
            .into_iter()
            .map(|(name, entity_ids)| {
                let modified_count = entity_ids
                    .iter()
                    .filter(|id| self.entities[**id].modified)
                    .count();
                let files: Vec<String> = {
                    let mut f: Vec<String> = entity_ids
                        .iter()
                        .map(|id| self.entities[*id].file.clone())
                        .collect();
                    f.sort();
                    f.dedup();
                    f
                };
                let entity_count = entity_ids.len();
                ModuleInfo {
                    name,
                    entity_ids,
                    entity_count,
                    modified_count,
                    files,
                }
            })
            .collect();

        modules.sort_by(|a, b| b.entity_count.cmp(&a.entity_count));
        modules
    }

    /// Compute aggregate relations between modules.
    /// Returns (from_module_idx, to_module_idx, relation_count).
    pub fn module_relations(&self, modules: &[ModuleInfo]) -> Vec<(usize, usize, usize)> {
        // Build entity_id → module_index map
        let mut entity_to_module: HashMap<usize, usize> = HashMap::new();
        for (mi, module) in modules.iter().enumerate() {
            for &eid in &module.entity_ids {
                entity_to_module.insert(eid, mi);
            }
        }

        // Count cross-module relations
        let mut edge_counts: HashMap<(usize, usize), usize> = HashMap::new();
        for rel in &self.relations {
            if let (Some(&from_mod), Some(&to_mod)) = (
                entity_to_module.get(&rel.from),
                entity_to_module.get(&rel.to),
            ) {
                if from_mod != to_mod {
                    let key = if from_mod < to_mod {
                        (from_mod, to_mod)
                    } else {
                        (to_mod, from_mod)
                    };
                    *edge_counts.entry(key).or_default() += 1;
                }
            }
        }

        let mut result: Vec<(usize, usize, usize)> = edge_counts
            .into_iter()
            .map(|((a, b), count)| (a, b, count))
            .collect();
        result.sort_by(|a, b| b.2.cmp(&a.2));
        result
    }
}

#[derive(Debug, Clone)]
pub struct ModuleInfo {
    pub name: String,
    pub entity_ids: Vec<usize>,
    pub entity_count: usize,
    pub modified_count: usize,
    pub files: Vec<String>,
}

/// Derive a human-readable module name from a file path.
fn module_name_from_path(path: &str) -> String {
    let path = path
        .strip_prefix("src/")
        .unwrap_or(path);

    let parts: Vec<&str> = path.split('/').collect();

    match parts.len() {
        0 => "root".to_string(),
        1 => {
            // Top-level file: src/app.rs → "app"
            parts[0]
                .strip_suffix(".rs")
                .or_else(|| parts[0].strip_suffix(".ts"))
                .or_else(|| parts[0].strip_suffix(".tsx"))
                .or_else(|| parts[0].strip_suffix(".js"))
                .or_else(|| parts[0].strip_suffix(".jsx"))
                .or_else(|| parts[0].strip_suffix(".py"))
                .or_else(|| parts[0].strip_suffix(".go"))
                .unwrap_or(parts[0])
                .to_string()
        }
        2 => {
            // src/git/client.rs → "git"
            parts[0].to_string()
        }
        _ => {
            // src/ui/widgets/file_list/mod.rs → "file_list"
            // src/ui/widgets/diff_view/mod.rs → "diff_view"
            let file = parts.last().unwrap();
            if *file == "mod.rs" || *file == "index.ts" || *file == "index.tsx" || *file == "index.js" {
                // Use the parent directory name
                parts[parts.len() - 2].to_string()
            } else {
                // Use the deepest directory
                parts[parts.len() - 2].to_string()
            }
        }
    }
}
