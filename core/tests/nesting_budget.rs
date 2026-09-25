//! The nesting guard must refuse nothing this repository actually contains.
//! 嵌套守卫不得拒绝本仓库里真实存在的任何东西。
//!
//! The guard's limits are deliberately far below what overflows a stack, so the
//! risk it carries is the opposite one: a threshold that refuses real source. That
//! risk cannot be settled by reading the heuristic — the only way to know is to
//! run it over every source file in the workspace and look. This test does that,
//! and it is why the third measured shape could be added at all: the linear-run
//! limit is a thousand tokens, which sounds tight until the longest run in this
//! repository turns out to be a small fraction of it.
//! 守卫的上限刻意远低于会撑爆栈的量级，因此它自带的风险是相反的那一个：某个阈值拒绝了真实
//! 源码。这个风险无法靠阅读启发式来定案——唯一的办法是把它跑遍工作区里每个源文件再看结果。
//! 本测试做的正是这件事，也正是第三种形状能被加进来的原因：线性串上限是一千个 token，听起来
//! 很紧，直到本仓库里最长的串被发现只占它的一小部分。
//!
//! It lives in `core/tests/` rather than beside the unit tests because it walks the
//! filesystem, and the kernel-purity gate covers all of `core/src` — the gate's
//! own documentation names this directory as the place for a test that genuinely
//! needs one.
//! 它在 `core/tests/` 而不是单元测试旁边，因为它要遍历文件系统，而内核纯净性门禁覆盖
//! `core/src` 全部——那道门禁自己的文档就把本目录点名为"确实需要这类东西的测试"该去的地方。

// The guard only exists with the parser, so the whole file is gated with it. A
// whole-workspace build enables `syntax` through the other members; a bare
// `cargo test -p nichlink-core` does not, and then this file has nothing to say.
// 守卫只随解析器存在，因此整个文件与之同门控。整工作区构建会经其他成员打开 `syntax`；
// 单跑 `cargo test -p nichlink-core` 不会，那时本文件无话可说。
#![cfg(feature = "syntax")]

use std::path::{Path, PathBuf};

/// The phrase a nesting refusal carries, from `TooDeep`'s `Display`.
/// 嵌套拒绝所带的措辞，来自 `TooDeep` 的 `Display`。
const REFUSAL: &str = "above the limit of";

/// Every `.rs` file under `directory`, skipping build output and version control.
/// `directory` 下每个 `.rs` 文件，跳过构建输出与版本控制。
fn rust_files(directory: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if name == "target" || name == ".git" {
                continue;
            }
            rust_files(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

#[test]
fn no_source_in_this_workspace_is_refused_by_the_nesting_guard() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the kernel lives in a workspace")
        .to_path_buf();
    let mut files = Vec::new();
    rust_files(&root, &mut files);
    // A floor, so a walk that silently finds nothing cannot pass. The workspace
    // has hundreds of Rust files; the number is not asserted exactly because a
    // new module should not have to update this test.
    // 一个下限，使"什么都没找到"的遍历无法通过。工作区有数百个 Rust 文件；这里不断言精确数字，
    // 因为新增模块不该被迫来改这条测试。
    assert!(
        files.len() > 100,
        "the walk found {} Rust files under {}; it is supposed to cover the workspace",
        files.len(),
        root.display()
    );

    let mut refused = Vec::new();
    for path in &files {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        // A file that is not a face source is allowed to fail to parse: what must
        // never happen is a *nesting* refusal, because that means the guard would
        // reject a file this repository ships.
        // 不是注册面源码的文件允许解析失败：绝不允许发生的是**嵌套**拒绝，因为那意味着守卫会
        // 拒绝一个本仓库出厂的文件的。
        if let Err(error) = nichlink::registry_core::syntax::parse_faces(&text) {
            let message = error.to_string();
            if message.contains(REFUSAL) {
                refused.push(format!("{}: {message}", path.display()));
            }
        }
    }
    assert!(
        refused.is_empty(),
        "the nesting guard refuses {} of this workspace's own files:\n{}",
        refused.len(),
        refused.join("\n")
    );
    println!("the nesting guard accepted all {} Rust files", files.len());
}
