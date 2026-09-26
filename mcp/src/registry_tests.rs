//! Tests for the registry tool: the derivation it reports, and the namespace it
//! derives it under.
//! 注册树工具的测试：它报告的那份推导，以及推导所用的命名空间。

use std::path::PathBuf;

use nichlink::NodeId;

use super::{namespace_from, registry};

/// The shape `face_views` recognises as a root face; the parent is omitted
/// deliberately, which means "the package root".
/// `face_views` 认作根面的形状；此处有意省略父级，含义就是"包根"。
const FACE: &str = "crate::root_object! {\n    kind: Control,\n    needs_registry: true,\n}\n";

/// A throwaway package with exactly one registration face, laid out the way the
/// discovery walk reads: a face lives in `<name>/<name>.rs` under `src/`, and a
/// loose `.rs` file at the top of `src/` would be an unplaced face instead.
/// 只含一个注册面的一次性包，布局按发现遍历的读法：面住在 `src/` 下的 `<name>/<name>.rs`，
/// 而 `src/` 顶层的散装 `.rs` 文件会成为无法安放的面。
fn fixture(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "nichlink-mcp-registry-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let face = root.join("src/control/control.rs");
    std::fs::create_dir_all(face.parent().expect("face directory")).expect("fixture directories");
    std::fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{label}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
    )
    .expect("manifest");
    std::fs::write(root.join("src/lib.rs"), "// host entry\n").expect("entry");
    std::fs::write(&face, FACE).expect("face");
    root
}

/// The rows carry the identity the host compiled, so the namespace has to be
/// the package's own name — Cargo's answer, not the documented default. The id
/// assertion is the load-bearing half: it is a hash over
/// `(namespace, "control/control.rs", "Control")`, so a tool that answered under
/// `nichlink.default` would report an id nothing in the package holds.
/// 行携带的是宿主编译出的身份，因此命名空间必须是这个包自己的名字——Cargo 的答案，而不是
/// 文档化的默认值。id 断言是承重的那一半：它是对
/// `(namespace, "control/control.rs", "Control")` 的散列，因此若工具在 `nichlink.default`
/// 之下作答，报告的 id 就是包里没有任何东西持有的。
#[test]
fn a_face_is_reported_under_the_namespace_the_package_compiled_with() {
    let root = fixture("fixture-host");
    let report = registry(&root).expect("the tree is derived");
    assert!(report.starts_with("namespace fixture-host\n"), "{report}");
    assert!(report.contains("faces 1\n"), "{report}");
    assert!(report.contains("root/control"), "{report}");
    assert!(report.contains("control/control.rs"), "{report}");
    let expected = NodeId::from_namespaced_path("fixture-host", "control/control.rs", "Control");
    assert!(
        report.contains(&expected.to_string()),
        "the row must carry the compiled identity {expected}: {report}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The configured override wins verbatim, and it does so *before* Cargo is
/// asked: a directory with no manifest still answers when the namespace is
/// given, which is what makes the override usable for a package Cargo cannot
/// name.
/// 配置的覆盖原样胜出，而且它**先于**询问 Cargo：只要给出了命名空间，连没有清单的目录也能
/// 作答——正是这一点让覆盖对 Cargo 说不出的包也可用。
#[test]
fn a_configured_namespace_wins_verbatim_before_cargo_is_asked() {
    let bare =
        std::env::temp_dir().join(format!("nichlink-mcp-registry-bare-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&bare);
    std::fs::create_dir_all(&bare).expect("bare directory");
    assert_eq!(
        namespace_from(Some("given-by-the-user"), &bare).expect("the override answers"),
        "given-by-the-user"
    );
    let _ = std::fs::remove_dir_all(&bare);
}

/// A package Cargo cannot name is refused, and the refusal names the way out.
/// This is the asymmetry with authoring: a query about an existing tree must not
/// invent the identity domain, because every id below it would be wrong.
/// Cargo 说不出的包会被拒绝，而且拒绝点名了出路。这正是与创作侧的不对称：针对已存在树的
/// 查询不能凭空造出身份域，因为其下每个 id 都会是错的。
#[test]
fn a_directory_without_a_package_is_refused_with_the_way_out() {
    let bare = std::env::temp_dir().join(format!(
        "nichlink-mcp-registry-nameless-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&bare);
    std::fs::create_dir_all(&bare).expect("bare directory");
    let error = namespace_from(None, &bare).expect_err("no manifest means no namespace");
    assert!(error.contains("identity namespace"), "{error}");
    assert!(error.contains("NICH_LINK_NAMESPACE"), "{error}");
    let _ = std::fs::remove_dir_all(&bare);
}
