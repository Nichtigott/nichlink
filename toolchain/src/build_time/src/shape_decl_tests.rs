//! Pins for reading a host's crate-shape declaration: it resolves, it publishes the lock, and every
//! way of writing something else is refused by name (audit `M7`, P3.1).
//! 读宿主 crate 形状声明的钉子：它解析、发布锁，而其它任何写法都被点名拒绝（审计 `M7`，P3.1）。

use std::fs;
use std::path::PathBuf;

use super::{read_shape_declaration, write_shape_lock};

/// A throwaway package with a declaration and a two-face tree under it.
/// 一个一次性包：一份声明，以及声明之下两个面的树。
fn package(label: &str, declaration: &str) -> (PathBuf, PathBuf, PathBuf) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "nichlink-shape-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    let src = root.join("src");
    let out = root.join("out");
    fs::create_dir_all(src.join("control/object/button")).expect("subtree directory");
    fs::write(
        src.join("control/control.rs"),
        "crate::root_object! {\n    kind: Control,\n}\n",
    )
    .expect("the root face");
    fs::write(
        src.join("control/object/button/button.rs"),
        "crate::control_object! {\n    kind: Button,\n    parent: crate::control::NODE_ID,\n}\n",
    )
    .expect("the leaf face");
    fs::create_dir_all(src.join("panel/gauge")).expect("a container face's subtree");
    fs::write(
        src.join("panel/panel.rs"),
        "crate::root_object! {\n    kind: Panel,\n}\n",
    )
    .expect("the container face");
    fs::write(
        src.join("panel/gauge/gauge.rs"),
        "crate::control_object! {\n    kind: Gauge,\n    parent: crate::panel::NODE_ID,\n}\n",
    )
    .expect("the gauge face");
    fs::create_dir_all(src.join("control/registry_rule")).expect("second subtree directory");
    fs::write(
        src.join("control/registry_rule/registry_rule.rs"),
        "crate::control_object! {\n    kind: RegistryRule,\n    parent: crate::control::NODE_ID,\n}\n",
    )
    .expect("the rule face");
    fs::create_dir_all(&out).expect("out directory");
    fs::write(root.join("add_crates.rs"), declaration).expect("the declaration");
    (root, src, out)
}

/// The declaration a host writes, in the shape the reader accepts.
/// 宿主写下的声明，读取器接受的那种形状。
const DECLARATION: &str = r#"//! The shape.
use nichlink_toolchain::runtime::{Crate, Shape};

pub const SHAPE: Shape = Shape {
    package_prefix: "myapp",
    crates: &[
        Crate::named("widgets").at(&[crate::control::object::SUBTREE]),
        Crate::named("panel").at(&[crate::panel::SUBTREE]),
    ],
};
"#;

/// A package with no declaration is the one-crate package, not a missing input.
/// 没有声明的包就是一个 crate 的包，而不是缺了输入。
#[test]
fn a_package_without_a_declaration_answers_none() {
    let (root, _, _) = package("absent", DECLARATION);
    fs::remove_file(root.join("add_crates.rs")).expect("remove the declaration");
    assert!(
        read_shape_declaration(&root)
            .expect("no declaration")
            .is_none()
    );
}

/// The declaration resolves to the crates it names, and the digest follows the file's bytes.
/// 声明解析成它点名的那些 crate，而摘要跟着文件的字节走。
#[test]
fn a_declaration_resolves_to_the_crates_it_names() {
    let (root, _, _) = package("reads", DECLARATION);
    let declaration = read_shape_declaration(&root)
        .expect("it reads")
        .expect("it declares a shape");
    assert_eq!(declaration.package_prefix, "myapp");
    assert_eq!(
        declaration.crates,
        vec![
            ("widgets".to_owned(), vec!["control::object".to_owned()]),
            ("panel".to_owned(), vec!["panel".to_owned()]),
        ]
    );
    // The digest is the identity `NodeId` derives from the file's bytes (128 bits, 32 hex digits),
    // not a second hash of our own: the shape records the same kind of value everything else does.
    // 摘要就是 `NodeId` 从文件字节推导出的身份（128 位、32 个十六进制位），不是我们自己另算的第二种散列：
    // 形状记录的是与其它一切同类的值。
    assert_eq!(declaration.digest.len(), 32);
}

/// The lock answers which faces each crate would own, and leaves the rest with the host.
/// 锁回答每个 crate 会拥有哪些面，其余的留在宿主。
#[test]
fn the_lock_says_which_faces_each_crate_would_own() {
    let (root, src, out) = package("lock", DECLARATION);
    let declaration = read_shape_declaration(&root)
        .expect("it reads")
        .expect("it declares a shape");
    let nodes = crate::build_time::source_walk::discover_root(&src);
    let rows = crate::build_time::manifests::write_pruning_manifest(&src, &nodes, &out)
        .expect("the record writes");
    write_shape_lock(&out, &declaration, &rows).expect("the lock writes");
    let lock = fs::read_to_string(out.join(nichlink_kernel::lexicon::ADD_CRATES_LOCK_FILE))
        .expect("the lock reads");
    assert!(
        lock.starts_with("# add-crates\tnichlink-crate-shape\n"),
        "{lock}"
    );
    assert!(lock.contains("package_prefix\tmyapp"), "{lock}");
    assert!(
        lock.contains("crate\twidgets\tsubtrees=control::object\tfaces=1"),
        "{lock}"
    );
    // The claimed node is a **container face**: its own face goes with the crate, and so does the one
    // below it.
    // 被认领的节点是一个**容器面**：它自己的面跟着 crate 走，它下面那个面也是。
    assert!(
        lock.contains("crate\tpanel\tsubtrees=panel\tfaces=2"),
        "{lock}"
    );
    // The host keeps the two faces no crate claimed: the root face and the rule face.
    // 宿主留着没有 crate 认领的那两个面：根面与规则面。
    assert!(lock.contains("host\tfaces=2"), "{lock}");
}

/// A second spelling of the same declaration is refused, because guessing is how a shape is decided
/// by accident.
/// 同一份声明的第二种拼写会被拒绝，因为猜测正是形状被意外决定的方式。
#[test]
fn another_spelling_is_refused_by_name() {
    let (root, _, _) = package(
        "spelling",
        "pub const SHAPE: Shape = Shape { package_prefix: \"myapp\", crates: &[\n\
         Crate::named(\"widgets\").at(&[crate::control::object::REGISTRATION]),\n] };\n",
    );
    let refusal = read_shape_declaration(&root).expect_err("refused");
    assert!(refusal.contains("::SUBTREE"), "{refusal}");
    assert!(refusal.contains("add_crates.rs"), "{refusal}");
}

/// A crate with no subtree, an overlapping pair and a missing prefix are three different refusals.
/// 没有子树的 crate、重叠的一对、以及缺失的前缀，是三种不同的拒绝。
#[test]
fn the_empty_and_overlapping_shapes_are_refused_where_they_are_wrong() {
    let (no_subtree, _, _) = package(
        "nosubtree",
        "pub const SHAPE: Shape = Shape { package_prefix: \"myapp\", crates: &[\n\
         Crate::named(\"widgets\"),\n] };\n",
    );
    assert!(
        read_shape_declaration(&no_subtree)
            .expect_err("nothing claimed")
            .contains("claims nothing")
    );

    let (prefix, _, _) = package(
        "prefix",
        "pub const SHAPE: Shape = Shape { crates: &[\n\
         Crate::named(\"widgets\").at(&[crate::control::object::SUBTREE]),\n] };\n",
    );
    assert!(
        read_shape_declaration(&prefix)
            .expect_err("no prefix")
            .contains("package_prefix")
    );

    let (overlap, _, _) = package(
        "overlap",
        "pub const SHAPE: Shape = Shape { package_prefix: \"myapp\", crates: &[\n\
         Crate::named(\"widgets\").at(&[crate::control::object::SUBTREE]),\n\
         Crate::named(\"tiny\").at(&[crate::control::object::button::SUBTREE]),\n] };\n",
    );
    let refusal = read_shape_declaration(&overlap).expect_err("overlap");
    assert!(refusal.contains("`widgets`"), "{refusal}");
    assert!(refusal.contains("`tiny`"), "{refusal}");
    assert!(refusal.contains("exactly one crate"), "{refusal}");
}

/// The rule the reader applies is the kernel's, so what the build refuses is what the host's own
/// crate refuses at load: one rule, two readers.
/// 读取器适用的规则是内核的，因此构建拒绝的东西与宿主自己的 crate 在装载时拒绝的是同一样东西：一条规则，
/// 两个读者。
#[test]
fn the_build_and_the_host_apply_one_rule() {
    let (root, _, _) = package(
        "onerule",
        "pub const SHAPE: Shape = Shape { package_prefix: \"myapp\", crates: &[\n\
         Crate::named(\"widgets\").at(&[crate::control::object::SUBTREE]),\n\
         Crate::named(\"widgets\").at(&[crate::control::registry_rule::SUBTREE]),\n] };\n",
    );
    assert!(
        read_shape_declaration(&root)
            .expect_err("declared twice")
            .contains("declared twice")
    );
}

/// A claim on a **face** is refused, and the refusal names the node that does have a subtree.
/// 认领一个**面**会被拒绝，而拒绝点名那个真正拥有子树的节点。
///
/// The compiler refuses this one too (a face has no `SUBTREE` marker); this is the reader saying the
/// same rule for a declaration no crate mounts — and, because it reads the tree, it can name the way
/// forward instead of leaving the author to guess.
/// 编译器也会拒它（面没有 `SUBTREE` 标记）；这里是读取器对"没有 crate 挂载的声明"说同一条规则——而且因为它
/// 读得到树，它能点名出路，而不是让作者去猜。
#[test]
fn a_claim_on_a_face_names_the_node_that_has_a_subtree() {
    let (root, src, out) = package(
        "leafclaim",
        "pub const SHAPE: Shape = Shape { package_prefix: \"myapp\", crates: &[\n\
         Crate::named(\"button\").at(&[crate::control::object::button::SUBTREE]),\n] };\n",
    );
    let nodes = crate::build_time::source_walk::discover_root(&src);
    let rows = crate::build_time::manifests::write_pruning_manifest(&src, &nodes, &out)
        .expect("the record writes");
    let refusal = super::check_shape_declaration(&root, &out, &rows).expect_err("refused");
    assert!(refusal.contains("is a face and not a subtree"), "{refusal}");
    assert!(refusal.contains("control::object"), "{refusal}");
}

/// A claim on a node with no faces below it is refused as an empty crate.
/// 认领一个下面没有面的节点，被拒为空 crate。
#[test]
fn a_claim_with_no_faces_below_it_is_an_empty_crate() {
    let (root, src, out) = package(
        "emptyclaim",
        "pub const SHAPE: Shape = Shape { package_prefix: \"myapp\", crates: &[\n\
         Crate::named(\"nothing\").at(&[crate::nowhere::SUBTREE]),\n] };\n",
    );
    let nodes = crate::build_time::source_walk::discover_root(&src);
    let rows = crate::build_time::manifests::write_pruning_manifest(&src, &nodes, &out)
        .expect("the record writes");
    let refusal = super::check_shape_declaration(&root, &out, &rows).expect_err("refused");
    assert!(refusal.contains("no faces below it"), "{refusal}");
    assert!(refusal.contains("would be empty"), "{refusal}");
}
