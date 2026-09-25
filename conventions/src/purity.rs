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

/// Blank the lines an `#[cfg(any())]` attribute governs.
/// 把 `#[cfg(any())]` 属性所管辖的那些行抹白。
///
/// The attribute governs the item on the following lines (and its indented body), so the
/// scan skips lines from the attribute until the indentation returns to the attribute's
/// own level.
/// 该属性管辖其后若干行的条目（以及它缩进的主体），因此扫描从该属性起跳过，直到缩进回到属性
/// 自身的层级。
fn drop_never_compiled(masked: &str) -> String {
    let mut kept = String::with_capacity(masked.len());
    let mut skipped_indent: Option<usize> = None;
    for line in masked.lines() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        if let Some(level) = skipped_indent {
            if trimmed.is_empty() || indent > level {
                kept.push('\n');
                continue;
            }
            skipped_indent = None;
        }
        if trimmed == "#[cfg(any())]" {
            skipped_indent = Some(indent);
            kept.push('\n');
            continue;
        }
        kept.push_str(line);
        kept.push('\n');
    }
    kept
}

/// The masked text with every whitespace character removed, and the source line each
/// remaining character came from.
/// 屏蔽后的文本去掉每一个空白字符，外加剩下每个字符来自的源码行。
fn folded_for_search(masked: &str) -> (String, Vec<usize>) {
    let mut folded = String::with_capacity(masked.len());
    let mut lines = Vec::with_capacity(masked.len());
    for (index, line) in masked.lines().enumerate() {
        for character in line.chars() {
            if character.is_whitespace() {
                continue;
            }
            folded.push(character);
            lines.push(index + 1);
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
    let mut expanded = String::with_capacity(folded.len());
    let mut expanded_lines = Vec::with_capacity(lines.len());
    let characters = folded.chars().collect::<Vec<_>>();
    let mut index = 0usize;
    while index < characters.len() {
        let head = characters[index..].iter().take(6).collect::<String>();
        if head == "std::{"
            && let Some(end) = characters[index..].iter().position(|c| *c == '}')
        {
            let end = index + end;
            for (offset, character) in characters[index..=end].iter().enumerate() {
                expanded.push(*character);
                expanded_lines.push(lines.get(index + offset).copied().unwrap_or(1));
            }
            let line = lines.get(index).copied().unwrap_or(1);
            let inner = characters[index + 6..end].iter().collect::<String>();
            for item in inner.split(',') {
                let item = item.trim();
                if item.is_empty() || item == "self" {
                    continue;
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
    const NEEDLE: &str = "usestdas";
    let mut aliases = Vec::new();
    let mut rest = searchable;
    while let Some(offset) = rest.find(NEEDLE) {
        let after = &rest[offset + NEEDLE.len()..];
        let alias = after
            .chars()
            .take_while(|character| character.is_alphanumeric() || *character == '_')
            .collect::<String>();
        if !alias.is_empty() {
            aliases.push(alias);
        }
        rest = after;
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
    for path in rust_sources(&root.join("core").join("src")) {
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
        let masked = nichlink::source::mask_non_code(&text);
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
mod tests {
    use super::*;
    use crate::workspace_root;
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::PathBuf;

    /// A throwaway checkout with the given files under it.
    /// 一个只含给定文件的一次性检出。
    fn synthetic(files: &[(&str, &str)]) -> PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "nichlink-purity-{}-{}-{sequence}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        for (relative, contents) in files {
            let path = root.join(relative);
            fs::create_dir_all(path.parent().expect("parent")).expect("fixture dir");
            fs::write(&path, contents).expect("fixture file");
        }
        crate::fixture_manifest(&root);
        root
    }

    /// A brace import names a forbidden module without ever spelling `std::fs`,
    /// and `std::io` is I/O like everything else in the list.
    /// 树形导入从未拼出 `std::fs` 却命名了被禁的模块，而 `std::io` 与表里其余各项一样是 I/O。
    #[test]
    fn a_brace_import_and_std_io_are_violations() {
        let root = synthetic(&[(
            "core/src/probe.rs",
            "use std::{env, fs};\n\npub fn probe() -> String {\n    \
             let _ = fs::read_to_string(\"/etc/hostname\");\n    \
             let _ = env::var(\"HOME\");\n    \
             let _ = std::io::stdout();\n    String::new()\n}\n",
        )]);
        let found = findings(&root);
        let tokens = found
            .iter()
            .map(|finding| finding.token)
            .collect::<BTreeSet<_>>();
        assert!(
            tokens.contains("std::io"),
            "std::io is I/O and belongs in the list: {found:#?}"
        );
        assert!(
            tokens.contains("std::env") && tokens.contains("std::fs"),
            "a brace import names both modules: {found:#?}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// Prose that names a forbidden module is not a violation, and the code
    /// beside it still is.
    /// 点名被禁模块的散文不是违规，而它旁边的代码依然是。
    #[test]
    fn comments_and_strings_are_not_the_code_that_is_scanned() {
        let root = synthetic(&[(
            "core/src/probe.rs",
            "/* std::fs is banned here, and this block comment says so */\n\
             /// `let probe = \"std::env\";` is prose too.\n\
             pub fn probe() -> usize {\n    \
             let _note = \"std::thread::spawn\";\n    \
             let _ = std::fs::metadata(\"/tmp\");\n    1\n}\n",
        )]);
        let found = findings(&root);
        let lines = found.iter().map(|finding| finding.line).collect::<Vec<_>>();
        assert_eq!(
            lines,
            vec![5],
            "only the real call is a violation: a block comment, a doc comment and a \
             string literal are prose, and line 5 is the code beside them: {found:#?}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// The four spellings a line-based scan was measured to miss: a brace import split
    /// across lines, spaces around the separators, a path split across a newline, and a
    /// compile-time environment read.
    /// 逐行扫描实测漏掉的四种写法：跨行的树形导入、分隔符两旁的空格、被换行切开的路径，以及
    /// 编译期读环境。
    #[test]
    fn the_ways_a_path_can_hide_from_a_line_scan_are_violations() {
        let root = synthetic(&[(
            "core/src/probe.rs",
            "use std::{\n    env,\n    fs,\n};\n\n\
             pub fn probe() {\n    \
             let _ = std :: fs :: metadata(\"/tmp\");\n    \
             let _ = std::\n        env::var(\"HOME\");\n    \
             let _ = option_env!(\"HOME\");\n}\n",
        )]);
        let found = findings(&root);
        let tokens = found
            .iter()
            .map(|finding| finding.token)
            .collect::<BTreeSet<_>>();
        assert!(
            tokens.contains("std::fs"),
            "spaces around the separators: {found:#?}"
        );
        assert!(
            tokens.contains("std::env"),
            "a split path and a brace import: {found:#?}"
        );
        assert!(
            tokens.contains("option_env!"),
            "a compile-time env read: {found:#?}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// An aliased `std` is the same capability, and the alias is found wherever it is
    /// declared in the file.
    /// 别名化的 `std` 是同一种能力，而别名在文件里任何位置声明都能被找到。
    #[test]
    fn an_aliased_std_is_still_std() {
        let root = synthetic(&[(
            "core/src/probe.rs",
            "use std as s;\n\npub fn probe() {\n    let _ = s::fs::metadata(\"/tmp\");\n}\n",
        )]);
        let found = findings(&root);
        assert!(
            found.iter().any(|finding| finding.token == "std::fs"),
            "`s::fs` is `std::fs` under an alias: {found:#?}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// The walk has a floor, so a renamed kernel directory cannot read as "clean".
    /// 遍历有一个下限，因此内核目录一旦改名就不会读作"干净"。
    #[test]
    fn the_walk_covers_the_kernel_tree() {
        let sources = rust_sources(&workspace_root().join("core").join("src"));
        assert!(
            sources.len() > 40,
            "the purity walk found only {} files; it is supposed to cover core/src",
            sources.len()
        );
    }

    /// The kernel stays pure: the audit verdict is now a gate.
    /// 内核保持纯净：审计结论现在是一道门禁。
    #[test]
    fn the_kernel_does_no_io_and_reads_no_environment() {
        let root = workspace_root();
        let found = findings(&root);
        assert!(
            found.is_empty(),
            "kernel purity is a documented promise; move the value in as a parameter instead: {found:#?}"
        );
    }
}
