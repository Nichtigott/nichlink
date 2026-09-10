//! Search query methods owned by App.
//! App 所有的搜索查询方法。

use super::*;

impl App {
    /// Folded-set filtering is not wired in yet; the parameter is accepted to
    /// keep the call sites stable.
    /// 折叠集合过滤尚未接入；为保持调用点稳定先保留该参数。
    pub fn search_rows(&self, query: &str, folded: &BTreeSet<usize>) -> Vec<SearchRow> {
        let mut rows = self.source_symbol_rows(query);
        let _ = folded;
        rows.dedup_by(|left, right| {
            left.node == right.node && left.function == right.function && left.line == right.line
        });
        rows
    }

    /// Add compact source symbols when a file or face is searched.
    /// 搜索文件或注册面时，补充紧凑的源代码符号项。
    fn source_symbol_rows(&self, query: &str) -> Vec<SearchRow> {
        let needle = query.trim().to_ascii_lowercase();
        if needle.is_empty() {
            return Vec::new();
        }
        let mut rows = Vec::new();
        for info in self.registry.depth_first() {
            let path = self.registry.path_for(info.id).unwrap_or_default();
            let source = source_path_for(&info.source.file);
            let file_match = info.source.file.to_ascii_lowercase().contains(&needle)
                || path.to_ascii_lowercase().contains(&needle)
                || info.registry_name.to_ascii_lowercase().contains(&needle)
                || info.kind.to_ascii_lowercase().contains(&needle);
            let Ok(text) = std::fs::read_to_string(source) else {
                continue;
            };
            if file_match {
                rows.push(SearchRow {
                    source_index: usize::MAX - rows.len(),
                    depth: 0,
                    has_children: false,
                    node: Some(info.id),
                    path: info.source.file.to_owned(),
                    function: String::new(),
                    line: Some(1),
                    signature: String::new(),
                    text: info.source.file.to_owned(),
                });
            }
            let functions = function_symbols(&text);
            for function in &functions {
                if !file_match
                    && !info.source.function.to_ascii_lowercase().contains(&needle)
                    && !function.name.to_ascii_lowercase().contains(&needle)
                    && !function.signature.to_ascii_lowercase().contains(&needle)
                {
                    continue;
                }
                rows.push(SearchRow {
                    source_index: usize::MAX - rows.len(),
                    depth: 1,
                    has_children: false,
                    node: Some(info.id),
                    path: info.source.file.to_owned(),
                    function: function.name.clone(),
                    line: Some(function.line),
                    signature: function.signature.clone(),
                    text: format!("{} -> fn {}", info.source.file, function.name),
                });
            }
            for (line_index, line) in text.lines().enumerate() {
                if functions
                    .iter()
                    .any(|function| function.line == line_index as u32 + 1)
                {
                    continue;
                }
                let trimmed = line.trim();
                let symbol = trimmed
                    .strip_prefix("pub fn ")
                    .or_else(|| trimmed.strip_prefix("fn "))
                    .or_else(|| trimmed.strip_prefix("pub async fn "))
                    .or_else(|| trimmed.strip_prefix("async fn "))
                    .or_else(|| trimmed.strip_prefix("pub struct "))
                    .or_else(|| trimmed.strip_prefix("struct "))
                    .or_else(|| trimmed.strip_prefix("pub enum "))
                    .or_else(|| trimmed.strip_prefix("enum "))
                    .or_else(|| trimmed.strip_prefix("pub type "))
                    .or_else(|| trimmed.strip_prefix("type "))
                    .or_else(|| trimmed.strip_prefix("pub const "))
                    .or_else(|| trimmed.strip_prefix("const "))
                    .or_else(|| trimmed.strip_prefix("let "));
                let Some(symbol) = symbol else {
                    continue;
                };
                let name = symbol
                    .split(|character: char| {
                        character == '('
                            || character == '{'
                            || character == ':'
                            || character == '='
                            || character.is_whitespace()
                    })
                    .next()
                    .unwrap_or(symbol)
                    .trim_matches([';', ',']);
                if !file_match
                    && !info.source.function.to_ascii_lowercase().contains(&needle)
                    && !name.to_ascii_lowercase().contains(&needle)
                    && !trimmed.to_ascii_lowercase().contains(&needle)
                {
                    continue;
                }
                rows.push(SearchRow {
                    source_index: usize::MAX - rows.len(),
                    depth: 1,
                    has_children: false,
                    node: Some(info.id),
                    path: info.source.file.to_owned(),
                    function: name.to_owned(),
                    line: Some(line_index as u32 + 1),
                    signature: trimmed.to_owned(),
                    text: if trimmed.starts_with("fn ")
                        || trimmed.starts_with("pub fn ")
                        || trimmed.contains(" fn ")
                    {
                        format!("{} -> fn {name}", info.source.file)
                    } else {
                        format!("{} -> {name}", info.source.file)
                    },
                });
            }
        }
        rows
    }

    pub(super) fn source_function_line(&self, node: NodeId, function: &str) -> Option<u32> {
        let info = self.registry.find(node)?;
        function_line(&info.source.file, function)
    }
}
