use crate::graph::{CodeGraph, EntityKind, RelationKind};
use std::path::Path;
use walkdir::WalkDir;

/// Parse TypeScript/JavaScript files from a directory into the graph.
/// Uses regex-like text scanning — no AST parser dependency.
pub fn parse_directory(root: &Path, graph: &mut CodeGraph) {
    let src_dir = root.join("src");
    let scan_dir = if src_dir.exists() { &src_dir } else { root };

    let mut fn_bodies: Vec<(usize, String)> = Vec::new();
    let mut fn_signatures: Vec<(usize, Vec<String>)> = Vec::new();

    for entry in WalkDir::new(scan_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| is_ts_file(e.path()))
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

        extract_entities(graph, &rel_path, &source, &mut fn_bodies, &mut fn_signatures);
    }

    // Resolve call relations
    for (caller_id, body) in &fn_bodies {
        let calls = extract_calls(body);
        for call_name in calls {
            if let Some(target_id) = graph.find_by_name(&call_name) {
                graph.add_relation(*caller_id, target_id, RelationKind::Calls);
            }
        }
    }

    // Resolve type usage relations
    for (fn_id, type_names) in &fn_signatures {
        for type_name in type_names {
            if let Some(type_id) = graph.find_by_name(type_name) {
                graph.add_relation(*fn_id, type_id, RelationKind::UsesType);
            }
        }
    }
}

fn is_ts_file(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    matches!(ext, "ts" | "tsx" | "js" | "jsx")
        && !path.to_string_lossy().contains("node_modules")
        && !path.to_string_lossy().contains(".d.ts")
}

fn extract_entities(
    graph: &mut CodeGraph,
    file: &str,
    source: &str,
    fn_bodies: &mut Vec<(usize, String)>,
    fn_signatures: &mut Vec<(usize, Vec<String>)>,
) {
    let lines: Vec<&str> = source.lines().collect();
    let mut current_class: Option<String> = None;
    let mut brace_depth: i32 = 0;
    let mut class_brace_depth: Option<i32> = None;

    for (line_idx, line) in lines.iter().enumerate() {
        let line_num = line_idx + 1;
        let trimmed = line.trim();

        // Track brace depth
        for ch in trimmed.chars() {
            match ch {
                '{' => brace_depth += 1,
                '}' => {
                    brace_depth -= 1;
                    if let Some(cbd) = class_brace_depth {
                        if brace_depth < cbd {
                            current_class = None;
                            class_brace_depth = None;
                        }
                    }
                }
                _ => {}
            }
        }

        // Skip comments
        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
            continue;
        }

        // Class declaration
        if let Some(name) = extract_class_name(trimmed) {
            let end = find_block_end(&lines, line_idx);
            graph.add_entity(
                name.clone(),
                None,
                EntityKind::Struct, // Class maps to Struct
                file.to_string(),
                line_num,
                end,
            );
            current_class = Some(name);
            class_brace_depth = Some(brace_depth);
            continue;
        }

        // Interface declaration
        if let Some(name) = extract_interface_name(trimmed) {
            let end = find_block_end(&lines, line_idx);
            let id = graph.add_entity(
                name.clone(),
                None,
                EntityKind::Trait, // Interface maps to Trait
                file.to_string(),
                line_num,
                end,
            );

            // Extract extended interfaces
            let types = extract_extends_implements(trimmed);
            if !types.is_empty() {
                fn_signatures.push((id, types));
            }
            continue;
        }

        // Enum declaration
        if let Some(name) = extract_enum_name(trimmed) {
            let end = find_block_end(&lines, line_idx);
            graph.add_entity(
                name,
                None,
                EntityKind::Enum,
                file.to_string(),
                line_num,
                end,
            );
            continue;
        }

        // Type alias
        if let Some(name) = extract_type_alias(trimmed) {
            graph.add_entity(
                name,
                None,
                EntityKind::Struct, // Type alias maps to Struct
                file.to_string(),
                line_num,
                line_num,
            );
            continue;
        }

        // Function declaration (standalone or exported)
        if let Some(name) = extract_function_name(trimmed) {
            let end = find_block_end(&lines, line_idx);
            let body = extract_body(&lines, line_idx, end);
            let types = extract_ts_type_annotations(trimmed, &body);

            let id = graph.add_entity(
                name,
                current_class.clone(),
                EntityKind::Function,
                file.to_string(),
                line_num,
                end,
            );

            fn_bodies.push((id, body));
            if !types.is_empty() {
                fn_signatures.push((id, types));
            }

            // Link to containing class
            if let Some(ref class_name) = current_class {
                if let Some(class_id) = graph.find_by_name(class_name) {
                    graph.add_relation(class_id, id, RelationKind::Contains);
                }
            }
            continue;
        }

        // Arrow function assigned to const/let (exported or not)
        if let Some(name) = extract_arrow_function_name(trimmed) {
            let end = find_block_end(&lines, line_idx);
            let body = extract_body(&lines, line_idx, end);
            let types = extract_ts_type_annotations(trimmed, &body);

            let id = graph.add_entity(
                name,
                current_class.clone(),
                EntityKind::Function,
                file.to_string(),
                line_num,
                end,
            );

            fn_bodies.push((id, body));
            if !types.is_empty() {
                fn_signatures.push((id, types));
            }
            continue;
        }

        // Method in class (not caught by function_name due to no 'function' keyword)
        if current_class.is_some() {
            if let Some(name) = extract_method_name(trimmed) {
                let end = find_block_end(&lines, line_idx);
                let body = extract_body(&lines, line_idx, end);
                let types = extract_ts_type_annotations(trimmed, &body);

                let id = graph.add_entity(
                    name,
                    current_class.clone(),
                    EntityKind::Function,
                    file.to_string(),
                    line_num,
                    end,
                );

                fn_bodies.push((id, body));
                if !types.is_empty() {
                    fn_signatures.push((id, types));
                }

                if let Some(ref class_name) = current_class {
                    if let Some(class_id) = graph.find_by_name(class_name) {
                        graph.add_relation(class_id, id, RelationKind::Contains);
                    }
                }
            }
        }
    }
}

// --- Name extractors ---

fn extract_class_name(line: &str) -> Option<String> {
    let line = strip_export(line);
    let line = strip_modifiers(&line);
    if line.starts_with("class ") {
        let rest = &line[6..];
        let name = take_identifier(rest)?;
        if !name.is_empty() {
            return Some(name);
        }
    }
    None
}

fn extract_interface_name(line: &str) -> Option<String> {
    let line = strip_export(line);
    if line.starts_with("interface ") {
        let rest = &line[10..];
        let name = take_identifier(rest)?;
        if !name.is_empty() {
            return Some(name);
        }
    }
    None
}

fn extract_enum_name(line: &str) -> Option<String> {
    let line = strip_export(line);
    let line = strip_modifiers(&line);
    if line.starts_with("enum ") {
        let rest = &line[5..];
        let name = take_identifier(rest)?;
        if !name.is_empty() {
            return Some(name);
        }
    }
    None
}

fn extract_type_alias(line: &str) -> Option<String> {
    let line = strip_export(line);
    if line.starts_with("type ") {
        let rest = &line[5..];
        let name = take_identifier(rest)?;
        // Must have = after name (possibly with generics)
        if rest[name.len()..].trim_start().starts_with('<')
            || rest[name.len()..].trim_start().starts_with('=')
        {
            return Some(name);
        }
    }
    None
}

fn extract_function_name(line: &str) -> Option<String> {
    let line = strip_export(line);
    let line = strip_modifiers(&line);

    // async function name(...) or function name(...)
    let rest = if line.starts_with("async function ") {
        &line[15..]
    } else if line.starts_with("function ") {
        &line[9..]
    } else {
        return None;
    };

    // Skip generator star
    let rest = rest.strip_prefix('*').unwrap_or(rest);
    take_identifier(rest)
}

fn extract_arrow_function_name(line: &str) -> Option<String> {
    let line = strip_export(line);

    // const name = (...) => or const name = async (...) =>
    // const name: Type = (...) =>
    let rest = if line.starts_with("const ") {
        &line[6..]
    } else if line.starts_with("let ") {
        &line[4..]
    } else {
        return None;
    };

    let name = take_identifier(rest)?;
    let after = rest[name.len()..].trim_start();

    // Check for type annotation or direct assignment
    let after = if after.starts_with(':') {
        // Skip type annotation until =
        if let Some(eq_pos) = after.find('=') {
            &after[eq_pos..]
        } else {
            return None;
        }
    } else {
        after
    };

    if !after.starts_with('=') {
        return None;
    }
    let after = after[1..].trim_start();

    // Check for arrow function patterns
    if after.starts_with('(')
        || after.starts_with("async")
        || after.contains("=>")
    {
        Some(name)
    } else {
        None
    }
}

fn extract_method_name(line: &str) -> Option<String> {
    let line = strip_modifiers(line);

    // async methodName(...) { or methodName(...) {
    let rest = if line.starts_with("async ") {
        &line[6..]
    } else {
        &line
    };

    // Skip getter/setter
    let rest = if rest.starts_with("get ") || rest.starts_with("set ") {
        &rest[4..]
    } else {
        rest
    };

    let name = take_identifier(rest)?;
    if name.is_empty() {
        return None;
    }

    // Must have ( after name (possibly with generics)
    let after = rest[name.len()..].trim_start();
    if after.starts_with('(') || after.starts_with('<') {
        // Filter out control flow keywords
        let keywords = [
            "if", "else", "for", "while", "switch", "return", "case", "throw", "new", "delete",
            "typeof", "void", "import", "export", "from", "const", "let", "var", "class",
        ];
        if keywords.contains(&name.as_str()) {
            return None;
        }
        Some(name)
    } else {
        None
    }
}

fn extract_extends_implements(line: &str) -> Vec<String> {
    let mut types = Vec::new();
    for keyword in &["extends", "implements"] {
        if let Some(pos) = line.find(keyword) {
            let rest = &line[pos + keyword.len()..];
            for part in rest.split(',') {
                let part = part.trim().split('<').next().unwrap_or("").trim();
                let part = part.split('{').next().unwrap_or("").trim();
                if let Some(name) = take_identifier(part) {
                    types.push(name);
                }
            }
        }
    }
    types
}

// --- Helpers ---

fn strip_export(line: &str) -> &str {
    let line = line.trim_start();
    if line.starts_with("export default ") {
        return &line[15..];
    }
    if line.starts_with("export ") {
        return &line[7..];
    }
    line
}

fn strip_modifiers(line: &str) -> String {
    let modifiers = [
        "public ",
        "private ",
        "protected ",
        "static ",
        "abstract ",
        "readonly ",
        "override ",
        "declare ",
    ];
    let mut result = line.to_string();
    loop {
        let trimmed = result.trim_start();
        let mut found = false;
        for m in &modifiers {
            if trimmed.starts_with(m) {
                result = trimmed[m.len()..].to_string();
                found = true;
                break;
            }
        }
        if !found {
            break;
        }
    }
    result
}

fn take_identifier(s: &str) -> Option<String> {
    let s = s.trim_start();
    let mut name = String::new();
    for ch in s.chars() {
        if ch.is_alphanumeric() || ch == '_' || ch == '$' {
            name.push(ch);
        } else {
            break;
        }
    }
    if name.is_empty() || name.chars().next().map_or(true, |c| c.is_ascii_digit()) {
        None
    } else {
        Some(name)
    }
}

fn find_block_end(lines: &[&str], start_idx: usize) -> usize {
    let mut depth = 0i32;
    let mut found_open = false;
    for i in start_idx..lines.len() {
        for ch in lines[i].chars() {
            if ch == '{' {
                depth += 1;
                found_open = true;
            } else if ch == '}' {
                depth -= 1;
            }
        }
        if found_open && depth <= 0 {
            return i + 1;
        }
    }
    (start_idx + 10).min(lines.len())
}

fn extract_body(lines: &[&str], start_idx: usize, end_line: usize) -> String {
    let end_idx = (end_line).min(lines.len());
    lines[start_idx..end_idx].join("\n")
}

fn extract_ts_type_annotations(signature: &str, body: &str) -> Vec<String> {
    let mut types = Vec::new();
    let skip = [
        "string", "number", "boolean", "void", "any", "unknown", "never", "null", "undefined",
        "object", "symbol", "bigint", "true", "false", "Array", "Promise", "Record", "Partial",
        "Required", "Readonly", "Pick", "Omit", "Map", "Set", "Date", "Error", "RegExp",
        "Function", "Object", "String", "Number", "Boolean",
    ];

    // Scan for type annotations (: Type or <Type>)
    let combined = format!("{}\n{}", signature, body);
    let mut chars = combined.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == ':' || ch == '<' {
            // Grab the type name after colon/angle
            let mut name = String::new();
            for next in chars.by_ref() {
                if next.is_alphanumeric() || next == '_' {
                    name.push(next);
                } else if name.is_empty() && next.is_whitespace() {
                    continue;
                } else {
                    break;
                }
            }
            if !name.is_empty()
                && name.chars().next().map_or(false, |c| c.is_uppercase())
                && !skip.contains(&name.as_str())
            {
                types.push(name);
            }
        }
    }

    types.sort();
    types.dedup();
    types
}

fn extract_calls(body: &str) -> Vec<String> {
    let mut calls = Vec::new();
    let mut chars = body.chars().peekable();
    let mut current_ident = String::new();
    let mut in_string = false;
    let mut string_char = ' ';
    let mut in_template = false;
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

        if !in_string && !in_template && (ch == '\'' || ch == '"') {
            in_string = true;
            string_char = ch;
            prev_char = ch;
            continue;
        }
        if in_string && ch == string_char && prev_char != '\\' {
            in_string = false;
            prev_char = ch;
            continue;
        }
        if !in_string && ch == '`' {
            in_template = !in_template;
            prev_char = ch;
            continue;
        }

        if in_string || in_template {
            prev_char = ch;
            continue;
        }

        if ch.is_alphanumeric() || ch == '_' || ch == '$' {
            current_ident.push(ch);
        } else {
            if ch == '(' && !current_ident.is_empty() {
                let skip = [
                    "if", "else", "for", "while", "switch", "return", "case", "throw",
                    "new", "delete", "typeof", "void", "import", "export", "from",
                    "const", "let", "var", "function", "async", "await", "class",
                    "catch", "finally", "try", "require", "super", "this",
                    "console", "log", "warn", "error", "info", "debug",
                    "setTimeout", "setInterval", "clearTimeout", "clearInterval",
                    "parseInt", "parseFloat", "isNaN", "isFinite",
                    "JSON", "Math", "Object", "Array", "String", "Number", "Boolean",
                    "Promise", "Map", "Set", "Date", "RegExp", "Error",
                    "push", "pop", "shift", "unshift", "splice", "slice", "concat",
                    "map", "filter", "reduce", "forEach", "find", "findIndex", "some",
                    "every", "flat", "flatMap", "sort", "reverse", "includes",
                    "keys", "values", "entries", "from", "of", "assign", "freeze",
                    "then", "catch", "finally", "resolve", "reject", "all", "race",
                    "toString", "valueOf", "hasOwnProperty", "bind", "call", "apply",
                    "trim", "split", "join", "replace", "match", "test", "search",
                    "startsWith", "endsWith", "includes", "indexOf", "lastIndexOf",
                    "substring", "charAt", "charCodeAt", "toLowerCase", "toUpperCase",
                    "addEventListener", "removeEventListener", "createElement",
                    "getElementById", "querySelector", "querySelectorAll",
                    "getAttribute", "setAttribute", "appendChild", "removeChild",
                ];
                if !skip.contains(&current_ident.as_str())
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
