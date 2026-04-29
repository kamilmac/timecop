use crate::app::{App, CellInfo, ZoomLevel};
use crate::graph::{EntityKind, RelationKind};
use crate::layout::LayoutNode;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::canvas::{Canvas, Line as CanvasLine};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(10),
            Constraint::Length(5),
            Constraint::Length(1),
        ])
        .split(frame.area());

    if app.search_mode {
        render_search(frame, chunks[0], app);
    } else {
        match app.zoom {
            ZoomLevel::Heatmap => render_heatmap(frame, chunks[0], app),
            ZoomLevel::Entities => render_entities(frame, chunks[0], app),
            ZoomLevel::Ego => render_ego(frame, chunks[0], app),
        }
    }

    render_details(frame, chunks[1], app);
    render_help(frame, chunks[2], app);
}

// ─── Coupling heatmap ────────────────────────────────────────────

fn render_heatmap(frame: &mut Frame, area: Rect, app: &App) {
    let modules = &app.modules;
    let matrix = &app.matrix;
    let n = modules.len();

    if n == 0 {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Coupling Heatmap — no modules ");
        frame.render_widget(block, area);
        return;
    }

    let title = format!(
        " Coupling Heatmap [{} modules, {} entities] ",
        n,
        app.graph.entity_count(),
    );

    // Layout: label column + matrix cells
    let label_width = modules
        .iter()
        .map(|m| m.name.len())
        .max()
        .unwrap_or(4)
        .min(14)
        + 2; // padding
    let cell_width = 5usize;

    let inner_height = area.height.saturating_sub(4) as usize; // borders + header rows
    let inner_width = area.width.saturating_sub(2) as usize; // borders

    // Scrolling: keep cursor_row visible
    let visible_rows = inner_height.saturating_sub(2); // minus column header lines
    let scroll_row = if app.cursor_row >= visible_rows {
        app.cursor_row - visible_rows + 1
    } else {
        0
    };

    // Column header scroll: keep cursor_col visible
    let max_visible_cols = (inner_width.saturating_sub(label_width)) / cell_width;
    let scroll_col = if app.cursor_col >= max_visible_cols {
        app.cursor_col - max_visible_cols + 1
    } else {
        0
    };

    let mut lines: Vec<Line> = Vec::new();

    // Column headers — two rows: truncated names rotated isn't practical in TUI,
    // so we show short indices and a legend

    // Row 1: column indices
    let mut header_spans = vec![Span::styled(
        format!("{:>width$}", "", width = label_width),
        Style::default(),
    )];
    for col in scroll_col..n.min(scroll_col + max_visible_cols) {
        let is_sel_col = col == app.cursor_col;
        let style = if is_sel_col {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let name = &modules[col].name;
        let short: String = if name.len() > cell_width - 1 {
            name[..cell_width - 1].to_string()
        } else {
            name.clone()
        };
        header_spans.push(Span::styled(
            format!("{:>width$}", short, width = cell_width),
            style,
        ));
    }
    lines.push(Line::from(header_spans));

    // Separator
    let sep_len = label_width + (n.min(scroll_col + max_visible_cols) - scroll_col) * cell_width;
    lines.push(Line::from(Span::styled(
        "─".repeat(sep_len.min(inner_width)),
        Style::default().fg(Color::DarkGray),
    )));

    // Matrix rows
    for row in scroll_row..n.min(scroll_row + visible_rows) {
        let is_sel_row = row == app.cursor_row;
        let module = &modules[row];

        // Row label
        let label: String = if module.name.len() > label_width - 2 {
            format!(
                " {:width$}",
                &module.name[..label_width - 2],
                width = label_width - 2
            )
        } else {
            format!(" {:width$}", module.name, width = label_width - 2)
        };

        let label_style = if is_sel_row {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else if module.modified_count > 0 {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(Color::Cyan)
        };

        let mut row_spans = vec![Span::styled(
            format!("{} ", label),
            label_style,
        )];

        // Cells
        for col in scroll_col..n.min(scroll_col + max_visible_cols) {
            let value = matrix[row][col];
            let is_cursor = row == app.cursor_row && col == app.cursor_col;
            let is_crosshair = is_sel_row || col == app.cursor_col;
            let is_diagonal = row == col;

            let cell_text = if value == 0 && !is_diagonal {
                format!("{:>width$}", "·", width = cell_width)
            } else {
                format!("{:>width$}", value, width = cell_width)
            };

            let style = if is_cursor {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else if is_diagonal {
                let bg = if is_crosshair {
                    Color::Rgb(30, 50, 80)
                } else {
                    Color::Reset
                };
                Style::default()
                    .fg(diagonal_color(value))
                    .bg(bg)
                    .add_modifier(Modifier::BOLD)
            } else if is_crosshair {
                Style::default()
                    .fg(heat_color(value, app.max_cell_value))
                    .bg(Color::Rgb(25, 25, 35))
            } else {
                Style::default().fg(heat_color(value, app.max_cell_value))
            };

            row_spans.push(Span::styled(cell_text, style));
        }

        lines.push(Line::from(row_spans));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(title);
    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, area);
}

fn heat_color(value: usize, max_value: usize) -> Color {
    if value == 0 {
        return Color::Rgb(40, 40, 40);
    }
    if max_value == 0 {
        return Color::DarkGray;
    }

    let ratio = (value as f64 / max_value as f64).min(1.0);

    if ratio < 0.25 {
        Color::Rgb(80, 80, 60)
    } else if ratio < 0.5 {
        Color::Yellow
    } else if ratio < 0.75 {
        Color::Rgb(255, 165, 0) // orange
    } else {
        Color::Red
    }
}

fn diagonal_color(entity_count: usize) -> Color {
    if entity_count > 30 {
        Color::LightCyan
    } else if entity_count > 10 {
        Color::Cyan
    } else {
        Color::Blue
    }
}

// ─── Entity list ─────────────────────────────────────────────────

fn render_entities(frame: &mut Frame, area: Rect, app: &App) {
    let graph = &app.graph;

    let title = format!(" {} ", app.entity_list_title);

    let visible_height = area.height.saturating_sub(2) as usize;
    let selected = app.selected_entity_idx;
    let scroll_offset = if selected >= visible_height {
        selected - visible_height + 1
    } else {
        0
    };

    let mut lines: Vec<Line> = Vec::new();

    for (i, &entity_id) in app.module_entities.iter().enumerate() {
        if i < scroll_offset {
            continue;
        }
        if lines.len() >= visible_height {
            break;
        }

        let entity = &graph.entities[entity_id];
        let is_selected = i == selected;
        let indicator = if entity.modified { "●" } else { "○" };
        let neighbor_count = graph.neighbors(entity_id).len();

        let name_style = if is_selected {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else if entity.modified {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(kind_color(entity.kind))
        };

        let mut spans = vec![
            Span::styled(
                format!(" {} {} ", indicator, entity.kind.short()),
                Style::default().fg(if entity.modified {
                    Color::Red
                } else {
                    Color::DarkGray
                }),
            ),
            Span::styled(
                format!("{:<30}", entity.display_name()),
                name_style,
            ),
            Span::styled(
                format!("  {:>2} connections", neighbor_count),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("  {}:{}", entity.file, entity.line),
                Style::default().fg(Color::DarkGray),
            ),
        ];

        if let Some(ref desc) = entity.description {
            spans.push(Span::styled(
                format!("  {}", desc),
                Style::default().fg(Color::Gray),
            ));
        }

        lines.push(Line::from(spans));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(title);
    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, area);
}

// ─── Ego graph ────────────────────────────────────────────────────

fn render_ego(frame: &mut Frame, area: Rect, app: &App) {
    let graph = &app.graph;
    let layout = &app.layout_nodes;
    let center_id = app.center_entity;
    let selected_neighbor = app
        .visible_neighbors
        .get(app.selected_neighbor)
        .copied();

    let (x_bound, y_bound) = compute_bounds(layout);
    let center_name = graph.entities[center_id].display_name();

    let title = format!(
        " {} — {} connections ",
        center_name,
        app.visible_neighbors.len(),
    );

    let canvas = Canvas::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .x_bounds([-x_bound, x_bound])
        .y_bounds([-y_bound, y_bound])
        .paint(|ctx| {
            // Draw edges
            let center_pos = layout.iter().find(|n| n.entity_id == center_id);
            if let Some(center) = center_pos {
                for node in layout.iter() {
                    if node.entity_id == center_id {
                        continue;
                    }
                    let color = edge_color(graph, center_id, node.entity_id);
                    ctx.draw(&CanvasLine {
                        x1: center.x,
                        y1: center.y,
                        x2: node.x,
                        y2: node.y,
                        color,
                    });
                }
            }

            // Draw node labels
            for node in layout.iter() {
                let entity = &graph.entities[node.entity_id];
                let is_center = node.entity_id == center_id;
                let is_selected = selected_neighbor == Some(node.entity_id);

                let indicator = if entity.modified { "●" } else { "○" };
                let label = format!(
                    "{} {} {}",
                    indicator,
                    entity.kind.short(),
                    entity.display_name()
                );
                let label_width = label.len() as f64;

                let style = node_style(entity.kind, entity.modified, is_center, is_selected);

                ctx.print(
                    node.x - label_width * 0.6,
                    node.y,
                    Line::from(Span::styled(label, style)),
                );

                if is_center {
                    let info = format!("{}:{}", entity.file, entity.line);
                    let info_width = info.len() as f64;
                    ctx.print(
                        node.x - info_width * 0.6,
                        node.y - 2.5,
                        Line::from(Span::styled(info, Style::default().fg(Color::DarkGray))),
                    );
                }
            }
        });

    frame.render_widget(canvas, area);
}

// ─── Search overlay ───────────────────────────────────────────────

fn render_search(frame: &mut Frame, area: Rect, app: &App) {
    let graph = &app.graph;

    let mut lines = vec![
        Line::from(vec![
            Span::styled(" Search: ", Style::default().fg(Color::Yellow)),
            Span::styled(
                &app.search_query,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("█", Style::default().fg(Color::Yellow)),
        ]),
        Line::from(""),
    ];

    for (i, &entity_id) in app.search_results.iter().enumerate() {
        let entity = &graph.entities[entity_id];
        let is_selected = i == app.search_selected;
        let indicator = if entity.modified { "●" } else { "○" };

        let style = if is_selected {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else {
            Style::default().fg(kind_color(entity.kind))
        };

        lines.push(Line::from(vec![
            Span::styled(
                format!(
                    "  {} {} {} ",
                    indicator,
                    entity.kind.short(),
                    entity.display_name()
                ),
                style,
            ),
            Span::styled(
                format!("  {}:{}", entity.file, entity.line),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    }

    if app.search_results.is_empty() && !app.search_query.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No matches",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(" Search Entities ");
    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, area);
}

// ─── Details panel ────────────────────────────────────────────────

fn render_details(frame: &mut Frame, area: Rect, app: &App) {
    let graph = &app.graph;

    let lines = match app.zoom {
        ZoomLevel::Heatmap => {
            let info = app.selected_cell_info();
            match info {
                CellInfo::Diagonal {
                    module,
                    entity_count,
                } => {
                    let mut lines = vec![
                        Line::from(vec![
                            Span::styled(
                                format!(" {} ", module.name),
                                Style::default()
                                    .fg(Color::Cyan)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(
                                format!("{} entities", entity_count),
                                Style::default().fg(Color::White),
                            ),
                            if module.modified_count > 0 {
                                Span::styled(
                                    format!("  ● {} modified", module.modified_count),
                                    Style::default().fg(Color::Red),
                                )
                            } else {
                                Span::raw("")
                            },
                        ]),
                        Line::from(Span::styled(
                            format!(" files: {}", module.files.join(", ")),
                            Style::default().fg(Color::DarkGray),
                        )),
                    ];
                    lines.push(Line::from(Span::styled(
                        " Enter to drill into module",
                        Style::default().fg(Color::DarkGray),
                    )));
                    lines
                }
                CellInfo::CrossModule {
                    row_module,
                    col_module,
                    count,
                    sample_relations,
                } => {
                    let mut lines = vec![Line::from(vec![
                        Span::styled(
                            format!(" {} ", row_module.name),
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("↔ {} ", col_module.name),
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("{} relations", count),
                            Style::default().fg(Color::White),
                        ),
                    ])];
                    // Show sample relations
                    for (from_id, to_id, kind) in &sample_relations {
                        let from_name = graph.entities[*from_id].display_name();
                        let to_name = graph.entities[*to_id].display_name();
                        lines.push(Line::from(vec![
                            Span::styled(
                                format!("   {} ", from_name),
                                Style::default().fg(Color::White),
                            ),
                            Span::styled(
                                format!("─{}─▶ ", kind.label()),
                                Style::default().fg(edge_color_for_kind(*kind)),
                            ),
                            Span::styled(to_name, Style::default().fg(Color::White)),
                        ]));
                    }
                    if count > sample_relations.len() {
                        lines.push(Line::from(Span::styled(
                            format!(
                                "   ... and {} more. Enter to see all",
                                count - sample_relations.len()
                            ),
                            Style::default().fg(Color::DarkGray),
                        )));
                    }
                    lines
                }
            }
        }
        ZoomLevel::Entities | ZoomLevel::Ego => {
            let sel_id = if app.search_mode {
                app.search_results.get(app.search_selected).copied()
            } else {
                app.selected_entity_id()
            };

            if let Some(sel_id) = sel_id {
                let entity = &graph.entities[sel_id];
                let mut lines = vec![
                    Line::from(vec![
                        Span::styled(
                            format!(" {} ", entity.kind.label()),
                            Style::default()
                                .fg(kind_color(entity.kind))
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            entity.display_name(),
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        if entity.modified {
                            Span::styled("  ● modified", Style::default().fg(Color::Red))
                        } else {
                            Span::raw("")
                        },
                    ]),
                    Line::from(Span::styled(
                        format!(" {}:{}", entity.file, entity.line),
                        Style::default().fg(Color::DarkGray),
                    )),
                ];
                if let Some(ref desc) = entity.description {
                    lines.push(Line::from(Span::styled(
                        format!(" {}", desc),
                        Style::default().fg(Color::White),
                    )));
                }
                // Show relation to center (ego mode)
                if app.zoom == ZoomLevel::Ego && sel_id != app.center_entity {
                    if let Some(rel) = graph.relations.iter().find(|r| {
                        (r.from == app.center_entity && r.to == sel_id)
                            || (r.from == sel_id && r.to == app.center_entity)
                    }) {
                        let center = &graph.entities[app.center_entity];
                        let (from_name, to_name) = if rel.from == app.center_entity {
                            (center.display_name(), entity.display_name())
                        } else {
                            (entity.display_name(), center.display_name())
                        };
                        lines.push(Line::from(vec![
                            Span::styled(" ", Style::default()),
                            Span::styled(from_name, Style::default().fg(Color::White)),
                            Span::styled(
                                format!(" ─{}─▶ ", rel.kind.label()),
                                Style::default().fg(edge_color_for_kind(rel.kind)),
                            ),
                            Span::styled(to_name, Style::default().fg(Color::White)),
                        ]));
                    }
                }
                lines
            } else {
                vec![Line::from(Span::styled(
                    " No entity selected",
                    Style::default().fg(Color::DarkGray),
                ))]
            }
        }
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Details ");
    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, area);
}

// ─── Help bar ─────────────────────────────────────────────────────

fn render_help(frame: &mut Frame, area: Rect, app: &App) {
    let help = match app.zoom {
        ZoomLevel::Heatmap => Line::from(vec![
            Span::styled(
                " [HEATMAP] ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled("j/k", Style::default().fg(Color::Yellow)),
            Span::raw(" row  "),
            Span::styled("h/l", Style::default().fg(Color::Yellow)),
            Span::raw(" col  "),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw(" drill in  "),
            Span::styled("/", Style::default().fg(Color::Yellow)),
            Span::raw(" search  "),
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::raw(" quit  "),
            Span::styled("●", Style::default().fg(Color::Red)),
            Span::raw(" modified"),
        ]),
        ZoomLevel::Entities => Line::from(vec![
            Span::styled(
                " [ENTITIES] ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled("j/k", Style::default().fg(Color::Yellow)),
            Span::raw(" navigate  "),
            Span::styled("Enter/l", Style::default().fg(Color::Yellow)),
            Span::raw(" zoom in  "),
            Span::styled("Esc/h", Style::default().fg(Color::Yellow)),
            Span::raw(" back  "),
            Span::styled("/", Style::default().fg(Color::Yellow)),
            Span::raw(" search  "),
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::raw(" quit"),
        ]),
        ZoomLevel::Ego => Line::from(vec![
            Span::styled(
                " [EGO] ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled("j/k", Style::default().fg(Color::Yellow)),
            Span::raw(" select  "),
            Span::styled("Enter/l", Style::default().fg(Color::Yellow)),
            Span::raw(" follow  "),
            Span::styled("Esc/h", Style::default().fg(Color::Yellow)),
            Span::raw(" back  "),
            Span::styled("/", Style::default().fg(Color::Yellow)),
            Span::raw(" search  "),
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::raw(" quit"),
        ]),
    };
    frame.render_widget(Paragraph::new(help), area);
}

// ─── Styling helpers ──────────────────────────────────────────────

fn compute_bounds(layout: &[LayoutNode]) -> (f64, f64) {
    if layout.is_empty() {
        return (100.0, 50.0);
    }
    let max_x = layout.iter().map(|n| n.x.abs()).fold(0.0_f64, f64::max);
    let max_y = layout.iter().map(|n| n.y.abs()).fold(0.0_f64, f64::max);
    (max_x + 30.0, max_y + 10.0)
}

fn node_style(kind: EntityKind, modified: bool, is_center: bool, is_selected: bool) -> Style {
    let base_color = if modified {
        Color::Red
    } else if is_center {
        Color::White
    } else {
        kind_color(kind)
    };

    let mut style = Style::default().fg(base_color);
    if is_center {
        style = style.add_modifier(Modifier::BOLD);
    }
    if is_selected {
        style = style.add_modifier(Modifier::BOLD | Modifier::REVERSED);
    }
    style
}

fn kind_color(kind: EntityKind) -> Color {
    match kind {
        EntityKind::Function => Color::Cyan,
        EntityKind::Struct => Color::Green,
        EntityKind::Enum => Color::Yellow,
        EntityKind::Trait => Color::Magenta,
        EntityKind::Module => Color::Blue,
        EntityKind::Component => Color::LightCyan,
        EntityKind::Hook => Color::LightMagenta,
        EntityKind::Middleware => Color::LightYellow,
        EntityKind::Route => Color::LightGreen,
        EntityKind::Config => Color::Gray,
        EntityKind::Test => Color::DarkGray,
        EntityKind::Other => Color::White,
    }
}

fn edge_color(graph: &crate::graph::CodeGraph, center_id: usize, neighbor_id: usize) -> Color {
    for rel in &graph.relations {
        if (rel.from == center_id && rel.to == neighbor_id)
            || (rel.from == neighbor_id && rel.to == center_id)
        {
            return edge_color_for_kind(rel.kind);
        }
    }
    Color::DarkGray
}

fn edge_color_for_kind(kind: RelationKind) -> Color {
    match kind {
        RelationKind::Calls => Color::Cyan,
        RelationKind::UsesType => Color::Yellow,
        RelationKind::Contains => Color::Magenta,
        RelationKind::Implements => Color::LightMagenta,
        RelationKind::Imports => Color::DarkGray,
        RelationKind::Configures => Color::Gray,
        RelationKind::Tests => Color::LightGreen,
        RelationKind::Validates => Color::LightYellow,
        RelationKind::Renders => Color::LightCyan,
        RelationKind::HandlesError => Color::Red,
        RelationKind::DependsOn => Color::Blue,
    }
}
