//! Search query methods owned by App.
//! App 所有的搜索查询方法。

use super::*;

use nichlink_run_method::registry_core::declaration::source_file_matches;

impl App {
    /// Flat, deduplicated rows for one search query.
    /// 一次搜索查询的扁平、已去重结果行。
    ///
    /// The list is intentionally flat: there is no hierarchy to fold, so the
    /// caller has no fold state and `Enter` promotes a row straight into the
    /// call graph. The fold keys and `▸/▾` markers that used to be advertised
    /// were removed rather than implemented.
    /// 结果是刻意扁平的：没有可折叠的层级，因此调用方不持有折叠状态，`Enter`
    /// 直接把选中行推进调用图。此前展示的折叠键与 `▸/▾` 标记已删除，而非实现。
    pub fn search_rows(&self, query: &str) -> Vec<SearchRow> {
        let mut rows = self.source_symbol_rows(query);
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
            let file_match = source_file_matches(&info.source.file, &needle)
                || path.to_ascii_lowercase().contains(&needle)
                || info.registry_name.to_ascii_lowercase().contains(&needle)
                || info.kind.to_ascii_lowercase().contains(&needle);
            let Ok(text) = std::fs::read_to_string(source) else {
                continue;
            };
            if file_match {
                rows.push(SearchRow {
                    depth: 0,
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
                    depth: 1,
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
                    depth: 1,
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
