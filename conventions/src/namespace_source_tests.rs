//! Pins for the namespace-source gate.
//! 命名空间来源门禁的钉子。

use super::findings_in;

/// A throwaway tree for the fixtures below.
/// 下面夹具用的一次性树。
fn tree(label: &str) -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "nichlink-namespace-source-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("src");
    root
}

/// The live tree spells the constant: this is the gate, and it is the reason the rule exists.
/// 活树拼的是常量：这就是门禁本身，也是这条规则存在的理由。
#[test]
fn the_shipped_tree_reads_the_constant() {
    let root = crate::workspace_root();
    let found = super::findings(&root);
    assert!(
        found.is_empty(),
        "face files still reading the package name:\n{}",
        found
            .iter()
            .map(super::Finding::reason)
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// A face file that reads the package name is named by file and line.
/// 读了包名的面文件会被点名到文件与行。
#[test]
fn a_face_file_reading_the_package_name_is_found() {
    let root = tree("found");
    std::fs::write(
        root.join("src/button.rs"),
        "pub struct Button;\n\ncrate::root_object! {\n    kind: Button,\n    parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
    )
    .expect("face");
    let found = findings_in(&root, &[root.join("src/button.rs")]);
    assert_eq!(found.len(), 1, "one line, one finding: {found:?}");
    assert_eq!(found[0].file, "src/button.rs");
    assert_eq!(found[0].line, 5, "the parent expression's line");
    assert!(
        found[0].reason().contains("NICHLINK_NAMESPACE"),
        "the reason names the replacement"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The two legitimate spellings stay legitimate: a **non-face** file that owns the constant, and the
/// definition line inside a face file.
/// 两种正当拼法仍然正当：**非面**文件里给常量下定义，以及面文件里那一行定义。
#[test]
fn owning_the_constant_is_not_a_finding() {
    let root = tree("legitimate");
    // A test crate that owns its constant and declares no face at all.
    std::fs::write(
        root.join("src/non_face.rs"),
        "pub const NICHLINK_NAMESPACE: &str = env!(\"CARGO_PKG_NAME\");\n",
    )
    .expect("non-face");
    // A face file that owns it as well: the definition line is allowed.
    std::fs::write(
        root.join("src/host.rs"),
        "pub const NICHLINK_NAMESPACE: &str = env!(\"CARGO_PKG_NAME\");\n\npub struct Button;\n\ncrate::root_object! {\n    kind: Button,\n    parent: crate::root_node_id(crate::NICHLINK_NAMESPACE),\n}\n",
    )
    .expect("face");
    let found = findings_in(
        &root,
        &[root.join("src/non_face.rs"), root.join("src/host.rs")],
    );
    assert!(
        found.is_empty(),
        "owning the constant is where the value comes from: {found:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
