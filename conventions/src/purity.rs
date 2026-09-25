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
];

/// The text of one line with `use std::{a, b}` expanded into the paths it names.
/// 一行的文本，其中 `use std::{a, b}` 展开为它命名的那些路径。
///
/// A brace import never spells `std::fs`, so the literal search below missed it.
/// The import line *is* the violation, which is why no alias resolution is needed:
/// `use std::fs as filesystem;` already contains `std::fs`, and a bare `fs::read`
/// after `use std::{env, fs}` is caught at the import that made it possible.
/// 树形导入从不拼出 `std::fs`，因此下面的字面量搜索看不到它。导入行**就是**违规本身，这也正是
/// 无需解析别名的原因：`use std::fs as filesystem;` 里已经有 `std::fs`，而
/// `use std::{env, fs}` 之后的裸 `fs::read` 在使其成为可能的那个导入处被抓到。
fn expand_imports(line: &str) -> String {
    let Some(start) = line.find("std::{") else {
        return line.to_owned();
    };
    let mut expanded = line.to_owned();
    let rest = &line[start + "std::{".len()..];
    let Some(end) = rest.find('}') else {
        return expanded;
    };
    for item in rest[..end].split(',') {
        let item = item.trim();
        if item.is_empty() || item == "self" {
            continue;
        }
        expanded.push(' ');
        expanded.push_str("std::");
        expanded.push_str(item);
    }
    expanded
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
        for (index, line) in nichlink::source::mask_non_code(&text).lines().enumerate() {
            let searchable = expand_imports(line);
            for token in FORBIDDEN {
                if searchable.contains(token) {
                    found.push(Finding {
                        file: relative(root, &path),
                        line: index + 1,
                        token,
                    });
                }
            }
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
