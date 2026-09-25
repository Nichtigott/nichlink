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

use std::path::Path;

use crate::{crate_directories, lines, relative, rust_sources};

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

/// Walk the workspace for mounting violations.
/// 遍历工作区，找出挂载违规。
pub fn findings(root: &Path) -> Findings {
    let mut found = Findings::default();
    for directory in crate_directories(root) {
        for path in rust_sources(&directory) {
            if path.file_name().is_some_and(|name| name == "mod.rs") {
                found.mod_rs.push(relative(root, &path));
            }
            for (index, line) in lines(&path).iter().enumerate() {
                // A splice is the `include!` macro, whatever delimiter it uses and
                // wherever it sits on the line: matching the exact `include!(` prefix
                // missed the brace spelling rustc's own diagnostic suggests, so a
                // splice could ship while the count still read 1. The search runs on
                // the kernel's masked text, because a workspace test's fixture string
                // that mentions `include!` is not a mount.
                // 拼接就是 `include!` 这个宏，无论它用哪种定界符、在行内什么位置：只匹配
                // `include!(` 前缀会漏掉 rustc 自己建议的花括号写法，于是拼接可以出厂而计数仍是 1。
                // 搜索跑在内核屏蔽后的文本上，因为工作区测试里提到 `include!` 的夹具字符串不是挂载。
                let code = nichlink::source::mask_non_code(line);
                let Some(position) = code.find("include!") else {
                    continue;
                };
                let delimiter = code[position + "include!".len()..].trim_start();
                if delimiter.starts_with(['(', '[', '{']) {
                    found.includes.push(format!(
                        "{}:{} {}",
                        relative(root, &path),
                        index + 1,
                        line.trim()
                    ));
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

    /// The only `include!` is the generated plan, and no file is named `mod.rs`.
    /// `include!` is recognised by the macro, not by one exact spelling: a
    /// delimiter change (`include! { … }`) splices a module just as well, and the
    /// workspace gate must see it.
    /// `include!` 靠宏本身识别，而不是靠一种拼法：换个定界符（`include! { … }`）照样拼接模块，
    /// 工作区门禁必须看见它。
    #[test]
    fn an_include_with_another_delimiter_is_still_a_splice() {
        let root = synthetic(&[(
            "cli/src/zz_audit_probe.rs",
            "pub mod spliced {\n    include! {\"zz_body.rs\"}\n}\n",
        )]);
        let found = findings(&root);
        assert_eq!(
            found.includes.len(),
            1,
            "a brace-delimited include! is a splice too: {:#?}",
            found.includes
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A throwaway checkout with the given files under it.
    /// 一个只含给定文件的一次性检出。
    fn synthetic(files: &[(&str, &str)]) -> std::path::PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "nichlink-mounting-{}-{}-{sequence}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        for (relative, contents) in files {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("fixture dir");
            std::fs::write(&path, contents).expect("fixture file");
        }
        crate::fixture_manifest(&root);
        root
    }

    /// 唯一的 `include!` 是生成计划，且没有文件叫 `mod.rs`。
    #[test]
    fn modules_are_mounted_without_splicing_identity() {
        let root = workspace_root();
        let found = findings(&root);
        assert!(
            found.mod_rs.is_empty(),
            "the workspace mounts modules with #[path], never mod.rs: {:#?}",
            found.mod_rs
        );
        assert_eq!(
            found.includes.len(),
            1,
            "exactly one include! mounts the generated plan; a second one would \
             splice a file and silently change the NodeId of every face it declares: {:#?}",
            found.includes
        );
        assert!(
            found.includes[0].contains("generated_lib.rs"),
            "the single include! must be host!()'s generated plan, found: {:#?}",
            found.includes
        );
    }
}
