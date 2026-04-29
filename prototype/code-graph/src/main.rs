mod app;
mod graph;
mod layout;
mod parser;
mod ts_parser;
mod ui;

use app::App;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use graph::{CodeGraph, GraphJson};
use ratatui::prelude::*;
use std::io;
use std::path::PathBuf;

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let stats_only = args.iter().any(|a| a == "--stats");
    let export_json = args.iter().any(|a| a == "--export-json");
    let from_json = args
        .iter()
        .position(|a| a == "--from-json")
        .and_then(|i| args.get(i + 1))
        .cloned();

    let positional = args
        .iter()
        .skip(1)
        .filter(|a| !a.starts_with('-'))
        .cloned()
        .collect::<Vec<_>>();

    // If --from-json is used, positional is the json path (already captured)
    // Otherwise positional is the repo path
    let repo_path = if from_json.is_some() {
        // Still need a repo path for diff overlay — use positional or cwd
        positional
            .first()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
    } else {
        positional
            .first()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
    };

    let repo_path = std::fs::canonicalize(&repo_path).unwrap_or(repo_path);

    let graph = if let Some(json_path) = from_json {
        // Load from agent-generated JSON
        eprintln!("Loading graph from: {}", json_path);
        let json_str = std::fs::read_to_string(&json_path).map_err(|e| {
            io::Error::new(io::ErrorKind::NotFound, format!("Cannot read {}: {}", json_path, e))
        })?;
        let graph_json: GraphJson = serde_json::from_str(&json_str).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, format!("Invalid JSON: {}", e))
        })?;
        let graph = CodeGraph::from_json(graph_json);
        eprintln!(
            "Loaded {} entities, {} relations",
            graph.entity_count(),
            graph.relation_count()
        );
        graph
    } else {
        // Parse from source
        eprintln!("Parsing codebase at: {}", repo_path.display());
        let mut graph = parser::parse_directory(&repo_path);
        eprintln!(
            "Found {} entities, {} relations",
            graph.entity_count(),
            graph.relation_count()
        );

        // Overlay diff information
        let diff = parser::parse_git_diff(&repo_path);
        if !diff.is_empty() {
            let modified_files: Vec<_> = diff.keys().collect();
            eprintln!("Diff overlay: {} files modified", modified_files.len());
            parser::mark_modified_entities(&mut graph, &diff);
            let modified_count = graph.entities.iter().filter(|e| e.modified).count();
            eprintln!("  {} entities marked as modified", modified_count);
        }

        if export_json {
            let json = serde_json::to_string_pretty(&graph.to_json()).unwrap();
            println!("{}", json);
            return Ok(());
        }

        graph
    };

    if graph.entity_count() == 0 {
        eprintln!("No entities found.");
        return Ok(());
    }

    if stats_only {
        print_stats(&graph);
        return Ok(());
    }

    let app = App::new(graph);

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_loop(&mut terminal, app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn print_stats(graph: &CodeGraph) {
    // Module overview
    let modules = graph.compute_modules();
    let mod_rels = graph.module_relations(&modules);
    eprintln!("\nModules ({}):", modules.len());
    for module in &modules {
        let mod_marker = if module.modified_count > 0 {
            format!(" ● {}", module.modified_count)
        } else {
            String::new()
        };
        eprintln!(
            "  {:<20} {:>3} entities  [{}]{}",
            module.name,
            module.entity_count,
            module.files.join(", "),
            mod_marker,
        );
    }
    eprintln!("\nModule connections ({}):", mod_rels.len());
    for &(a, b, count) in mod_rels.iter().take(15) {
        eprintln!(
            "  {} ←({})→ {}",
            modules[a].name, count, modules[b].name,
        );
    }

    let mut counts: Vec<(usize, usize)> = vec![];
    for i in 0..graph.entity_count() {
        let n = graph.neighbors(i).len();
        counts.push((i, n));
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1));
    eprintln!("\nTop 20 most connected:");
    for (id, count) in counts.iter().take(20) {
        let e = &graph.entities[*id];
        let mod_marker = if e.modified { " ●" } else { "" };
        let desc = e
            .description
            .as_ref()
            .map(|d| format!("  -- {}", d))
            .unwrap_or_default();
        eprintln!(
            "  {:>3} connections  {} {} ({}:{}){}{}",
            count,
            e.kind.short(),
            e.display_name(),
            e.file,
            e.line,
            mod_marker,
            desc,
        );
    }
    let modified: Vec<_> = graph.entities.iter().filter(|e| e.modified).collect();
    if !modified.is_empty() {
        eprintln!("\nModified entities:");
        for e in &modified {
            eprintln!(
                "  {} {} ({}:{})",
                e.kind.short(),
                e.display_name(),
                e.file,
                e.line
            );
        }
    }
}

fn run_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|frame| ui::render(frame, &app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            if app.search_mode {
                match key.code {
                    KeyCode::Esc => app.zoom_out(),
                    KeyCode::Enter => app.zoom_in(),
                    KeyCode::Backspace => app.search_backspace(),
                    KeyCode::Up => app.move_vertical(-1),
                    KeyCode::Down => app.move_vertical(1),
                    KeyCode::Char(c) => app.search_input(c),
                    _ => {}
                }
            } else {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('j') | KeyCode::Down => app.move_vertical(1),
                    KeyCode::Char('k') | KeyCode::Up => app.move_vertical(-1),
                    KeyCode::Char('J') => {
                        for _ in 0..5 {
                            app.move_vertical(1);
                        }
                    }
                    KeyCode::Char('K') => {
                        for _ in 0..5 {
                            app.move_vertical(-1);
                        }
                    }
                    KeyCode::Char('h') | KeyCode::Left => {
                        if app.zoom == app::ZoomLevel::Heatmap {
                            app.move_horizontal(-1);
                        } else {
                            app.zoom_out();
                        }
                    }
                    KeyCode::Char('l') | KeyCode::Right => {
                        if app.zoom == app::ZoomLevel::Heatmap {
                            app.move_horizontal(1);
                        } else {
                            app.zoom_in();
                        }
                    }
                    KeyCode::Enter => app.zoom_in(),
                    KeyCode::Esc | KeyCode::Backspace => app.zoom_out(),
                    KeyCode::Char('/') => app.enter_search(),
                    _ => {}
                }
            }
        }
    }
}
