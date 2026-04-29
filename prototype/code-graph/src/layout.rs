use crate::graph::{CodeGraph, ModuleInfo, RelationKind};

#[derive(Debug, Clone)]
pub struct LayoutNode {
    pub entity_id: usize,
    pub x: f64,
    pub y: f64,
}

/// Module-level layout: arrange modules by connectivity.
/// Most connected module in the center, others around it.
pub fn module_layout(
    modules: &[ModuleInfo],
    relations: &[(usize, usize, usize)],
) -> Vec<LayoutNode> {
    if modules.is_empty() {
        return Vec::new();
    }

    // Find most connected module
    let mut connectivity = vec![0usize; modules.len()];
    for &(a, b, count) in relations {
        connectivity[a] += count;
        connectivity[b] += count;
    }
    let center_idx = connectivity
        .iter()
        .enumerate()
        .max_by_key(|(_, c)| *c)
        .map(|(i, _)| i)
        .unwrap_or(0);

    let mut nodes = Vec::new();

    // Center module at origin (using module index as entity_id placeholder)
    nodes.push(LayoutNode {
        entity_id: center_idx,
        x: 0.0,
        y: 0.0,
    });

    // Others on a circle, sorted by connectivity to center
    let mut others: Vec<(usize, usize)> = (0..modules.len())
        .filter(|&i| i != center_idx)
        .map(|i| {
            let conn = relations
                .iter()
                .filter(|&&(a, b, _)| (a == center_idx && b == i) || (a == i && b == center_idx))
                .map(|&(_, _, c)| c)
                .sum::<usize>();
            (i, conn)
        })
        .collect();
    others.sort_by(|a, b| b.1.cmp(&a.1));

    let n = others.len();
    let radius = match n {
        0..=4 => 35.0,
        5..=8 => 42.0,
        9..=14 => 50.0,
        _ => 58.0,
    };

    for (i, &(mod_idx, _)) in others.iter().enumerate() {
        let angle = 2.0 * std::f64::consts::PI * (i as f64) / (n as f64)
            - std::f64::consts::FRAC_PI_2;
        nodes.push(LayoutNode {
            entity_id: mod_idx,
            x: radius * angle.cos(),
            y: radius * angle.sin(),
        });
    }

    nodes
}

/// Ego-graph layout: selected entity at center, neighbors on circle.
pub fn ego_layout(graph: &CodeGraph, center_id: usize) -> Vec<LayoutNode> {
    let mut nodes = Vec::new();

    nodes.push(LayoutNode {
        entity_id: center_id,
        x: 0.0,
        y: 0.0,
    });

    let neighbors = graph.neighbors(center_id);
    if neighbors.is_empty() {
        return nodes;
    }

    let mut outgoing_behavior: Vec<usize> = Vec::new();
    let mut outgoing_structural: Vec<usize> = Vec::new();
    let mut incoming: Vec<usize> = Vec::new();

    for (neighbor_id, kind, is_outgoing) in &neighbors {
        if *is_outgoing {
            match kind {
                RelationKind::Calls
                | RelationKind::Renders
                | RelationKind::Validates
                | RelationKind::HandlesError
                | RelationKind::Tests => outgoing_behavior.push(*neighbor_id),
                _ => outgoing_structural.push(*neighbor_id),
            }
        } else {
            incoming.push(*neighbor_id);
        }
    }

    let groups: Vec<(&[usize], f64)> = vec![
        (&outgoing_behavior, 0.0),
        (&outgoing_structural, 90.0),
        (&incoming, 180.0),
    ];

    let total = neighbors.len();
    let radius = if total <= 6 {
        40.0
    } else if total <= 15 {
        50.0
    } else {
        60.0
    };
    let sector_span = if total <= 10 { 70.0 } else { 85.0 };

    for (group, base_angle) in &groups {
        if group.is_empty() {
            continue;
        }
        let n = group.len();
        for (i, &entity_id) in group.iter().enumerate() {
            let offset = if n == 1 {
                0.0
            } else {
                (i as f64 / (n as f64 - 1.0) - 0.5) * sector_span
            };
            let angle_deg = base_angle + offset;
            let angle_rad = angle_deg.to_radians();

            nodes.push(LayoutNode {
                entity_id,
                x: radius * angle_rad.cos(),
                y: radius * angle_rad.sin(),
            });
        }
    }

    nodes
}
