//! A library target outside `src/` is read where it is, with the identities the
//! declaration macros compute.
//! `src/` 之外的库目标就地读取，身份与声明宏计算的一致。

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use nichlink::NodeId;
use nichlink_build_method::{check_for, face_views};

/// The package name every fixture uses. `check_for` pins the identity namespace
/// in a process-global first-write-wins cell, so a test that ran under a different
/// name would hand the other tests ids computed in the wrong domain — the fixtures
/// therefore differ by directory, not by name.
/// 每个夹具共用的包名。`check_for` 把身份命名空间钉在一个进程级先到先得的格子里，因此用别的名字
/// 运行的测试会给其他测试算出错误域里的 id——所以夹具按目录区分，而不按名字。
const NAME: &str = "outside-src";

/// A throwaway package whose library target is `host/lib.rs`, with one face next
/// to it, laid out the way the discovery walk reads a tree.
/// 库目标是 `host/lib.rs` 的一次性包，旁边有一个注册面，布局按发现遍历的读法。
fn package(label: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("nichlink-outside-src-{label}-{sequence}"));
    let _ = std::fs::remove_dir_all(&root);
    let face = root.join("host/control/control.rs");
    std::fs::create_dir_all(face.parent().expect("face directory")).expect("package directory");
    std::fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"{NAME}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
             [lib]\npath = \"host/lib.rs\"\n"
        ),
    )
    .expect("manifest");
    std::fs::write(root.join("host/lib.rs"), "// host entry\n").expect("library target");
    std::fs::write(
        &face,
        "crate::root_object! {\n    kind: Control,\n    needs_registry: true,\n    \
         parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
    )
    .expect("face");
    root
}

/// The generated tree carries the identity a host would compile: the file is read
/// from `host/`, and its identity path is `host/control/control.rs` — the manifest-
/// relative path, because `manifest_relative_source` only drops a leading `src/`.
/// The second assertion is the one that fails if the walk keeps the historical
/// root: `control/control.rs` would be an identity no face of this package has.
/// 生成树携带宿主会编译出的身份：文件从 `host/` 读入，其身份路径是 `host/control/control.rs`
/// ——相对清单的路径，因为 `manifest_relative_source` 只去掉一个前导 `src/`。第二条断言在遍历
/// 沿用历史根时会失败：`control/control.rs` 是这个包里没有任何面持有的身份。
#[test]
fn a_library_target_outside_src_keeps_its_identity_prefix() {
    let root = package("outside-src");
    let out_dir = root.join("target/nichlink/out");
    check_for(&root, &out_dir, NAME).expect("the layout is supported");

    let generated = std::fs::read_to_string(out_dir.join("generated_lib.rs")).expect("generated");
    // The generated tree carries identities as raw byte arrays
    // (`NodeId::from_raw([…])`), which is the same 16 bytes `Display` renders as
    // hex — so this asserts the identity itself, not its spelling.
    // 生成树以原始字节数组携带身份（`NodeId::from_raw([…])`），也就是 `Display` 渲染成十六进制的
    // 那 16 个字节——因此这里断言的是身份本身，而不是它的拼法。
    let expected = NodeId::from_namespaced_path(NAME, "host/control/control.rs", "Control");
    let wrong = NodeId::from_namespaced_path(NAME, "control/control.rs", "Control");
    assert_ne!(expected, wrong, "the two layouts must not collide");
    assert!(
        generated.contains(&format!("from_raw({:?})", expected.into_bytes())),
        "the generated tree must carry {expected}: {generated}"
    );
    assert!(
        !generated.contains(&format!("from_raw({:?})", wrong.into_bytes())),
        "the src-relative identity {wrong} belongs to no face of this package"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The read-only view agrees with the build: same face, same identity, same source
/// spelling. `explain` and the MCP registry tool both report what this returns, so
/// a disagreement here is what would put an editor and a build in different
/// identity domains.
/// 只读视图与构建一致：同一个面、同一个身份、同一种源码写法。`explain` 与 MCP 注册树工具报告的
/// 都是它返回的东西，因此这里的不一致正是会让编辑器与构建处在不同身份域的原因。
#[test]
fn the_face_view_reports_the_same_identity_as_the_build() {
    let root = package("outside-src-view");
    let faces = face_views(&root, NAME).expect("the view reads the layout");
    assert_eq!(faces.len(), 1, "{faces:?}");
    assert_eq!(faces[0].source, "host/control/control.rs");
    assert_eq!(faces[0].path, "root/control");
    assert_eq!(
        faces[0].id,
        NodeId::from_namespaced_path(NAME, "host/control/control.rs", "Control")
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A `[lib] path` that names no file is refused by name, and the tree it looked
/// for is not guessed: the verify step on the narrow manifest read.
/// 指不到文件的 `[lib] path` 被按名字拒绝，也不去猜它找的是哪棵树：窄读清单的校验步。
#[test]
fn a_library_target_that_is_not_a_file_is_refused_by_name() {
    let root = package("outside-src-missing");
    std::fs::remove_file(root.join("host/lib.rs")).expect("remove the target");
    let out_dir = root.join("target/nichlink/out");
    let diagnostics =
        check_for(&root, &out_dir, NAME).expect_err("a target that is not a file is refused");
    let rendered = diagnostics.render();
    assert!(rendered.contains("face-layout"), "{rendered}");
    assert!(rendered.contains("host/lib.rs"), "{rendered}");
    assert!(rendered.contains("is not a file"), "{rendered}");
    let _ = std::fs::remove_dir_all(&root);
}
