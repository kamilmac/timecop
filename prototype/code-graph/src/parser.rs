use crate::graph::{CodeGraph, EntityKind, RelationKind};
use std::collections::HashMap;
use std::path::Path;
use syn::visit::Visit;
use syn::{ItemEnum, ItemFn, ItemImpl, ItemStruct, ItemTrait, ReturnType, Type};
use walkdir::WalkDir;

pub fn parse_directory(root: &Path) -> CodeGraph {
    let mut graph = CodeGraph::new();

    // Parse Rust files
    parse_rust_files(root, &mut graph);

    // Parse TypeScript/JavaScript files
    crate::ts_parser::parse_directory(root, &mut graph);

    graph
}

fn parse_rust_files(root: &Path, graph: &mut CodeGraph) {
    let src_dir = root.join("src");
    let scan_dir = if src_dir.exists() { &src_dir } else { root };

    // First pass: collect all entities
    let mut fn_bodies: Vec<(usize, String)> = Vec::new(); // (entity_id, body_source)
    let mut fn_signatures: Vec<(usize, Vec<String>)> = Vec::new(); // (entity_id, type_names)

    for entry in WalkDir::new(scan_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map_or(false, |ext| ext == "rs")
        })
    {
        let path = entry.path();
        let rel_path = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let syntax = match syn::parse_file(&source) {
            Ok(f) => f,
            Err(_) => continue,
        };

        let mut extractor = Extractor {
            graph,
            fn_bodies: &mut fn_bodies,
            fn_signatures: &mut fn_signatures,
            file: rel_path,
            source: &source,
            current_impl_type: None,
        };
        extractor.visit_file(&syntax);
    }

    // Second pass: resolve call relations
    for (caller_id, body) in &fn_bodies {
        let calls = extract_calls_from_source(body);
        for call_name in calls {
            if let Some(target_id) = graph.find_by_name(&call_name) {
                graph.add_relation(*caller_id, target_id, RelationKind::Calls);
            }
        }
    }

    // Third pass: resolve type usage relations
    for (fn_id, type_names) in &fn_signatures {
        for type_name in type_names {
            if let Some(type_id) = graph.find_by_name(type_name) {
                graph.add_relation(*fn_id, type_id, RelationKind::UsesType);
            }
        }
    }

}

struct Extractor<'a> {
    graph: &'a mut CodeGraph,
    fn_bodies: &'a mut Vec<(usize, String)>,
    fn_signatures: &'a mut Vec<(usize, Vec<String>)>,
    file: String,
    source: &'a str,
    current_impl_type: Option<String>,
}

impl<'a> Extractor<'a> {
    fn source_range(&self, start_line: usize, end_line: usize) -> String {
        self.source
            .lines()
            .skip(start_line.saturating_sub(1))
            .take(end_line.saturating_sub(start_line) + 1)
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl<'a, 'ast> Visit<'ast> for Extractor<'a> {
    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        let name = node.sig.ident.to_string();
        let start = node.sig.ident.span().start().line;
        let end = node.block.brace_token.span.close().end().line;

        let id = self.graph.add_entity(
            name,
            None,
            EntityKind::Function,
            self.file.clone(),
            start,
            end,
        );

        let body_src = self.source_range(start, end);
        self.fn_bodies.push((id, body_src));

        let mut types = Vec::new();
        extract_sig_types(&node.sig, &mut types);
        if !types.is_empty() {
            self.fn_signatures.push((id, types));
        }

        syn::visit::visit_item_fn(self, node);
    }

    fn visit_item_struct(&mut self, node: &'ast ItemStruct) {
        let name = node.ident.to_string();
        let start = node.ident.span().start().line;
        let end = estimate_end(self.source, start);

        self.graph.add_entity(
            name,
            None,
            EntityKind::Struct,
            self.file.clone(),
            start,
            end,
        );

        syn::visit::visit_item_struct(self, node);
    }

    fn visit_item_enum(&mut self, node: &'ast ItemEnum) {
        let name = node.ident.to_string();
        let start = node.ident.span().start().line;
        let end = node.brace_token.span.close().end().line;

        self.graph.add_entity(
            name,
            None,
            EntityKind::Enum,
            self.file.clone(),
            start,
            end,
        );

        syn::visit::visit_item_enum(self, node);
    }

    fn visit_item_trait(&mut self, node: &'ast ItemTrait) {
        let name = node.ident.to_string();
        let start = node.ident.span().start().line;
        let end = node.brace_token.span.close().end().line;

        self.graph.add_entity(
            name,
            None,
            EntityKind::Trait,
            self.file.clone(),
            start,
            end,
        );

        syn::visit::visit_item_trait(self, node);
    }

    fn visit_item_impl(&mut self, node: &'ast ItemImpl) {
        let type_name = type_to_name(&node.self_ty);
        let prev = self.current_impl_type.take();
        self.current_impl_type = Some(type_name.clone());

        // Extract methods
        for item in &node.items {
            if let syn::ImplItem::Fn(method) = item {
                let name = method.sig.ident.to_string();
                let start = method.sig.ident.span().start().line;
                let end = method.block.brace_token.span.close().end().line;

                let id = self.graph.add_entity(
                    name,
                    Some(type_name.clone()),
                    EntityKind::Function,
                    self.file.clone(),
                    start,
                    end,
                );

                let body_src = self.source_range(start, end);
                self.fn_bodies.push((id, body_src));

                let mut types = Vec::new();
                extract_sig_types(&method.sig, &mut types);
                if !types.is_empty() {
                    self.fn_signatures.push((id, types));
                }

                // Link method to its containing type
                if let Some(type_id) = self.graph.find_by_name(&type_name) {
                    self.graph.add_relation(type_id, id, RelationKind::Contains);
                }
            }
        }

        self.current_impl_type = prev;
        // Don't recurse into impl items — we handled methods above
    }
}

fn extract_sig_types(sig: &syn::Signature, types: &mut Vec<String>) {
    for input in &sig.inputs {
        if let syn::FnArg::Typed(pat_type) = input {
            collect_type_names(&pat_type.ty, types);
        }
    }
    if let ReturnType::Type(_, ty) = &sig.output {
        collect_type_names(ty, types);
    }
}

fn collect_type_names(ty: &Type, names: &mut Vec<String>) {
    const SKIP: &[&str] = &[
        "Option", "Vec", "Result", "String", "Box", "Arc", "Rc", "HashMap", "HashSet", "bool",
        "usize", "u8", "u16", "u32", "u64", "i8", "i16", "i32", "i64", "f32", "f64", "str",
        "Self", "self",
    ];
    match ty {
        Type::Path(tp) => {
            if let Some(seg) = tp.path.segments.last() {
                let name = seg.ident.to_string();
                if !SKIP.contains(&name.as_str()) {
                    names.push(name);
                }
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    for arg in &args.args {
                        if let syn::GenericArgument::Type(inner) = arg {
                            collect_type_names(inner, names);
                        }
                    }
                }
            }
        }
        Type::Reference(tr) => collect_type_names(&tr.elem, names),
        Type::Slice(ts) => collect_type_names(&ts.elem, names),
        Type::Tuple(tt) => {
            for elem in &tt.elems {
                collect_type_names(elem, names);
            }
        }
        _ => {}
    }
}

fn type_to_name(ty: &Type) -> String {
    match ty {
        Type::Path(tp) => tp
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_else(|| "?".into()),
        _ => "?".into(),
    }
}

/// Simple call extraction by scanning source text for identifier patterns
/// followed by '('. Not perfect but good enough for a prototype.
fn extract_calls_from_source(source: &str) -> Vec<String> {
    let mut calls = Vec::new();
    let mut chars = source.chars().peekable();
    let mut current_ident = String::new();
    let mut in_string = false;
    let mut in_line_comment = false;
    let mut prev_char = ' ';

    while let Some(&ch) = chars.peek() {
        chars.next();

        if in_line_comment {
            if ch == '\n' {
                in_line_comment = false;
            }
            prev_char = ch;
            continue;
        }

        if ch == '/' && chars.peek() == Some(&'/') {
            in_line_comment = true;
            prev_char = ch;
            continue;
        }

        if ch == '"' && prev_char != '\\' {
            in_string = !in_string;
            prev_char = ch;
            continue;
        }

        if in_string {
            prev_char = ch;
            continue;
        }

        if ch.is_alphanumeric() || ch == '_' {
            current_ident.push(ch);
        } else {
            if ch == '(' && !current_ident.is_empty() {
                // Filter out keywords and common macros
                let kw = [
                    // keywords
                    "if", "else", "while", "for", "loop", "match", "return", "let", "mut", "ref",
                    "pub", "fn", "struct", "enum", "impl", "trait", "use", "mod", "where", "as",
                    "in",
                    // common constructors/conversions (too noisy)
                    "new", "default", "clone", "into", "from", "to_string", "to_owned",
                    "as_ref", "as_mut", "borrow", "borrow_mut", "deref", "deref_mut",
                    "unwrap", "unwrap_or", "unwrap_or_else", "unwrap_or_default",
                    "expect", "ok", "err", "map", "and_then", "or_else", "filter",
                    "map_err", "is_some", "is_none", "is_ok", "is_err",
                    // iterator methods
                    "iter", "into_iter", "collect", "enumerate", "zip", "take", "skip",
                    "chain", "flat_map", "fold", "any", "all", "find", "position",
                    "max", "min", "sum", "count", "last", "first", "nth", "rev",
                    "cloned", "copied", "peekable", "fuse", "by_ref",
                    // common std methods
                    "push", "pop", "insert", "remove", "contains", "get", "set",
                    "len", "is_empty", "clear", "extend", "retain", "drain",
                    "sort", "sort_by", "sort_by_key", "dedup", "dedup_by_key",
                    "join", "split", "trim", "starts_with", "ends_with", "replace",
                    "to_lowercase", "to_uppercase", "chars", "bytes", "lines",
                    "entry", "or_default", "or_insert", "or_insert_with",
                    "read", "write", "flush", "seek",
                    // formatting/macros
                    "Some", "None", "Ok", "Err", "vec", "format", "println", "eprintln",
                    "write", "writeln", "todo", "unimplemented", "unreachable", "assert",
                    "assert_eq", "assert_ne", "debug_assert", "cfg", "panic",
                ];
                if !kw.contains(&current_ident.as_str())
                    && current_ident.chars().next().map_or(false, |c| c.is_lowercase())
                {
                    calls.push(current_ident.clone());
                }
            }
            current_ident.clear();
        }
        prev_char = ch;
    }

    calls.sort();
    calls.dedup();
    calls
}

/// Estimate end line for struct definitions by counting braces
fn estimate_end(source: &str, start_line: usize) -> usize {
    let mut depth = 0i32;
    let mut found_open = false;
    for (i, line) in source.lines().enumerate() {
        let line_num = i + 1;
        if line_num < start_line {
            continue;
        }
        for ch in line.chars() {
            if ch == '{' {
                depth += 1;
                found_open = true;
            } else if ch == '}' {
                depth -= 1;
            }
        }
        if found_open && depth <= 0 {
            return line_num;
        }
    }
    start_line + 10 // fallback
}

// --- Git diff overlay ---

pub fn parse_git_diff(repo_root: &Path) -> HashMap<String, Vec<(usize, usize)>> {
    let output = std::process::Command::new("git")
        .args(["diff", "main", "--unified=0"])
        .current_dir(repo_root)
        .output();

    let output = match output {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
        Err(_) => return HashMap::new(),
    };

    let mut result: HashMap<String, Vec<(usize, usize)>> = HashMap::new();
    let mut current_file: Option<String> = None;

    for line in output.lines() {
        if line.starts_with("+++ b/") {
            current_file = Some(line[6..].to_string());
        } else if line.starts_with("@@ ") {
            if let Some(ref file) = current_file {
                if let Some(range) = parse_hunk_header(line) {
                    result.entry(file.clone()).or_default().push(range);
                }
            }
        }
    }

    result
}

fn parse_hunk_header(line: &str) -> Option<(usize, usize)> {
    // @@ -old,count +new,count @@
    let plus_part = line.split('+').nth(1)?;
    let nums = plus_part.split(' ').next()?;
    let parts: Vec<&str> = nums.split(',').collect();
    let start = parts[0].parse::<usize>().ok()?;
    let count = if parts.len() > 1 {
        parts[1].parse::<usize>().unwrap_or(1)
    } else {
        1
    };
    let end = start + count.max(1) - 1;
    Some((start, end))
}

pub fn mark_modified_entities(graph: &mut CodeGraph, diff: &HashMap<String, Vec<(usize, usize)>>) {
    for entity in &mut graph.entities {
        if let Some(ranges) = diff.get(&entity.file) {
            for &(start, end) in ranges {
                if entity.line <= end && entity.end_line >= start {
                    entity.modified = true;
                    break;
                }
            }
        }
    }
}
