//! Pins for name resolution: an unknown name has to say where the tree's root is, and the
//! name the refusal points at has to be one that actually resolves.
//! 名字解析的钉子：未知的名字必须说出这棵树的根在哪，而拒绝文案点名的那个名字必须真的解析得到。

use std::path::PathBuf;

use super::resolve_node;

/// A throwaway package with no faces: a manifest Cargo can name and a library target.
/// 一个没有注册面的一次性包：Cargo 能给它命名的清单，以及一个库目标。
fn package(label: &str) -> (PathBuf, String) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("mcp-resolve-{label}");
    let root = std::env::temp_dir().join(format!("{name}-{}-{sequence}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("package directory");
    std::fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
    )
    .expect("manifest");
    std::fs::write(root.join("src/lib.rs"), "// host entry\n").expect("library target");
    (root, name)
}

/// An unknown name says where the root is and how many faces the tree derives. The bare
/// "no registration face" it used to answer with left a fresh agent guessing: the round-13
/// benchmark measured five calls spent on `control`, `control::`, `crate`, `control::control`
/// and an empty value before it found `root`.
/// 未知的名字要说出根在哪、这棵树推导出几个面。它过去回的那句干巴巴的 "no registration face"
/// 把一个新代理留在猜里：第十三轮量到它在 `control`、`control::`、`crate`、`control::control`
/// 与空值上花了五次才摸到 `root`。
#[test]
fn an_unknown_name_names_the_trees_root() {
    let (root, namespace) = package("unknown");
    let refused =
        resolve_node(&root, &namespace, "control").expect_err("`control` is not a face here");
    assert!(
        refused.contains("no registration face at `control`")
            && refused.contains("this tree's root is `root`")
            && refused.contains("face(s)")
            && refused.contains("`registry`"),
        "the refusal names the root and the way forward: {refused}"
    );
    // The reverse direction: the bare sentence the fix replaced must not come back on its
    // own — a message that stops at "no such face" is the defect, not just a short message.
    // 反向：被修掉的那句干话不许单独回来——停在"没有这个面"的消息是缺陷本身，而不只是话说短了。
    assert!(
        refused.trim() != "no registration face at `control`",
        "the refusal grew a way forward rather than staying bare: {refused}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The name the refusal points at resolves. A way forward that does not work is worse than
/// none, so this pin calls it rather than reading it.
/// 拒绝文案点名的那个名字真的解析得到。一条走不通的"继续走的路"比没有更糟，因此这枚钉子去**调**它，
/// 而不是读它。
#[test]
fn the_root_the_refusal_names_is_the_one_that_resolves() {
    let (root, namespace) = package("root-resolves");
    let resolved =
        resolve_node(&root, &namespace, "root").expect("the root the refusal names resolves");
    assert_eq!(resolved, xirang_kernel::root_node_id(&namespace));
    let _ = std::fs::remove_dir_all(&root);
}
