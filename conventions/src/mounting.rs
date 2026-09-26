//! The module-mounting gate: no `mod.rs`, and exactly one `include!`.
//! 模块挂载门禁：没有 `mod.rs`，且只有一个 `include!`。
//!
//! Why these two and not the whole convention: the workspace mounts module files
//! with `#[path = "<dir>/<name>.rs"] pub mod <name>;`, and the reason is
//! resolution, not style. A `#[path]`-loaded module resolves its own children
//! relative to its directory (the behaviour `mod.rs` would give), which is what
//! makes the `<dir>/<name>.rs` layout work without `mod.rs`. A bare `mod x;` is
//! equally correct when the parent is a crate root or is itself `#[path]`-loaded,
//! which is why 29 such declarations exist and why gating them would mean
//! churning five crates for no failure mode.
//! 为什么只查这两条而不是整条约定：工作区用 `#[path = "<dir>/<name>.rs"] pub mod <name>;`
//! 挂载模块文件，原因是解析而不是风格。经 `#[path]` 载入的模块会以所在目录为基准解析自己的
//! 子模块（也就是 `mod.rs` 能给出的行为），这正是 `<dir>/<name>.rs` 布局无需 `mod.rs` 的
//! 原因。当父文件是 crate 根或本身也是 `#[path]` 载入时，裸 `mod x;` 同样正确——这就是那
//! 29 处声明的由来，也是为什么对它们设门禁只会为五个 crate 带来没有故障模式的改动。
//!
//! `include!` is different in kind: it is a text splice, so `file!()` inside the
//! spliced file reports the *including* file. A registration face mounted that
//! way would silently change its own `NodeId`, and because identities are written
//! into on-disk graft records the damage would appear later as unresolved
//! selectors rather than as a build failure. That is a hidden-behaviour risk, so
//! it is gated.
//! `include!` 性质不同：它是文本拼接，因此被拼入文件里的 `file!()` 报告的是**引入方**文件。
//! 以这种方式挂载的注册面会静默改变自己的 `NodeId`；又因为身份会写入落盘的 graft 记录，损害
//! 会在以后表现为未解析的选择器，而不是构建失败。这是隐性行为风险，因此设门禁。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::{crate_directories, is_real_directory, relative, rust_sources};

/// What the mounting walk found.
/// 挂载遍历的结果。
#[derive(Debug, Default)]
pub struct Findings {
    /// Every `mod.rs`, which the workspace does not use.
    /// 每一处 `mod.rs`，工作区不使用它。
    pub mod_rs: Vec<String>,
    /// Every `include!(...)` at statement position, with file and line.
    /// 每一处位于语句位置的 `include!(...)`，含文件与行号。
    pub includes: Vec<String>,
}

/// What the kernel's own module tree looks like when it is walked as a tree.
/// 把内核自己的模块树当作树来遍历时看到的样子。
///
/// `AGENTS.md` change rule 1 says new pure logic is registered in
/// `core/src/registry_core.rs`, and nothing checked it: a module file that no
/// declaration names is still in the package, still passes `cargo package
/// --list` (the package audit's content half), and is simply never compiled.
/// The kernel is the scope because it mounts every module by hand; the example
/// hosts mount faces from `host!()`'s generated plan, which no declaration in
/// the tree names, so the same walk would report their faces as unmounted.
/// `AGENTS.md` 改动规则 1 说新的纯逻辑注册在 `core/src/registry_core.rs`，而此前无人检查：
/// 没有任何声明指名的模块文件仍在包里、仍能通过 `cargo package --list`（包审计的内容那一半），
/// 只是从未被编译。范围限于内核，因为内核的每个模块都靠手写声明挂载；示例宿主从 `host!()` 的
/// 生成计划挂载注册面，树里没有任何声明为它们命名，同样的遍历会把那些注册面报成未挂载。
#[derive(Debug, Default)]
pub struct Mounts {
    /// Kernel module files no declaration resolves to.
    /// 没有任何声明指向的内核模块文件。
    pub unmounted: Vec<String>,
    /// Declarations whose `#[path]`/name resolves to no file.
    /// `#[path]`/名字解析不到文件的声明。
    pub dangling: Vec<String>,
    /// Files named by more than one declaration.
    /// 被多于一个声明指名的文件。
    pub duplicated: Vec<String>,
}

/// One `mod` declaration: its name and its `#[path]`, when it has one.
/// 一条 `mod` 声明：名字，以及可选的 `#[path]`。
struct Declaration {
    name: String,
    path: Option<String>,
}

/// Module declarations in one file, in order.
/// 单个文件里的模块声明，按顺序。
///
/// The `mod` keyword and the attribute shape are read from `masked`, so a fixture
/// string cannot fake a declaration; the `#[path]` *value* is read from `raw`,
/// because masking blanks string literals and a blanked path is no path at all.
/// `mod` 关键字与属性形状读自 `masked`，因此夹具字符串伪造不出声明；`#[path]` 的**取值**读自
/// `raw`，因为屏蔽会把字符串字面量抹白，而被抹白的路径根本不是路径。
fn declarations(raw: &str, masked: &str) -> Vec<Declaration> {
    let bytes = masked.as_bytes();
    let mut found = Vec::new();
    let mut from = 0usize;
    while let Some(offset) = masked[from..].find("mod") {
        let at = from + offset;
        from = at + 3;
        // An identifier character before `mod` means it is part of a longer name.
        // `mod` 之前紧邻标识符字符意味着它是更长名字的一部分。
        if at > 0 && (bytes[at - 1].is_ascii_alphanumeric() || bytes[at - 1] == b'_') {
            continue;
        }
        let after_keyword = &masked[at + 3..];
        let rest = after_keyword.trim_start();
        let name_len = rest
            .find(|character: char| !(character.is_alphanumeric() || character == '_'))
            .unwrap_or(rest.len());
        if name_len == 0 {
            continue;
        }
        let name = &rest[..name_len];
        if !rest[name_len..].trim_start().starts_with(';') {
            continue;
        }
        // Resume *after* the name, or the next search finds the `mod` inside it:
        // `mod model;` used to yield a second declaration named `el`.
        // 从名字**之后**继续，否则下一次搜索会在名字里找到 `mod`：`mod model;` 过去会再产出一条
        // 名叫 `el` 的声明。
        from = at + 3 + (after_keyword.len() - rest.len()) + name_len;
        found.push(Declaration {
            name: name.to_owned(),
            path: path_attribute_before(raw, masked, at),
        });
    }
    found
}

/// The value of a `#[path = "…"]` attribute immediately before `at`, if any.
/// 紧邻 `at` 之前的 `#[path = "…"]` 属性的值（如果有）。
///
/// The walk crosses whatever sits between the attribute and the `mod` keyword: a
/// visibility qualifier (`pub`, `pub(crate)`), other attributes (`#[cfg(test)]`),
/// and whitespace. Stopping at the first of those was measured to hide every
/// `#[path]` in `core/src/registry_core.rs`, whose declarations are all spelled
/// `#[path = …]` newline `pub mod …;`.
/// 遍历会穿过属性与 `mod` 关键字之间的任何东西：可见性限定（`pub`、`pub(crate)`）、其他属性
/// （`#[cfg(test)]`）与空白。停在其中任何一个之前，实测会漏掉 `core/src/registry_core.rs` 里的
/// 每一个 `#[path]`——那里的声明全都写成 `#[path = …]` 换行 `pub mod …;`。
fn path_attribute_before(raw: &str, masked: &str, at: usize) -> Option<String> {
    let bytes = masked.as_bytes();
    let mut index = at;
    loop {
        while index > 0 && bytes[index - 1].is_ascii_whitespace() {
            index -= 1;
        }
        if index == 0 {
            return None;
        }
        if let Some(before) = visibility_start(masked, index) {
            index = before;
            continue;
        }
        if bytes[index - 1] != b']' {
            return None;
        }
        let mut depth = 0usize;
        let mut start = index;
        while start > 0 {
            start -= 1;
            match bytes[start] {
                b']' => depth += 1,
                b'[' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
        }
        if start == 0 || bytes[start - 1] != b'#' {
            return None;
        }
        let attribute = &raw[start..index];
        if let Some(value) = attribute_path(attribute) {
            return Some(value.to_owned());
        }
        index = start - 1;
    }
}

/// Where a visibility qualifier ending at `index` starts, if one does.
/// 结束于 `index` 的可见性限定符的起始位置（如果存在）。
fn visibility_start(masked: &str, index: usize) -> Option<usize> {
    let trimmed = masked[..index].trim_end();
    if let Some(head) = trimmed.strip_suffix("pub") {
        return identifier_boundary(head).then_some(head.len());
    }
    let open = matching_open_paren(trimmed, trimmed.len().checked_sub(1)?)?;
    let head = trimmed[..open].trim_end().strip_suffix("pub")?;
    identifier_boundary(head).then_some(head.len())
}

/// Whether the text ends at an identifier boundary.
/// 文本是否结束在标识符边界上。
fn identifier_boundary(head: &str) -> bool {
    head.chars()
        .next_back()
        .is_none_or(|previous| !(previous.is_alphanumeric() || previous == '_'))
}

/// The index of the `(` matching the `)` at `close`, if there is one.
/// `close` 处的 `)` 所配对的 `(` 的下标（如果存在）。
fn matching_open_paren(masked: &str, close: usize) -> Option<usize> {
    let bytes = masked.as_bytes();
    if bytes.get(close) != Some(&b')') {
        return None;
    }
    let mut depth = 0usize;
    let mut index = close + 1;
    while index > 0 {
        index -= 1;
        match bytes[index] {
            b')' => depth += 1,
            b'(' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

/// The string in one `path = "…"` attribute body.
/// 一个 `path = "…"` 属性体里的字符串。
fn attribute_path(attribute: &str) -> Option<&str> {
    let rest = attribute.split_once("path")?.1.trim_start();
    let rest = rest.strip_prefix('=')?.trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(&rest[..end])
}

/// The file one declaration mounts, relative to the declaring file's directory.
/// 一条声明所挂载的文件，以声明文件所在目录为基准。
fn target_of(declaration: &Declaration, directory: &Path) -> PathBuf {
    if let Some(path) = &declaration.path {
        return directory.join(path);
    }
    let nested = directory
        .join(&declaration.name)
        .join(format!("{}.rs", declaration.name));
    if nested.is_file() {
        return nested;
    }
    directory.join(format!("{}.rs", declaration.name))
}

/// Walk the kernel's module tree and report what does not add up.
/// 遍历内核的模块树，报告对不上的地方。
pub fn mounts(root: &Path) -> Mounts {
    let mut found = Mounts::default();
    let kernel = root.join("core/src");
    if !is_real_directory(&kernel) {
        return found;
    }
    let files = rust_sources(&kernel);
    let mut mounted: BTreeMap<PathBuf, Vec<String>> = BTreeMap::new();
    for path in &files {
        let text = std::fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let masked = nichlink::source::mask_non_code(&text);
        let directory = path.parent().unwrap_or(&kernel);
        for declaration in declarations(&text, &masked) {
            mounted
                .entry(target_of(&declaration, directory))
                .or_default()
                .push(relative(root, path));
        }
    }
    for path in &files {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if name == "lib.rs" || name == "build.rs" {
            continue;
        }
        if !mounted.contains_key(path) {
            found.unmounted.push(relative(root, path));
        }
    }
    for (target, declarers) in &mounted {
        if !target.is_file() {
            found.dangling.push(format!(
                "{} (declared by {})",
                relative(root, target),
                declarers.join(", ")
            ));
        } else if declarers.len() > 1 {
            found.duplicated.push(format!(
                "{} (declared by {})",
                relative(root, target),
                declarers.join(", ")
            ));
        }
    }
    found
}

/// Walk the workspace for mounting violations.
/// 遍历工作区，找出挂载违规。
pub fn findings(root: &Path) -> Findings {
    let mut found = Findings::default();
    for directory in crate_directories(root) {
        for path in rust_sources(&directory) {
            if path.file_name().is_some_and(|name| name == "mod.rs") {
                found.mod_rs.push(relative(root, &path));
            }
            // The splice search runs on the whole masked file, not line by line, and
            // accepts any run of spaces between the macro name and its `!`. Three
            // spellings were measured green against the line-based literal search:
            // `include ! ("x")`, a `!` whose delimiter sits on the next line, and a
            // comment between the name and the `!` (masking blanks the comment, which
            // used to destroy the literal `include!`). A splice renumbers faces either
            // way, so the gate has to see the macro, not one spelling of it.
            // 拼接搜索跑在整个屏蔽文本上而不是逐行，并接受宏名与 `!` 之间的任意空格。对逐行字面
            // 搜索实测有三种写法为绿：`include ! ("x")`、定界符位于下一行的 `!`、以及宏名与 `!`
            // 之间的注释（屏蔽会把注释抹白，过去会破坏字面量 `include!`）。三种写法都会重编面孔
            // 编号，因此门禁必须看见这个宏，而不是它的某一种拼法。
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
            let masked = nichlink::source::mask_non_code(&text);
            let mut from = 0usize;
            while let Some(offset) = masked[from..].find("include") {
                let at = from + offset;
                let boundary = at == 0
                    || !masked[..at]
                        .chars()
                        .next_back()
                        .is_some_and(|previous| previous.is_alphanumeric() || previous == '_');
                let after = masked[at + "include".len()..].trim_start_matches([' ', '\t']);
                if boundary && after.starts_with('!') {
                    let line = masked[..at].matches('\n').count() + 1;
                    let source_line = text.lines().nth(line - 1).unwrap_or("").trim();
                    found.includes.push(format!(
                        "{}:{} {}",
                        relative(root, &path),
                        line,
                        source_line
                    ));
                }
                from = at + "include".len();
            }
        }
    }
    found
}

#[cfg(test)]
#[path = "mounting_tests.rs"]
mod mounting_tests;
