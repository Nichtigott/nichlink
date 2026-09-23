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

use std::path::Path;

use crate::{is_comment, lines, relative, rust_sources};

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
];

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
        for (index, line) in lines(&path).iter().enumerate() {
            if is_comment(line) {
                continue;
            }
            for token in FORBIDDEN {
                if line.contains(token) {
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
