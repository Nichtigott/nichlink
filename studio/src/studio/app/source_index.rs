//! Source indexing and registry snapshot helpers for Studio.
//! Studio 的源码索引与注册快照辅助逻辑。

use super::support::{package_namespace, package_root, with_authoring_context};
use super::*;

pub(crate) fn function_source_range(lines: &[&str], name: &str) -> Option<(usize, usize)> {
    let start = lines.iter().position(|line| {
        let trimmed = line.trim_start();
        trimmed.contains("fn ") && trimmed.contains(&format!("{name}("))
    })?;
    let mut depth = 0usize;
    let mut opened = false;
    for (index, line) in lines.iter().enumerate().skip(start) {
        for character in line.chars() {
            match character {
                '{' => {
                    depth += 1;
                    opened = true;
                }
                '}' if opened => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
        if opened && depth == 0 {
            return Some((start, index));
        }
    }
    Some((start, start))
}

/// Compose compiled, external, and newly authored faces into one snapshot.
/// 将已编译、外部和刚落盘的注册面装配成同一个快照。
pub(super) fn load_registry() -> Result<Registry, String> {
    // Studio owns an empty, namespace-isolated root and lets the authoring UI
    // add faces. A host can choose a stable namespace per library through the
    // environment when several libraries share one process.
    let namespace = package_namespace();
    let mut registry =
        Registry::root_for_namespace(nichlink::FrameworkId::new("nichlink.studio"), namespace);
    // Scan only the host package's `src/` tree. When Studio is launched from
    // the NichLink workspace itself there is no host `src/`; show an empty
    // registry instead of mistaking build/debug fixtures for faces.
    // 只扫描宿主包的 `src/`。直接从 NichLink workspace 启动时没有宿主 `src/`，
    // 此时显示空注册树，不要把 build/debug 测试夹具误认成注册面。
    let source_root = package_root().join("src");
    if !source_root.is_dir() {
        return Ok(registry);
    }
    let snapshots = with_authoring_context(|| nichlink::generated_snapshots_from(&source_root))
        .map_err(|error| format!("cannot load generated registration faces: {error}"))?;
    registry
        .register_snapshot_batch(snapshots)
        .map_err(|error| format!("generated registration faces were rejected: {error}"))?;
    Ok(registry)
}

pub(super) fn admission_text(admission: &nichlink::OwnedAdmission) -> String {
    if admission.allowed_paths.is_empty() && admission.denied_paths.is_empty() {
        return "ANY".to_owned();
    }
    if !admission.allowed_paths.is_empty() {
        return format!("allow:{}", admission.allowed_paths.join(","));
    }
    format!("deny:{}", admission.denied_paths.join(","))
}

pub(super) fn registration_rule_text(rule: &nichlink::OwnedRegistrationRule) -> String {
    let mut clauses = Vec::new();
    if let Some(preset) = &rule.required_preset {
        clauses.push(format!("preset:{preset}"));
    }
    if !rule.required_parts.is_empty() {
        clauses.push(format!("parts:{}", rule.required_parts.join(",")));
    }
    if !rule.required_exports.is_empty() {
        clauses.push(format!("exports:{}", rule.required_exports.join(",")));
    }
    if !rule.required_handle_traits.is_empty() {
        clauses.push(format!("handle:{}", rule.required_handle_traits.join(",")));
    }
    if !rule.required_part_traits.is_empty() {
        clauses.push(format!(
            "part_trait:{}",
            rule.required_part_traits.join(",")
        ));
    }
    if clauses.is_empty() {
        "ANY".to_owned()
    } else {
        clauses.join(";")
    }
}

#[cfg(test)]
pub(super) fn visible_search_rows(lines: &[String], folded: &BTreeSet<usize>) -> Vec<SearchRow> {
    let depths = lines
        .iter()
        .map(|line| {
            line.chars()
                .take_while(|character| character.is_whitespace())
                .count()
                / 2
        })
        .collect::<Vec<_>>();
    let mut folded_ancestors = Vec::new();
    let mut rows = Vec::new();
    for (source_index, text) in lines.iter().enumerate() {
        if text.trim().is_empty() {
            folded_ancestors.clear();
            continue;
        }
        let depth = depths[source_index];
        folded_ancestors.retain(|ancestor_depth| *ancestor_depth < depth);
        let hidden = !folded_ancestors.is_empty();
        let has_children = lines
            .get(source_index + 1)
            .is_some_and(|next| !next.trim().is_empty() && depths[source_index + 1] > depth);
        if !hidden {
            let (node, path, function, line, display) = parse_search_line(text);
            rows.push(SearchRow {
                source_index,
                depth,
                has_children,
                node,
                path,
                function,
                line,
                signature: String::new(),
                text: display,
            });
        }
        if folded.contains(&source_index) && has_children {
            folded_ancestors.push(depth);
        }
    }
    rows
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SourceFunction {
    pub(super) name: String,
    pub(super) signature: String,
    pub(super) body: String,
    pub(super) line: u32,
}

/// Index function bodies without treating comments, strings, or macro text as Rust.
/// 扫描函数体时屏蔽注释、字符串和宏文本，避免把它们误认成 Rust 函数。
pub(super) fn function_symbols(source: &str) -> Vec<SourceFunction> {
    let masked = mask_non_code(source);
    let bytes = masked.as_bytes();
    let mut result = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
        if !is_ident_start(bytes[index]) {
            index += 1;
            continue;
        }
        let token_start = index;
        index += 1;
        while index < bytes.len() && is_ident_continue(bytes[index]) {
            index += 1;
        }
        if &masked[token_start..index] != "fn" {
            continue;
        }
        let mut name_start = index;
        while name_start < bytes.len() && bytes[name_start].is_ascii_whitespace() {
            name_start += 1;
        }
        if name_start >= bytes.len() || !is_ident_start(bytes[name_start]) {
            continue;
        }
        let mut name_end = name_start + 1;
        while name_end < bytes.len() && is_ident_continue(bytes[name_end]) {
            name_end += 1;
        }
        let name = masked[name_start..name_end].to_owned();
        let mut open = name_end;
        let mut angle_depth = 0usize;
        while open < bytes.len() {
            match bytes[open] {
                b'<' => angle_depth += 1,
                b'>' if angle_depth > 0 => angle_depth -= 1,
                b'{' if angle_depth == 0 => break,
                b';' if angle_depth == 0 => break,
                _ => {}
            }
            open += 1;
        }
        if open >= bytes.len() || bytes[open] != b'{' {
            continue;
        }
        let mut depth = 1usize;
        let mut close = open + 1;
        while close < bytes.len() && depth > 0 {
            match bytes[close] {
                b'{' => depth += 1,
                b'}' => depth = depth.saturating_sub(1),
                _ => {}
            }
            close += 1;
        }
        if depth != 0 {
            continue;
        }
        let line_start = source[..token_start].rfind('\n').map_or(0, |line| line + 1);
        let line = source[..token_start]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count() as u32
            + 1;
        let signature = source[line_start..open].trim().to_owned();
        result.push(SourceFunction {
            name,
            signature,
            body: source[open + 1..close - 1].to_owned(),
            line,
        });
        index = close;
    }
    result
}

#[cfg(test)]
pub(super) fn function_bodies(source: &str) -> Vec<(String, String)> {
    function_symbols(source)
        .into_iter()
        .map(|function| (function.name, function.body))
        .collect()
}

fn is_ident_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_ident_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Replace comments and quoted literals with spaces while preserving offsets.
/// 用空格替换注释和引号字面量，同时保留原始偏移量。
fn mask_non_code(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut masked = bytes.to_vec();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'/') {
            index += 2;
            while index < bytes.len() && bytes[index] != b'\n' {
                masked[index] = b' ';
                index += 1;
            }
            continue;
        }
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
            masked[index] = b' ';
            if index + 1 < bytes.len() {
                masked[index + 1] = b' ';
            }
            index += 2;
            let mut depth = 1usize;
            while index < bytes.len() && depth > 0 {
                if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
                    depth += 1;
                    masked[index] = b' ';
                    masked[index + 1] = b' ';
                    index += 2;
                } else if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') {
                    depth = depth.saturating_sub(1);
                    masked[index] = b' ';
                    masked[index + 1] = b' ';
                    index += 2;
                } else {
                    if bytes[index] != b'\n' {
                        masked[index] = b' ';
                    }
                    index += 1;
                }
            }
            continue;
        }
        if bytes[index] == b'"' || bytes[index] == b'\'' {
            let quote = bytes[index];
            masked[index] = b' ';
            index += 1;
            while index < bytes.len() {
                let escaped = bytes[index] == b'\\';
                if bytes[index] != b'\n' {
                    masked[index] = b' ';
                }
                index += 1;
                if escaped && index < bytes.len() {
                    if bytes[index] != b'\n' {
                        masked[index] = b' ';
                    }
                    index += 1;
                } else if bytes[index - 1] == quote {
                    break;
                }
            }
            continue;
        }
        index += 1;
    }
    String::from_utf8(masked).unwrap_or_else(|_| source.to_owned())
}

/// Match a real function call in a body, ignoring comments, strings and macros.
/// 只匹配函数体里的真实调用，跳过注释、字符串和宏调用。
pub(super) fn body_calls(body: &str, target: &str) -> bool {
    if target.is_empty() {
        return false;
    }
    let bytes = body.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'/') {
            index += 2;
            while index < bytes.len() && bytes[index] != b'\n' {
                index += 1;
            }
            continue;
        }
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
            index += 2;
            while index + 1 < bytes.len() && !(bytes[index] == b'*' && bytes[index + 1] == b'/') {
                index += 1;
            }
            index = (index + 2).min(bytes.len());
            continue;
        }
        if bytes[index] == b'"' {
            index += 1;
            while index < bytes.len() {
                if bytes[index] == b'\\' {
                    index = (index + 2).min(bytes.len());
                    continue;
                }
                let end = bytes[index] == b'"';
                index += 1;
                if end {
                    break;
                }
            }
            continue;
        }
        let is_start = bytes[index].is_ascii_alphabetic() || bytes[index] == b'_';
        if !is_start {
            index += 1;
            continue;
        }
        let start = index;
        index += 1;
        while index < bytes.len() && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
        {
            index += 1;
        }
        let mut lookahead = index;
        while lookahead < bytes.len() && bytes[lookahead].is_ascii_whitespace() {
            lookahead += 1;
        }
        if bytes.get(lookahead) == Some(&b'!') {
            let macro_name = &body[start..index];
            if matches!(macro_name, "control_object" | "external_object") {
                index = lookahead + 1;
                if let Some(open) = bytes.get(index).copied() {
                    let close = match open {
                        b'(' => Some(b')'),
                        b'[' => Some(b']'),
                        b'{' => Some(b'}'),
                        _ => None,
                    };
                    if let Some(close) = close {
                        let mut depth = 0usize;
                        while index < bytes.len() {
                            if bytes[index] == open {
                                depth += 1;
                            }
                            if bytes[index] == close {
                                depth = depth.saturating_sub(1);
                                if depth == 0 {
                                    index += 1;
                                    break;
                                }
                            }
                            index += 1;
                        }
                    }
                }
                continue;
            }
            // Keep scanning ordinary macros: closures passed to tracing or
            // instrumentation macros still contain real function calls.
            // 普通宏内部继续扫描；追踪宏闭包里的调用仍是真实调用。
            index = lookahead + 1;
            continue;
        }
        if &body[start..index] != target {
            continue;
        }
        while lookahead < bytes.len() && bytes[lookahead].is_ascii_whitespace() {
            lookahead += 1;
        }
        if bytes.get(lookahead) == Some(&b':') && bytes.get(lookahead + 1) == Some(&b':') {
            lookahead += 2;
            while lookahead < bytes.len() && bytes[lookahead].is_ascii_whitespace() {
                lookahead += 1;
            }
            if bytes.get(lookahead) == Some(&b'<') {
                let mut angle_depth = 0usize;
                while lookahead < bytes.len() {
                    match bytes[lookahead] {
                        b'<' => angle_depth += 1,
                        b'>' if angle_depth > 0 => {
                            angle_depth -= 1;
                            if angle_depth == 0 {
                                lookahead += 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                    lookahead += 1;
                }
                while lookahead < bytes.len() && bytes[lookahead].is_ascii_whitespace() {
                    lookahead += 1;
                }
            }
        }
        if bytes.get(lookahead) == Some(&b'(') {
            return true;
        }
    }
    false
}

/// Turn the verbose headless report into a human-readable search row.
/// 将详细的无终端报告转换为人能快速扫描的搜索行。
#[cfg(test)]
fn parse_search_line(line: &str) -> (Option<NodeId>, String, String, Option<u32>, String) {
    let trimmed = line.trim();
    let mut tokens = trimmed.split_whitespace();
    let _branch = tokens.next();
    let node = tokens.next().and_then(|value| value.parse().ok());
    let remainder = tokens.collect::<Vec<_>>().join(" ");
    let (target, declaration) = remainder
        .split_once(" declared-at=")
        .map_or((remainder.as_str(), ""), |(target, rest)| {
            (target, rest.split_whitespace().next().unwrap_or(""))
        });
    let line = declaration
        .rsplit_once(':')
        .and_then(|(_, line)| line.parse::<u32>().ok());
    let (path, function) = target
        .split_once("::")
        .map(|(path, function)| (path.to_owned(), function.to_owned()))
        .unwrap_or_else(|| (target.to_owned(), String::new()));
    let display = if function.is_empty() {
        path.clone()
    } else {
        format!("{path} -> fn {function}")
    };
    (node, path, function, line, display)
}
