//! Source locations, their portable text form, and localized labels.
//! 源码位置、其可移植文本形式，以及本地化标签。

use super::*;

/// File and line captured at the declaration site.
/// 在声明点捕获的文件和行号。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceLocation {
    /// Source file path as recorded by `file!()` at the declaration site.
    /// 声明点由 `file!()` 记录的源码文件路径。
    pub file: &'static str,
    /// Line of the declaration site, as reported by `line!()`.
    /// 声明点所在行号，由 `line!()` 报告。
    pub line: u32,
    /// Column of the declaration site, as reported by `column!()`.
    /// 声明点所在列号，由 `column!()` 报告。
    pub column: u32,
    /// The registration-face handle or logical function associated with this declaration.
    /// 与该注册面声明关联的 handle 或逻辑函数名。
    pub function: &'static str,
}

/// Render a source path with `/` separators on every platform.
/// 在任何平台上都以 `/` 分隔符渲染源码路径。
///
/// `file!()` records the platform separator, and a declaration cannot rewrite
/// it at compile time without allocating, so the portable form is produced when
/// a path is displayed or compared instead. Identity is unaffected: hashing
/// folds both separators to the same byte.
/// `file!()` 记录平台分隔符，而声明在编译期无法在不分配的前提下改写它，因此改为
/// 在显示或比较时产出可移植形式。身份不受影响：哈希会把两种分隔符折叠为同一字节。
pub fn portable_path(file: &str) -> String {
    if file.contains('\\') {
        file.replace('\\', "/")
    } else {
        file.to_owned()
    }
}

/// Whether a recorded source path matches a lowercase search needle.
/// 记录的源码路径是否匹配一个小写的搜索关键词。
///
/// The search surfaces match a `file!()`-recorded path against what a user
/// typed, and on Windows that path is spelled with backslashes while the path
/// the same surface *displays* is [`portable_path`]'s `/` form. Comparing the
/// raw value therefore misses the query a user can see and retype — the search
/// for `control/widget.rs` finds nothing where the screen says
/// `control/widget.rs`. The fold has to happen here, at the comparison, for the
/// same reason it happens at the display.
/// 搜索面把 `file!()` 记录的路径与用户输入做匹配，而在 Windows 上该路径以反斜杠拼写，
/// 同一个面**显示**的却是 [`portable_path`] 的 `/` 形式。因此比较原始值时，用户
/// 照着屏幕重敲的查询会落空——搜 `control/widget.rs` 什么都找不到，而屏幕上写的就是
/// `control/widget.rs`。与显示时同理，折叠必须发生在比较处。
///
/// `needle` must already be lowercase: every caller lowercases its query once and
/// reuses it across all the fields it searches, and folding case here would
/// allocate a second time on every candidate.
/// `needle` 必须已经是小写：每个调用方把查询小写一次并在它搜索的所有字段间复用，在这里
/// 折叠大小写会让每个候选再多分配一次。
pub fn source_file_matches(file: &str, needle: &str) -> bool {
    if file.contains('\\') {
        portable_path(file).to_ascii_lowercase().contains(needle)
    } else {
        file.to_ascii_lowercase().contains(needle)
    }
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}:{}:{}",
            portable_path(self.file),
            self.line,
            self.column
        )
    }
}

impl SourceLocation {
    /// Whether this location was synthesized by a static analyzer.
    /// 该位置是否由静态分析器合成。
    pub fn is_synthetic(self) -> bool {
        self.file.starts_with('<') && self.file.ends_with('>')
    }

    /// Render the source location with the branch-local function name.
    /// 渲染带分支函数名的声明位置。
    pub fn describe(self) -> String {
        format!(
            "{}:{}:{} function={}",
            portable_path(self.file),
            self.line,
            self.column,
            self.function
        )
    }
}

/// Bilingual text kept on the registration face.
/// 注册面上的双语文本。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalizedText {
    /// Chinese text shown by authoring and diagnostic surfaces.
    /// 创作与诊断界面显示的中文文本。
    pub zh: &'static str,
    /// English text shown by authoring and diagnostic surfaces.
    /// 创作与诊断界面显示的英文文本。
    pub en: &'static str,
}

#[cfg(test)]
#[path = "source_location_tests.rs"]
mod source_location_tests;
