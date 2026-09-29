//! Tests for the read-only face view: the paths and identities it derives, and
//! the registration files it cannot read.
//! 只读注册面视图的测试：它推导出的路径与身份，以及它读不了的注册面文件。

use super::{
    face_views, face_views_and_unreadable, face_views_with_external, read_build_scope,
    read_pruning_manifest,
};
use std::fs;
use std::path::PathBuf;

/// A throwaway package root; the caller writes `src/` files into it.
/// 一次性的包根；调用方往其中写 `src/` 文件。
fn temporary_root(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "nichlink-face-view-{label}-{}-{}-{sequence}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::create_dir_all(root.join("src")).expect("fixture src");
    root
}

/// The logical path and the registry name follow the chain a runtime tree
/// would build, and the default registry name is the module's last segment.
/// 逻辑路径与 registry 名跟随运行期树会构建的链，默认注册面名取模块末段。
#[test]
fn faces_carry_their_logical_path_and_default_registry_name() {
    let root = temporary_root("paths");
    let panel = root.join("src/control/object");
    fs::create_dir_all(&panel).expect("panel directory");
    fs::write(
        panel.join("object.rs"),
        "crate::root_object! {\n    kind: Control,\n    needs_registry: true,\n}\n",
    )
    .expect("panel face");
    let button = root.join("src/control/object/button");
    fs::create_dir_all(&button).expect("button directory");
    fs::write(
            button.join("button.rs"),
            "crate::root_object! {\n    kind: Button,\n    parent: crate::control::object::NODE_ID,\n}\n",
        )
        .expect("button face");

    let faces = face_views(&root, "host").expect("faces");
    assert_eq!(faces.len(), 2);
    let control = faces
        .iter()
        .find(|face| face.kind == "Control")
        .expect("control face");
    assert_eq!(control.path, "root/object");
    assert!(control.owns_registry);
    assert!(control.parent_resolved);
    let button = faces
        .iter()
        .find(|face| face.kind == "Button")
        .expect("button face");
    assert_eq!(button.registry_name, "button");
    assert!(button.parent_resolved);
    assert!(
        button.id
            == nichlink_kernel::identity::NodeId::from_namespaced_path(
                "host",
                "control/object/button/button.rs",
                "Button"
            ),
        "identity must be the namespace + relative path + kind hash"
    );

    fs::remove_dir_all(&root).expect("cleanup");
}

/// A parent the tree cannot resolve keeps the face visible and says so,
/// rather than dropping it or guessing a different parent.
/// 树解析不出的父级仍让该面可见并如实说明，而不是丢掉它或改猜另一个父级。
#[test]
fn an_unresolved_parent_is_reported_not_guessed() {
    let root = temporary_root("unresolved-parent");
    let folder = root.join("src/child");
    fs::create_dir_all(&folder).expect("child directory");
    fs::write(
        folder.join("child.rs"),
        "crate::root_object! {\n    kind: Child,\n    parent: crate::missing::NODE_ID,\n}\n",
    )
    .expect("child face");

    let faces = face_views(&root, "host").expect("faces");
    assert_eq!(faces.len(), 1);
    assert!(!faces[0].parent_resolved);

    fs::remove_dir_all(&root).expect("cleanup");
}

/// The two manifest readers agree with what the writers in `manifests.rs`
/// emit, including the `# result all` spelling for a whole-tree scope.
/// 两个清单读取方与 `manifests.rs` 的写入方一致，包括整树作用域的 `# result all`
/// 写法。
#[test]
fn manifests_round_trip_through_the_readers() {
    let root = temporary_root("manifests");
    let out = root.join("out");
    fs::create_dir_all(&out).expect("out dir");
    fs::write(
            out.join("source_scope.tsv"),
            "# mode\tauto\n# selected\t1\n# node\tsource\tmodule\nabc\tcontrol/button.rs\tcontrol::button\n",
        )
        .expect("scope manifest");
    let scope = read_build_scope(&out).expect("scope");
    assert_eq!(scope.mode, "auto");
    assert!(!scope.all);
    assert_eq!(scope.selected_sources.len(), 1);
    assert!(scope.selected_sources.contains("control/button.rs"));
    assert!(scope.keeps("control::button"), "a selected face is kept");
    assert!(
        scope.keeps("control"),
        "a selected face keeps its parent registry"
    );
    assert!(
        !scope.keeps("control_extra"),
        "the boundary is the `::` segment, not a text prefix"
    );

    fs::write(
        out.join("source_scope.tsv"),
        "# mode\texplicit\n# result\tall\n# selected\tall\n# reason\tscope-all\n",
    )
    .expect("all scope");
    let scope = read_build_scope(&out).expect("all scope");
    assert!(scope.all);
    assert_eq!(scope.reason.as_deref(), Some("scope-all"));

    fs::write(
        out.join("pruning_manifest.tsv"),
        "# node\tsource\tsymbol\n00000000000000000000000000000000\tcontrol/button.rs\t-\n",
    )
    .expect("pruning manifest");
    let rows = read_pruning_manifest(&out).expect("pruning rows");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].source, "control/button.rs");
    assert_eq!(rows[0].symbol, "-");

    assert!(read_build_scope(&root.join("missing")).is_err());
    fs::remove_dir_all(&root).expect("cleanup");
}
/// The path is resolved from the parent chain, not from the order the walk
/// happened to visit the files in.
/// 路径由父级链解出，而不是由遍历恰好访问文件的顺序决定。
///
/// `alpha` declares a parent whose directory sorts *after* it, so a walk that
/// answered "root/<own name>" whenever the parent had not been seen yet gave
/// `root/alpha` — a path the runtime tree never had — and froze it in the
/// cache. This is the one case the older test skipped: it asserted the parent's
/// path and left the child's unasserted.
/// `alpha` 声明的父级的目录名排在它**之后**，因此"父级还没见过就用 `root/<自己的名字>`"
/// 的遍历会给出 `root/alpha`——一条运行期的树从未有过的路径——并把它冻进缓存。这正是旧测试绕过
/// 的那一格：它断言了父面的路径，却没断言子面的。
#[test]
fn a_parent_that_sorts_later_still_forms_the_full_path() {
    let root = temporary_root("order");
    let zeta = root.join("src/zeta");
    fs::create_dir_all(&zeta).expect("zeta directory");
    fs::write(
        zeta.join("zeta.rs"),
        "crate::root_object! {\n    kind: Zeta,\n    needs_registry: true,\n}\n",
    )
    .expect("zeta face");
    let alpha = root.join("src/alpha");
    fs::create_dir_all(&alpha).expect("alpha directory");
    fs::write(
        alpha.join("alpha.rs"),
        "crate::root_object! {\n    kind: Alpha,\n    parent: crate::zeta::NODE_ID,\n}\n",
    )
    .expect("alpha face");

    let faces = face_views(&root, "host").expect("faces");
    let path_of = |kind: &str| {
        faces
            .iter()
            .find(|face| face.kind == kind)
            .map(|face| face.path.clone())
            .unwrap_or_else(|| panic!("no {kind} face in {faces:?}"))
    };
    assert_eq!(path_of("Zeta"), "root/zeta");
    assert_eq!(path_of("Alpha"), "root/zeta/alpha");
    let alpha = faces
        .iter()
        .find(|face| face.kind == "Alpha")
        .expect("alpha face");
    assert!(alpha.parent_resolved, "the parent is in the tree");
    let _ = fs::remove_dir_all(&root);
}

/// A registration file that does not parse is named instead of dropped in
/// silence, and its child is reported with a broken chain.
/// 解析不了的注册面文件被点名，而不是静默丢弃；它的子面带断链出现。
///
/// The first half of the answer is unchanged — the file is not a face, so it
/// cannot be listed as one — and that is exactly why the second half has to
/// exist: without it the caller answers "this package declares these two faces"
/// with no count and no reason, while the build reports the same file as a
/// `face-syntax` diagnostic.
/// 答案的前半段不变——该文件不是面，因此不能被列成面——而这正是后半段必须存在的原因：没有它，
/// 调用方会回答"这个包声明了这两个面"，既没有计数也没有原因，而构建把同一个文件报成
/// `face-syntax` 诊断。
#[test]
fn a_registration_file_that_does_not_parse_is_named_not_dropped() {
    let root = temporary_root("unparsable");
    let control = root.join("src/control");
    fs::create_dir_all(&control).expect("control directory");
    fs::write(
        control.join("control.rs"),
        "crate::root_object! {\n    kind: Broken\n",
    )
    .expect("broken face");
    let button = root.join("src/button");
    fs::create_dir_all(&button).expect("button directory");
    fs::write(
        button.join("button.rs"),
        "crate::root_object! {\n    kind: Button,\n    parent: crate::control::NODE_ID,\n}\n",
    )
    .expect("button face");

    let (faces, unreadable) = face_views_and_unreadable(&root, "host").expect("faces");
    assert_eq!(faces.len(), 1, "{faces:?}");
    assert_eq!(faces[0].kind, "Button");
    assert!(
        !faces[0].parent_resolved,
        "the parent file did not parse, so the chain is broken and the reply says so"
    );
    assert_eq!(unreadable.len(), 1, "{unreadable:?}");
    assert!(
        unreadable[0].contains("control/control.rs"),
        "the file that could not be read is named: {unreadable:?}"
    );
    let _ = fs::remove_dir_all(&root);
}

/// An `external_object!` face is a face of its own registry, not an unreadable file: the
/// generated tree deliberately does not contain it, and saying "the build can never compile
/// it" about a working example was false. The two lists stay separate so a reader can tell a
/// broken crate from a deliberately out-of-tree one.
/// `external_object!` 写的面是它自己注册机的面，不是读不了的文件：生成树有意不含它，而对着一个能工作的
/// 示例说"构建永远编译不了它"是假话。两张清单保持分开，读者才能分清"坏掉的 crate"与"有意在树外的 crate"。
#[test]
fn an_external_face_is_reported_as_external_rather_than_unparsable() {
    let root = temporary_root("external");
    fs::write(
        root.join("src/button_fast.rs"),
        "nichlink_toolchain::runtime::external_object! {\n    source: \"button_fast/button_fast.rs\",\n    kind: ButtonFast,\n}\n",
    )
    .expect("external face");
    let (views, unreadable, external) =
        face_views_with_external(&root, "graft-host").expect("an answer");
    assert!(
        views.is_empty(),
        "not part of the generated tree: {views:?}"
    );
    assert!(
        unreadable.is_empty(),
        "an external face is not an unreadable one: {unreadable:?}"
    );
    assert_eq!(external.len(), 1, "{external:?}");
    assert!(external[0].contains("button_fast.rs"), "{external:?}");
    assert!(external[0].contains("external face"), "{external:?}");
    // The older entry point keeps its promise: it reports the unreadable half, and the
    // external face is not in it.
    // 旧的入口保持它的承诺：它报的是读不了的那一半，而外部面不在其中。
    let (_, only_unreadable) = face_views_and_unreadable(&root, "graft-host").expect("an answer");
    assert!(only_unreadable.is_empty(), "{only_unreadable:?}");
}

/// A generated-layout face that sits outside the layout is still a layout problem.
/// 本该在生成布局里、却长在布局之外的面仍然是布局问题。
#[test]
fn a_misplaced_generated_face_is_still_a_layout_problem() {
    let root = temporary_root("misplaced");
    fs::write(
        root.join("src/panel.rs"),
        "crate::root_object! {\n    kind: Panel,\n}\n",
    )
    .expect("misplaced face");
    let (_, unreadable, external) = face_views_with_external(&root, "host").expect("an answer");
    assert_eq!(unreadable.len(), 1, "{unreadable:?}");
    assert!(unreadable[0].contains("outside the"), "{unreadable:?}");
    assert!(external.is_empty(), "{external:?}");
}
