//! The kernel-purity gate: `core/` does no I/O, reads no environment, spawns
//! nothing, and binds to no process lifetime.
//! 内核纯净性门禁：`core/` 不做 I/O、不读环境、不派生进程、不绑定进程生存期。
//!
//! `AGENTS.md` states the rule together with its escape hatch: if the kernel
//! needs such a value, it arrives as a parameter. Until this gate existed the
//! rule held only because reviewers remembered it, and the verdict lived in a
//! human-audit document (`docs/audit-2026-09-21.md` lists "kernel purity" under
//! *Known clean*) rather than in a check that can fail a build.
//! `AGENTS.md` 陈述了该规则及其出口：内核若需要这类值，就以参数传入。在本门禁出现之前，
//! 这条规则只靠评审者记得，结论记在人工审计文档里（`docs/audit-2026-09-21.md` 把
//! "kernel purity" 列在 *Known clean* 下），而不是记在一个能让构建失败的检查里。
//!
//! Boundary, stated rather than discovered later: the walk covers all of
//! `core/src`, including inline `#[cfg(test)]` modules, because the directory
//! itself is the promise. A test that genuinely needs a temporary directory or
//! an environment variable belongs in `core/tests/`, which is outside this walk,
//! and so does any host. `std::path` is deliberately allowed: it is a value
//! vocabulary, not an I/O capability, and forbidding it would push callers into
//! hand-rolled string splitting.
//! 边界，明说而不是留给以后发现：遍历覆盖 `core/src` 全部内容，包括内联的
//! `#[cfg(test)]` 模块，因为目录本身就是承诺。真正需要临时目录或环境变量的测试应放在
//! `core/tests/`，那不在遍历范围内，宿主同样如此。`std::path` 有意放行：它是取值词汇而
//! 不是 I/O 能力，禁掉它只会把调用方推向手写的字符串切分。

use std::fs;
use std::path::Path;

use crate::{relative, rust_sources};

/// Tokens that must not appear in the kernel outside comments.
/// 内核中除注释外不得出现的词。
///
/// `std::time` is included because both of its types break purity in different
/// ways: `Instant` binds to process lifetime, and a wall-clock reading makes a
/// diagnostic irreproducible. The escape hatch is the same as for I/O — take
/// the value as a parameter.
/// 列入 `std::time` 是因为它的两个类型以不同方式破坏纯净性：`Instant` 绑定进程生存期，
/// 读取挂钟时间则让诊断不可复现。出口与 I/O 相同：把值作为参数传入。
pub const FORBIDDEN: &[&str] = &[
    "std::fs",
    "std::env",
    "std::process",
    "std::net",
    "std::time",
    "std::io",
    "std::thread",
    "std::os",
    // The compile-time readers of the environment. They were measured green against the
    // old line scan, and they are the same capability as `std::env::var`: a constant
    // baked from the build machine.
    // 环境的编译期读取器。它们对旧的逐行扫描实测为绿，而它们与 `std::env::var` 是同一种能力：
    // 一个从构建机烤进二进制的常量。
    "env!",
    "option_env!",
];

/// The paths a `use std::*;` makes reachable without the `std::` prefix.
/// 一条 `use std::*;` 让不带 `std::` 前缀的这些路径变得可达。
///
/// Only consulted when the file carries a glob import of `std`, so a local module that
/// happens to be called `fs` is not reported by a file that never imported `std` wholesale.
/// 只有在文件确实带了 `std` 的 glob 导入时才查这份表，因此一个凑巧叫 `fs` 的本地模块不会在
/// 从未整包导入 `std` 的文件里被报出来。
const GLOB_FORBIDDEN: &[&str] = &[
    "fs::",
    "env::",
    "process::",
    "net::",
    "time::",
    "io::",
    "thread::",
    "os::",
];

/// Blank the lines an `#[cfg(any())]` attribute governs.
/// 把 `#[cfg(any())]` 属性所管辖的那些行抹白。
///
/// Only `#[cfg(any())]`, not `#[cfg(test)]`: an inline test module *is* compiled in a test
/// build, and the kernel's promise covers everything under `core/src`.
/// 只认 `#[cfg(any())]`，不认 `#[cfg(test)]`：内联测试模块在测试构建里**确实**会被编译，而内核的
/// 承诺覆盖 `core/src` 下的全部内容。
fn drop_never_compiled(masked: &str) -> String {
    crate::drop_governed_lines(masked, |trimmed| trimmed == "#[cfg(any())]")
}

/// The masked text with one separator per whitespace run, and the source line each
/// remaining character came from.
/// 屏蔽后的文本，每段空白保留一个分隔符，外加剩下每个字符来自的源码行。
///
/// The earlier version deleted every whitespace character, which glued `use std::fs;`
/// into `usestd::fs;`; the left-boundary test then read the `e` of `use` as part of a
/// longer name and the *idiomatic* import was invisible. A separator next to `:` is
/// dropped so `std :: fs` still folds to `std::fs`.
/// 早先的版本删掉每一个空白字符，把 `use std::fs;` 粘成 `usestd::fs;`；左边界测试于是把
/// `use` 的 `e` 读成更长名字的一部分，**最惯用的**那种 import 就此不可见。紧邻 `:` 的分隔符会
/// 被丢弃，因此 `std :: fs` 仍折叠成 `std::fs`。
fn folded_for_search(masked: &str) -> (String, Vec<usize>) {
    const SEP: char = '\u{1}';
    let mut folded = String::with_capacity(masked.len());
    let mut lines = Vec::with_capacity(masked.len());
    let mut pending: Option<usize> = None;
    for (index, line) in masked.lines().enumerate() {
        let number = index + 1;
        for character in line.chars() {
            if character.is_whitespace() {
                pending.get_or_insert(number);
                continue;
            }
            if let Some(at) = pending.take() {
                let previous_is_colon = folded.ends_with(':');
                if !previous_is_colon && character != ':' {
                    folded.push(SEP);
                    lines.push(at);
                }
            }
            folded.push(character);
            lines.push(number);
        }
    }
    (folded, lines)
}

/// Every `std::{a, b}` in the folded text expanded into the paths it names, with the
/// line map kept aligned.
/// 折叠文本里每个 `std::{a, b}` 展开成它命名的那些路径，并保持行映射对齐。
///
/// Folding removed the newlines a multi-line brace import used to hide behind, so this
/// sees every one of them.
/// 折叠去掉了多行树形导入曾用来藏身的换行，因此这里能看到每一个。
fn expand_folded_imports(folded: String, lines: Vec<usize>) -> (String, Vec<usize>) {
    const SEP: char = '\u{1}';
    let mut expanded = String::with_capacity(folded.len());
    let mut expanded_lines = Vec::with_capacity(lines.len());
    let characters = folded.chars().collect::<Vec<_>>();
    let mut index = 0usize;
    while index < characters.len() {
        let head = characters[index..].iter().take(6).collect::<String>();
        if head == "std::{" {
            // Match the closing brace by depth: `std::{{fs}, env}` names two paths, and
            // cutting at the first `}` would read it as one malformed item.
            // 按深度匹配闭合花括号：`std::{{fs}, env}` 命名两条路径，而在第一个 `}` 处截断会把
            // 它读成一个畸形条目。
            let mut depth = 0usize;
            let mut end = None;
            for (offset, character) in characters[index + 5..].iter().enumerate() {
                match character {
                    '{' => depth += 1,
                    '}' => {
                        depth = depth.saturating_sub(1);
                        if depth == 0 {
                            end = Some(index + 5 + offset);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if let Some(end) = end {
                for (offset, character) in characters[index..=end].iter().enumerate() {
                    expanded.push(*character);
                    expanded_lines.push(lines.get(index + offset).copied().unwrap_or(1));
                }
                let line = lines.get(index).copied().unwrap_or(1);
                // Split the group at its *top-level* commas, so a nested group stays whole
                // until it is unwrapped below.
                // 在该组的**顶层**逗号处切分，因此嵌套组会整体保留，直到下面被拆开。
                let mut items: Vec<String> = Vec::new();
                let mut current = String::new();
                let mut depth = 0usize;
                for character in characters[index + 6..end].iter() {
                    match character {
                        '{' => {
                            depth += 1;
                            current.push(*character);
                        }
                        '}' => {
                            depth = depth.saturating_sub(1);
                            current.push(*character);
                        }
                        ',' if depth == 0 => items.push(std::mem::take(&mut current)),
                        _ => current.push(*character),
                    }
                }
                items.push(current);
                for item in items {
                    let item = item.trim_matches(|character: char| {
                        character == SEP
                            || character == '{'
                            || character == '}'
                            || character.is_whitespace()
                    });
                    if item.is_empty() || item == "self" {
                        continue;
                    }
                    // `use std::{self as s};` names an alias rather than a path, so it is
                    // rewritten into the plain form the alias table already reads.
                    // `use std::{self as s};` 命名的是一个别名而不是路径，因此把它改写成别名表
                    // 已经会读的普通形式。
                    if let Some(rest) = item.strip_prefix("self") {
                        let rest = rest.trim_start_matches(|character: char| {
                            character == SEP || character.is_whitespace()
                        });
                        if let Some(alias) = rest.strip_prefix("as") {
                            let alias = alias.trim_matches(|character: char| {
                                character == SEP || character.is_whitespace()
                            });
                            if !alias.is_empty() {
                                expanded.push(' ');
                                expanded_lines.push(line);
                                for character in format!("use{SEP}std{SEP}as{SEP}{alias};").chars()
                                {
                                    expanded.push(character);
                                    expanded_lines.push(line);
                                }
                            }
                            continue;
                        }
                    }
                    expanded.push(' ');
                    expanded_lines.push(line);
                    for character in format!("std::{item}").chars() {
                        expanded.push(character);
                        expanded_lines.push(line);
                    }
                }
                index = end + 1;
                continue;
            }
        }
        expanded.push(characters[index]);
        expanded_lines.push(lines.get(index).copied().unwrap_or(1));
        index += 1;
    }
    (expanded, expanded_lines)
}

/// Every local alias of `std`, from `use std as <alias>;` — whitespace-free, since the
/// search text is folded.
/// 每个 `std` 的局部别名，来自 `use std as <alias>;`——搜索文本已折叠，因此没有空白。
fn std_aliases(searchable: &str) -> Vec<String> {
    // The search text keeps one separator per whitespace run, so the needles carry it
    // too. Three spellings beside the plain one are read here: `use ::std as s;`,
    // `extern crate std as s;`, and the brace form the expander rewrites into the plain
    // one above.
    // 搜索文本为每段空白保留一个分隔符，因此 needle 也带上它。除了普通拼写，这里还认三种：
    // `use ::std as s;`、`extern crate std as s;`，以及展开器会改写成普通形式的大括号写法。
    const SEP: &str = "\u{1}";
    let needles = [
        format!("use{SEP}std{SEP}as{SEP}"),
        format!("use::std{SEP}as{SEP}"),
        format!("crate{SEP}std{SEP}as{SEP}"),
    ];
    let mut aliases = Vec::new();
    for needle in &needles {
        let mut rest = searchable;
        while let Some(offset) = rest.find(needle.as_str()) {
            let after = &rest[offset + needle.len()..];
            let alias = after
                .chars()
                .take_while(|character| character.is_alphanumeric() || *character == '_')
                .collect::<String>();
            if !alias.is_empty() && !aliases.contains(&alias) {
                aliases.push(alias);
            }
            rest = after;
        }
    }
    aliases
}

/// One forbidden token on one line of the kernel.
/// 内核某一行上的一个禁用词。
#[derive(Debug)]
pub struct Finding {
    /// Path relative to the workspace root, with `/` separators.
    /// 以工作区根为基准的路径，使用 `/` 分隔符。
    pub file: String,
    /// One-based line number.
    /// 从 1 开始的行号。
    pub line: usize,
    /// The token that matched.
    /// 命中的词。
    pub token: &'static str,
}

/// Every purity violation under `<root>/core/src`.
/// `<root>/core/src` 下的每一处纯净性违规。
pub fn findings(root: &Path) -> Vec<Finding> {
    let mut found = Vec::new();
    for path in rust_sources(&root.join("kernel").join("src")) {
        // What counts as code is the kernel's own rule rather than a second one
        // kept here: `mask_non_code` blanks string literals and both comment
        // forms while keeping every line break, so `/* std::fs */` and
        // `let probe = "std::fs";` are prose while the code around them is not.
        // The line-based `//` test this replaced reported a block comment, and
        // `is_comment` had no caller left once it went.
        // 什么算代码由内核自己的规则决定，而不是这里另留一份：`mask_non_code` 抹白字符串字面量
        // 与两种注释形式并保留每一个换行，因此 `/* std::fs */` 与 `let probe = "std::fs";`
        // 是散文，而它们周围的代码不是。这里替换掉的按行 `//` 判断会把块注释报出来，而在它退场
        // 之后 `is_comment` 已无调用方。
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        // The search runs on a whitespace-free copy of the masked text, not line by
        // line. Four spellings were measured green against the line-based scan: a brace
        // import whose braces are on different lines, `std :: fs` with spaces around the
        // separators, a path split across a newline (`std::` ↵ `fs::metadata`), and
        // `use std as s;` followed by `s::fs::…`. Removing whitespace closes the first
        // three — a path is a token stream, not a line — and the alias table closes the
        // fourth. `expand_imports` then sees every brace import on one line, because
        // folding removed the newlines inside it.
        // 搜索跑在屏蔽后文本的**去空白副本**上，而不是逐行。对逐行扫描实测有四种写法是绿的：
        // 花括号分处两行的树形导入、分隔符两旁有空格的 `std :: fs`、被换行切开的路径
        // （`std::` ↵ `fs::metadata`）、以及 `use std as s;` 之后的 `s::fs::…`。去掉空白关闭前
        // 三种——路径是 token 流而不是行——别名表关闭第四种。`expand_imports` 随后看到的每个树形
        // 导入都在同一行上，因为折叠已经把其中的换行去掉了。
        let masked = xirang_kernel::source::mask_non_code(&text);
        // `#[cfg(any())]` is the workspace's idiom for code that is never compiled, so a
        // line carrying it is not a capability the kernel has. Reporting it was a false
        // positive that made the gate look wrong about a file it had read correctly.
        // `#[cfg(any())]` 是本工作区表示"永不编译"的写法，因此带它的那一行不是内核拥有的能力。
        // 把它报出来是误报，会让门禁在一份它其实读对了的文件上显得不对。
        let masked = drop_never_compiled(&masked);
        let (folded, lines) = folded_for_search(&masked);
        let (searchable, search_lines) = expand_folded_imports(folded, lines);
        let aliases = std_aliases(&searchable);
        let mut reported = std::collections::BTreeSet::new();
        for token in FORBIDDEN {
            let mut from = 0usize;
            while let Some(offset) = searchable[from..].find(token) {
                let at = from + offset;
                // An identifier character immediately before a token means the token is
                // part of a longer name (`my_env!`, `reth::fs`), not the path.
                // 紧邻 token 之前的标识符字符意味着它是更长名字的一部分（`my_env!`、`reth::fs`），
                // 而不是那个路径。
                let boundary = at == 0
                    || !searchable[..at]
                        .chars()
                        .next_back()
                        .is_some_and(|previous| previous.is_alphanumeric() || previous == '_');
                if boundary {
                    reported.insert((search_lines.get(at).copied().unwrap_or(1), *token));
                }
                from = at + token.len();
            }
        }
        // An alias declared anywhere in the file makes `alias::fs` the same capability
        // as `std::fs`.
        // 文件里任何位置声明的别名都让 `alias::fs` 与 `std::fs` 是同一种能力。
        for alias in aliases {
            for token in FORBIDDEN {
                let Some(module) = token.strip_prefix("std::") else {
                    continue;
                };
                let aliased = format!("{alias}::{module}");
                let mut from = 0usize;
                while let Some(offset) = searchable[from..].find(&aliased) {
                    let at = from + offset;
                    reported.insert((search_lines.get(at).copied().unwrap_or(1), *token));
                    from = at + aliased.len();
                }
            }
        }
        // A glob import of `std` makes every module name reachable unqualified, so the
        // bare spelling is the same capability. The boundary test also rejects a `:`
        // before the token, so `reth::fs::…` stays green.
        // 一条 `std` 的 glob 导入让每个模块名不带前缀就可达，因此裸拼写是同一种能力。边界测试
        // 还会拒绝 token 之前的 `:`，所以 `reth::fs::…` 保持绿色。
        if searchable.contains("std::*") {
            for token in GLOB_FORBIDDEN {
                let mut from = 0usize;
                while let Some(offset) = searchable[from..].find(token) {
                    let at = from + offset;
                    let boundary = at == 0
                        || !searchable[..at]
                            .chars()
                            .next_back()
                            .is_some_and(|previous| {
                                previous.is_alphanumeric() || previous == '_' || previous == ':'
                            });
                    if boundary {
                        let named = FORBIDDEN
                            .iter()
                            .find(|candidate| candidate.ends_with(token))
                            .copied()
                            .unwrap_or("std::*");
                        reported.insert((search_lines.get(at).copied().unwrap_or(1), named));
                    }
                    from = at + token.len();
                }
            }
        }
        for (line, token) in reported {
            found.push(Finding {
                file: relative(root, &path),
                line,
                token,
            });
        }
    }
    found
}

#[cfg(test)]
#[path = "purity_tests.rs"]
mod purity_tests;
