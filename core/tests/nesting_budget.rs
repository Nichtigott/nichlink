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

/// What the guard says about a set of files: the ones it refused, and the ones it
/// could not read at all.
/// 守卫对一组文件的结论：它拒绝的那些，以及它根本读不到的那些。
fn inspect(files: &[PathBuf]) -> (Vec<String>, Vec<String>) {
    let mut refused = Vec::new();
    let mut unreadable = Vec::new();
    for path in files {
        let Ok(text) = std::fs::read_to_string(path) else {
            // A file the walk found but cannot read is not "clean". The guard never
            // saw it, and this test's whole point is that the guard sees everything
            // the repository ships; skipping it silently is how a budget test decays
            // into a test of whatever it happened to be able to open.
            // 遍历找到却读不到的文件不算"干净"。守卫从未见过它，而本测试的全部意义正是守卫见过
            // 仓库出厂的每一样东西；静默跳过它，就是一道预算测试退化成"只测它碰巧能打开的那些
            // 文件"的方式。
            unreadable.push(path.display().to_string());
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
    (refused, unreadable)
}

#[test]
fn no_source_in_this_workspace_is_refused_by_the_nesting_guard() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest
        .parent()
        .expect("the kernel is unpacked somewhere")
        .to_path_buf();
    // This test ships inside the published crate, and a consumer has no workspace around
    // it: from a registry unpack the parent is the registry's `src/` directory, so the
    // walk used to "pass" by scanning whichever unrelated crates happened to be unpacked
    // next to it. It says so and stands down instead — the promise it keeps ("the guard
    // refuses nothing this repository ships") is about this repository.
    // 本测试随已发布 crate 出厂，而消费者周围没有工作区：从 registry 解包后父目录是 registry
    // 的 `src/`，于是遍历过去靠"恰好解包在旁边的无关 crate"来"通过"。现在它说明情况并退出——
    // 它守的承诺（"守卫不拒绝本仓库出厂的任何东西"）是关于本仓库的。
    if !root.join("core").is_dir() || !root.join("build_method").is_dir() {
        eprintln!(
            "skipping: {} is not a NichLink checkout; this gate is about that repository",
            root.display()
        );
        return;
    }
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

    let (refused, unreadable) = inspect(&files);
    assert!(
        unreadable.is_empty(),
        "the walk found {} file(s) it could not read, so the budget does not cover \
         them and cannot claim the workspace is clean:\n{}",
        unreadable.len(),
        unreadable.join("\n")
    );
    assert!(
        refused.is_empty(),
        "the nesting guard refuses {} of this workspace's own files:\n{}",
        refused.len(),
        refused.join("\n")
    );
    println!("the nesting guard accepted all {} Rust files", files.len());
}

/// A file that cannot be read is reported rather than skipped, because a skipped
/// file is one the guard never checked.
/// 读不到的文件会被报出而不是被跳过，因为被跳过的文件就是守卫从未检查过的文件。
#[test]
fn a_file_that_cannot_be_read_is_reported() {
    let path = std::env::temp_dir().join(format!(
        "nichlink-nesting-unreadable-{}-{}.rs",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    // Invalid UTF-8 in a file that exists: exactly the shape of a silently skipped
    // source, and `read_to_string` refuses it.
    // 一个存在但含非法 UTF-8 的文件：正是被静默跳过的源码的形状，而 `read_to_string` 会拒绝它。
    std::fs::write(&path, [0x66, 0x6e, 0x20, 0xff, 0xfe, 0x0a]).expect("fixture file");
    let (refused, unreadable) = inspect(std::slice::from_ref(&path));
    let _ = std::fs::remove_file(&path);
    assert!(refused.is_empty(), "{refused:#?}");
    assert_eq!(unreadable.len(), 1, "{unreadable:#?}");
}
