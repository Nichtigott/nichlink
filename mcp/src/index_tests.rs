//! Tests for the source index and its path safety.
//! 源码索引及其路径安全的测试。

use super::*;

/// The write path's recoverable trash lives under `.nichlink/` and holds
/// `.rs` files, so a deleted fact must not keep answering `status` and
/// `search` from its own backup.
/// 写入路径的可恢复回收目录在 `.nichlink/` 下、里面就是 `.rs` 文件，因此被删掉的东西不得继续
/// 从它自己的备份里回答 `status` 与 `search`。
#[test]
fn the_recoverable_trash_is_not_indexed() {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("mcp-scan-{}-{sequence}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("source directory");
    std::fs::create_dir_all(root.join(".nichlink/trash/faces")).expect("trash directory");
    std::fs::write(root.join("src/live.rs"), "pub fn live() {}\n").expect("live file");
    std::fs::write(
        root.join(".nichlink/trash/faces/deleted.rs"),
        "pub fn deleted() {}\n",
    )
    .expect("backup file");
    let files = load_sources(&root).expect("the scan reads the tree");
    let names = files
        .iter()
        .map(|file| file.relative.clone())
        .collect::<Vec<_>>();
    assert!(names.iter().any(|name| name == "src/live.rs"), "{names:?}");
    assert!(
        !names.iter().any(|name| name.contains(".nichlink")),
        "the trash must not be indexed: {names:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn parser_indexes_functions_and_direct_calls() {
    let functions = parse_functions(
        "fn source() { let value = helper(1); sink(value); }\nfn helper(_: i32) {}\nfn sink(_: i32) {}",
    );
    assert_eq!(functions.len(), 3);
    assert_eq!(functions[0].name, "source");
    assert_eq!(functions[0].calls, ["helper", "sink"]);
}

#[test]
fn registration_kinds_are_compact_and_deduplicated() {
    let kinds = nichlink::source::registration_kinds(
        "crate::control_object! { kind: Button, }\ncrate::control_object! { kind: Button, }",
    );
    assert_eq!(kinds, ["Button"]);
}

/// A throwaway source root, unique per call.
/// 每次调用唯一的临时源码根。
fn temporary_root(tag: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "nichlink-mcp-{tag}-{}-{sequence}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create root");
    root
}

/// A file genuinely inside the root is read, so the guard is about location
/// and not about links as such.
/// 真正位于根内的文件照常读取，因此这道守卫针对的是位置而不是链接本身。
#[test]
fn a_file_inside_the_source_root_is_still_read() {
    let root = temporary_root("inside");
    fs::write(root.join("inside.rs"), "fn inside() {}\n").expect("write");
    let sources = load_sources(&root).expect("the walk reads the root");
    assert_eq!(sources.len(), 1);
    assert!(sources[0].source.contains("fn inside"));
    let _ = fs::remove_dir_all(&root);
}

/// A link out of the root is neither walked into nor read. A prefix
/// comparison accepts both, because the link's own path is inside the root.
/// 指向根外的链接既不会被进入，也读不到。前缀比较两者都会接受，因为链接自身的路径就在
/// 根内。
#[cfg(unix)]
#[test]
fn a_link_out_of_the_source_root_is_neither_walked_nor_read() {
    let root = temporary_root("escape");
    let outside = root.with_file_name(format!(
        "{}-outside",
        root.file_name().expect("a name").to_string_lossy()
    ));
    let _ = fs::remove_dir_all(&outside);
    fs::create_dir_all(&outside).expect("create outside tree");
    fs::write(outside.join("secret.rs"), "fn secret() {}\n").expect("write outside file");
    std::os::unix::fs::symlink(&outside, root.join("linked")).expect("link the outside tree");
    std::os::unix::fs::symlink(outside.join("secret.rs"), root.join("secret.rs"))
        .expect("link the outside file");

    let sources = load_sources(&root).expect("the walk survives an escaping link");
    assert!(
        sources
            .iter()
            .all(|file| !file.source.contains("fn secret")),
        "a file outside the root must not be indexed: {sources:?}"
    );
    let error = load_one(&root, "secret.rs").expect_err("a link out of the root is refused");
    assert!(error.contains("source root"), "{error}");

    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_dir_all(&outside);
}

/// A tree-relative path has one spelling, and it is the shared rule's: a file whose
/// *name* contains a backslash — legal bytes on Unix — is reported with `/`
/// separators here and in `preview`'s declaration anchors alike, because neither
/// decides the spelling for itself. This module used to fold only the platform
/// separator, so on Unix it reported `a\b.rs` where the anchor that names the same
/// file reported `a/b.rs`.
/// 树内相对路径只有一种拼法，而且来自那条共用规则：名字里含反斜杠的文件（Unix 上合法的字节）在这里
/// 与 `preview` 的声明锚点里都用 `/` 分隔符报告，因为两者都不自行决定拼法。本模块过去只折平台分隔符，
/// 因此在 Unix 上它报 `a\b.rs`，而命名同一个文件的锚点报 `a/b.rs`。
#[cfg(unix)]
#[test]
fn a_backslash_in_a_file_name_is_spelled_one_way() {
    let root = temporary_root("portable");
    fs::write(root.join(r"a\b.rs"), "fn inside() {}\n").expect("write");
    let sources = load_sources(&root).expect("the walk reads the root");
    let names = sources
        .iter()
        .map(|file| file.relative.clone())
        .collect::<Vec<_>>();
    assert_eq!(names, ["a/b.rs"], "{names:?}");
    let _ = fs::remove_dir_all(&root);
}
