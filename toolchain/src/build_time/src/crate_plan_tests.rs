//! Pins for planning a host's ghost crates: what gets mounted, and what is refused by name.
//! 规划宿主幽灵 crate 的钉子：挂载什么，以及什么被点名拒绝（审计 `M7`，P3.2）。

use std::fs;
use std::path::{Path, PathBuf};

use super::{PlannedCrate, plan};
use crate::build_time::shape_decl::read_shape_declaration;

/// A throwaway host package: `src/` files as written below, plus a declaration.
/// 一个一次性宿主包：下面写下的 `src/` 文件，外加一份声明。
fn host(label: &str, files: &[(&str, &str)], declaration: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let parent = std::env::temp_dir().join(format!(
        "nichlink-plan-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&parent);
    let root = parent.join("host");
    fs::create_dir_all(root.join("src")).expect("src");
    for (relative, text) in files {
        let path = root.join("src").join(relative);
        fs::create_dir_all(path.parent().expect("a parent")).expect("face directory");
        fs::write(&path, text).expect("face");
    }
    fs::write(root.join("add_crates.rs"), declaration).expect("the declaration");
    // The ghost's manifest is built from the host's, so the fixture needs one — and its dependency
    // section is what the copy has to carry verbatim.
    // 幽灵的清单由宿主的清单生成，因此夹具需要一份——而它的依赖节正是照抄要带走的东西。
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"host\"\nversion = \"9.9.9\"\nedition = \"2024\"\n\n\
         [dependencies]\nnichlink-toolchain = { path = \"../toolchain\", features = [\"run\"] }\n\n\
         [build-dependencies]\nnichlink-toolchain = { path = \"../toolchain\" }\n",
    )
    .expect("the host manifest");
    root
}

/// Plan the declaration the host wrote, through the real rows.
/// 用真实的行规划宿主写下的声明。
fn plan_host(root: &Path) -> Result<Vec<PlannedCrate>, String> {
    let src = root.join("src");
    let out = root.join("out");
    fs::create_dir_all(&out).expect("out");
    let declaration = read_shape_declaration(root)
        .expect("it reads")
        .expect("it declares a shape");
    let nodes = crate::build_time::source_walk::discover_root(&src);
    let rows = crate::build_time::manifests::write_pruning_manifest(&src, &nodes, &out)
        .expect("the record writes");
    // The pipeline hands the planner its face list as `(source, module)` pairs, computed before the
    // render; the fixture derives the same pairs from the rows the build published.
    // 管线把面清单以 `(源码, 模块)` 对交给规划器（在渲染之前算好）；夹具从构建发布的行里推导出同样的对。
    let faces: Vec<(String, String, crate::build_time::registry_identity::NodeId)> = rows
        .iter()
        .map(|row| {
            (
                row.source.clone(),
                crate::build_time::static_plan::source_module_path(&row.source),
                row.id,
            )
        })
        .collect();
    plan(root, "myapp", &declaration, &faces)
}

/// A declaration for one crate claiming `control::object`.
/// 一份声明：一个 crate 认领 `control::object`。
const DECLARATION: &str = r#"use nichlink_toolchain::runtime::{Crate, Shape};

pub const SHAPE: Shape = Shape {
    package_prefix: "myapp",
    crates: &[Crate::named("widgets").at(&[crate::control::object::SUBTREE])],
};
"#;

/// A self-contained fragment plans into one crate, one mount, and the remap that keeps `file!()`
/// reading as the host's own source spelling.
/// 一个自足的碎片规划成一个 crate、一次挂载，以及让 `file!()` 读起来就是宿主源码拼写的那对映射。
#[test]
fn a_self_contained_fragment_plans_a_mount_and_its_remap() {
    let root = host(
        "self-contained",
        &[
            (
                "control/control.rs",
                "crate::root_object! {\n    kind: Control,\n}\n",
            ),
            (
                "control/object/button/button.rs",
                "crate::control_object! {\n    kind: Button,\n    parent: crate::root_node_id(crate::NICHLINK_NAMESPACE),\n}\n",
            ),
        ],
        DECLARATION,
    );
    let planned = plan_host(&root).expect("the fragment is self-contained");
    assert_eq!(planned.len(), 1, "one declared crate");
    let widgets = &planned[0];
    assert_eq!(widgets.name, "widgets");
    assert_eq!(widgets.package, "myapp-widgets");
    assert_eq!(
        widgets.directory,
        root.parent().expect("a parent").join("myapp-widgets"),
        "the ghost is the host's sibling"
    );
    assert_eq!(
        widgets.namespace, "myapp",
        "the ghost declares the host's namespace"
    );
    assert_eq!(
        widgets.mounts.len(),
        1,
        "only the faces below the claimed subtree: {:?}",
        widgets.mounts
    );
    let mount = &widgets.mounts[0];
    assert_eq!(mount.module_path, "control::object::button");
    assert_eq!(mount.source, "control/object/button/button.rs");

    // The invariant that makes the remap derivable rather than guessed: the spelling is exactly the
    // prefix plus the host-relative source, so remapping the prefix to nothing leaves `file!()`
    // reading as the source the host's records name.
    // 让映射**可推导**而不是靠猜的那条不变量：拼写恰好是前缀加宿主相对源码，因此把前缀映射为空之后，
    // `file!()` 读起来就是宿主记录点名的那个源码。
    assert_eq!(widgets.remap.len(), 1, "{:?}", widgets.remap);
    let (from, to) = &widgets.remap[0];
    assert_eq!(to, "", "the prefix maps to nothing");
    assert_eq!(
        mount.spelling,
        format!("{from}{}", mount.source),
        "the spelling is the prefix plus the source"
    );
    assert_eq!(
        mount.spelling, "../../../../host/src/control/object/button/button.rs",
        "four levels out: the inline directories, src, and the ghost package"
    );
    let _ = fs::remove_dir_all(root.parent().expect("a parent"));
}

/// A face that reaches outside the fragment is refused **by name**, with the line, the reason and the
/// two ways forward — because a ghost mounts only the fragment and has no module above it.
/// 够到碎片之外的引用会被**点名**拒绝，带上行号、原因与两条出路——因为幽灵只挂载碎片，它上面没有模块。
#[test]
fn a_face_reaching_outside_the_fragment_is_refused_by_name() {
    let root = host(
        "reaching-out",
        &[
            (
                "control/control.rs",
                "pub trait ControlHandle {}\n\ncrate::root_object! {\n    kind: Control,\n}\n",
            ),
            (
                "control/object/button/button.rs",
                "use crate::control::ControlHandle;\n\ncrate::control_object! {\n    kind: Button,\n}\n",
            ),
        ],
        DECLARATION,
    );
    let refused = plan_host(&root).expect_err("refused");
    assert!(
        refused.contains("control/object/button/button.rs:1"),
        "the refusal names the file and line: {refused}"
    );
    assert!(
        refused.contains("`crate::control::ControlHandle`"),
        "and the whole path that would not resolve: {refused}"
    );
    assert!(
        !refused.contains("::`,"),
        "no path is printed with a dangling `::`: {refused}"
    );
    assert!(
        refused.contains("way forward") && refused.contains("larger subtree"),
        "and the two ways forward: {refused}"
    );
    // A reference the ghost *can* resolve — its own subtree, or the constant the macros read — is not
    // a refusal, which the self-contained pin above shows from the other side.
    // 幽灵**解析得到**的引用——它自己的子树，或宏读的那个常量——不是拒绝的理由；上面那条自足的钉子从另一侧
    // 展示了这一点。
    let _ = fs::remove_dir_all(root.parent().expect("a parent"));
}

/// Two subtrees of **one** crate may reference each other: both are mounted, so both resolve.
/// **同一个** crate 的两棵子树之间可以互相引用：两棵都被挂载，因此都解析得到。
#[test]
fn two_subtrees_of_one_crate_may_reference_each_other() {
    let root = host(
        "siblings",
        &[
            (
                "control/control.rs",
                "crate::root_object! {\n    kind: Control,\n}\n",
            ),
            (
                "control/object/button/button.rs",
                "crate::control_object! {\n    kind: Button,\n    parent: crate::panel::NODE_ID,\n}\n",
            ),
            (
                "panel/panel.rs",
                "crate::root_object! {\n    kind: Panel,\n}\n",
            ),
            (
                "panel/gauge/gauge.rs",
                "crate::control_object! {\n    kind: Gauge,\n}\n",
            ),
        ],
        r#"use nichlink_toolchain::runtime::{Crate, Shape};

pub const SHAPE: Shape = Shape {
    package_prefix: "myapp",
    crates: &[Crate::named("widgets").at(&[
        crate::control::object::SUBTREE,
        crate::panel::SUBTREE,
    ])],
};
"#,
    );
    let planned = plan_host(&root).expect("a reference between two claimed subtrees resolves");
    assert_eq!(planned.len(), 1);
    // Three, not two: a claim names a subtree **rooted at** its node, so the face at that node is
    // part of it — `panel/panel.rs` belongs to the `panel` claim exactly as the button belongs to
    // `control::object`.
    // 三个而不是两个：认领点名的是**以该节点为根**的子树，因此该节点自己的面也属于它——`panel/panel.rs` 属于
    // `panel` 这条认领，正如 button 属于 `control::object`。
    assert_eq!(
        planned[0].mounts.len(),
        3,
        "both subtrees are mounted, each including its root face: {:?}",
        planned[0].mounts
    );
    // Two inline depths ⇒ two distinct prefixes, each mapping to nothing.
    // 两个不同的内联深度 ⇒ 两个不同的前缀，各自映射为空。
    // One prefix per inline depth, and the depths differ by construction: `control::object::button`
    // sits three modules deep, `panel` one, `panel::gauge` two.
    // 每个内联深度一个前缀，而深度按构造各不相同：`control::object::button` 深三层、`panel` 一层、
    // `panel::gauge` 两层。
    assert_eq!(planned[0].remap.len(), 3, "{:?}", planned[0].remap);
    assert!(
        planned[0]
            .remap
            .iter()
            .all(|(from, to)| from.ends_with("host/src/") && to.is_empty()),
        "every remap walks out of the ghost into the host's src: {:?}",
        planned[0].remap
    );
    let _ = fs::remove_dir_all(root.parent().expect("a parent"));
}

/// The two render modes read one shape differently: a host hands its claims away, a ghost is the crate
/// they were handed **to** and therefore hands nothing away (the end-to-end demonstration is what
/// found this — the plan was right and the render was not).
/// 两种渲染模式对同一份形状的读法不同：宿主把认领交出去，幽灵是它们被交给的 crate、因此什么都不交出去
/// （端到端演示发现了这件事——规划是对的，渲染不是）。
#[test]
fn a_host_hands_its_claims_away_and_a_ghost_hands_nothing_away() {
    let root = host(
        "modes",
        &[
            (
                "control/control.rs",
                "crate::root_object! {\n    kind: Control,\n}\n",
            ),
            (
                "control/object/button/button.rs",
                "crate::control_object! {\n    kind: Button,\n}\n",
            ),
        ],
        DECLARATION,
    );
    let declaration = read_shape_declaration(&root)
        .expect("it reads")
        .expect("it declares a shape");
    assert_eq!(
        super::cut_out_for(Some(&declaration), true),
        Vec::<String>::new(),
        "a ghost renders exactly the fragment it was handed"
    );
    assert_eq!(
        super::cut_out_for(Some(&declaration), false),
        vec!["control::object".to_owned()],
        "a host renders everything but the subtrees it hands away"
    );
    assert_eq!(
        super::cut_out_for(None, false),
        Vec::<String>::new(),
        "no declaration: the whole tree"
    );
    let _ = fs::remove_dir_all(root.parent().expect("a parent"));
}

/// A fragment may name its **ancestor's identity** — the shell carries that one constant so
/// `parent: crate::panel::NODE_ID` still resolves in a crate that does not compile `panel` — and may
/// name nothing else of it: a trait defined above the fragment stays a refusal, which is the honest
/// boundary (audit `M7`, P3.2).
/// 碎片可以点名**祖先的身份**——壳携带那一个常量，好让 `parent: crate::panel::NODE_ID` 在一个不编译 `panel`
/// 的 crate 里仍然解析——而祖先的其它东西一概不能点名：定义在碎片之上的 trait 仍被拒绝，那是诚实的边界
/// （审计 `M7`，P3.2）。
#[test]
fn an_ancestors_identity_is_reachable_and_its_traits_are_not() {
    let declaration = r#"use nichlink_toolchain::runtime::{Crate, Shape};

pub const SHAPE: Shape = Shape {
    package_prefix: "myapp",
    crates: &[Crate::named("widgets").at(&[crate::panel::frame::SUBTREE])],
};
"#;
    let files = [
        (
            "panel/panel.rs",
            "crate::root_object! {\n    kind: Panel,\n}\n",
        ),
        (
            "panel/frame/frame.rs",
            "crate::control_object! {\n    kind: Frame,\n    parent: crate::panel::NODE_ID,\n}\n",
        ),
        (
            "panel/frame/widget/widget.rs",
            "crate::control_object! {\n    kind: Widget,\n    parent: crate::panel::frame::NODE_ID,\n}\n",
        ),
    ];
    let root = host("ancestor-identity", &files, declaration);
    let planned = plan_host(&root).expect("an ancestor's identity is reachable");
    assert_eq!(planned.len(), 1);
    assert_eq!(
        planned[0].ancestors.len(),
        1,
        "exactly the faces above the claim: {:?}",
        planned[0].ancestors
    );
    assert_eq!(planned[0].ancestors[0].0, "panel");
    assert_eq!(
        planned[0]
            .mounts
            .iter()
            .map(|mount| mount.module_path.as_str())
            .collect::<Vec<_>>(),
        vec!["panel::frame", "panel::frame::widget"],
        "the fragment includes the face **at** its root, which the host hands away whole"
    );

    // The same fragment reaching for a **trait** above it is refused, by file and line.
    // 同一个碎片若去够它上面的**trait**，则被点名拒绝（文件与行）。
    let root = host(
        "ancestor-trait",
        &[
            (
                "panel/panel.rs",
                "pub trait ControlHandle {}\n\ncrate::root_object! {\n    kind: Panel,\n}\n",
            ),
            (
                "panel/frame/frame.rs",
                "use crate::panel::ControlHandle;\n\ncrate::control_object! {\n    kind: Frame,\n}\n",
            ),
            (
                "panel/frame/widget/widget.rs",
                "crate::control_object! {\n    kind: Widget,\n}\n",
            ),
        ],
        declaration,
    );
    let refused = plan_host(&root).expect_err("a trait above the fragment stays unreachable");
    assert!(
        refused.contains("crate::panel::ControlHandle") && refused.contains("way forward"),
        "the refusal names the path and the way forward: {refused}"
    );
    let _ = fs::remove_dir_all(root.parent().expect("a parent"));
}

/// The three files a ghost is made of, and the config the workspace root has to carry.
/// 幽灵由哪三份文件组成，以及工作区根必须携带的那份配置。
#[test]
fn a_ghost_is_three_files_and_one_workspace_config() {
    let root = host(
        "artifacts",
        &[
            (
                "control/control.rs",
                "crate::root_object! {\n    kind: Control,\n}\n",
            ),
            (
                "control/object/button/button.rs",
                "crate::control_object! {\n    kind: Button,\n}\n",
            ),
        ],
        DECLARATION,
    );
    let planned = plan_host(&root).expect("the fragment is self-contained");
    let widgets = &planned[0];

    // lib.rs: the host's namespace, and nothing else but the include of its own generated tree.
    // lib.rs：宿主的命名空间，除自己那棵生成树的 include 之外没有别的。
    assert!(
        widgets
            .lib_rs
            .contains("pub const NICHLINK_NAMESPACE: &str = \"myapp\";"),
        "the ghost declares the host's namespace: {}",
        widgets.lib_rs
    );
    assert!(
        widgets
            .lib_rs
            .contains("include!(concat!(env!(\"OUT_DIR\"), \"/generated_lib.rs\"));"),
        "and includes its own generated tree: {}",
        widgets.lib_rs
    );

    // build.rs: the host's **package root**, the host's namespace, and the fragment as the render
    // mode. The root, not the manifest: the first end-to-end build failed with
    // `<root>/Cargo.toml/src is not a source directory`, which is what passing the manifest does.
    // build.rs：宿主的**包根**、宿主的命名空间，以及作为渲染模式的碎片。是包根而不是清单：第一次端到端
    // 构建报出 `<root>/Cargo.toml/src is not a source directory`，那正是传清单的后果。
    assert!(
        widgets.build_rs.contains(&format!("{:?}", root))
            && !widgets
                .build_rs
                .contains(&format!("{}/Cargo.toml", root.display()))
            && widgets
                .build_rs
                .contains("nichlink_toolchain::build_time::run_for"),
        "the ghost builds the host's manifest: {}",
        widgets.build_rs
    );
    assert!(
        widgets
            .build_rs
            .contains("NICH_LINK_SHAPE_ONLY\", \"control::object\""),
        "and asks for the fragment only: {}",
        widgets.build_rs
    );
    assert!(
        widgets.build_rs.contains("\"myapp\""),
        "with the host's namespace: {}",
        widgets.build_rs
    );

    // Cargo.toml: the host's dependencies verbatim, and no publishing.
    // Cargo.toml：宿主的依赖逐字照抄，且不发布。
    assert!(
        widgets
            .cargo_toml
            .contains("nichlink-toolchain = { path = \"../toolchain\", features = [\"run\"] }"),
        "the dependencies are the host's: {}",
        widgets.cargo_toml
    );
    assert!(
        widgets.cargo_toml.contains("[build-dependencies]")
            && widgets.cargo_toml.contains("publish = false"),
        "build-dependencies too, and no publishing: {}",
        widgets.cargo_toml
    );

    // The config patch: every remap prefix, and the reason it is the workspace root's file.
    // config 补丁：每一个 remap 前缀，以及它为何属于工作区根那份文件。
    assert!(
        widgets
            .config_patch
            .contains("--remap-path-prefix=../../../../host/src/="),
        "the remap entries are spelled out: {}",
        widgets.config_patch
    );
    assert!(
        widgets.config_patch.contains("workspace root"),
        "and say where they belong: {}",
        widgets.config_patch
    );
    let _ = fs::remove_dir_all(root.parent().expect("a parent"));
}

/// The spelling is a **walk between two paths**, not "sibling plus depth": a ghost the author puts
/// somewhere else walks out of its own nesting, through the shared ancestors, and in through the
/// host's path — and the invariant that makes the remap derivable holds there too.
/// 拼写是**两个路径之间的一段路**，而不是"同级 + 深度"：作者把幽灵放在别处时，它要走出自己的嵌套、穿过共同的
/// 祖先、再顺着宿主的路径走进去——而让映射可推导的那条不变量在那里同样成立。
#[test]
fn a_ghost_placed_elsewhere_still_spells_a_walk_to_the_host() {
    let root = host(
        "elsewhere",
        &[
            (
                "control/control.rs",
                "crate::root_object! {\n    kind: Control,\n}\n",
            ),
            (
                "control/object/button/button.rs",
                "crate::control_object! {\n    kind: Button,\n}\n",
            ),
        ],
        DECLARATION,
    );
    let planned = plan_host(&root).expect("the fragment is self-contained");
    let sibling = &planned[0];
    // The same plan, computed for a ghost nested two levels deeper: the walk grows by two `../` and
    // names the directories it crosses.
    // 同一份规划，但幽灵嵌深两层：那段路多出两个 `../`，并点名它穿过的目录。
    let deeper_dir = root
        .parent()
        .expect("a parent")
        .join("nested/deep")
        .join(&sibling.package);
    let host_dir = root.canonicalize().expect("the host canonicalizes");
    let deeper = crate_plan_spelling(
        &host_dir,
        &deeper_dir,
        "control/object/button/button.rs",
        "control::object::button",
    );
    // Going **up** does not name the directories it passes: the deeper placement simply needs two
    // more `../` before it turns into the host's path.
    // **向上走**不会点名它经过的目录：嵌得更深只意味着在拐进宿主路径之前多两个 `../`。
    assert_eq!(
        deeper.matches("../").count(),
        sibling
            .mounts
            .iter()
            .map(|mount| mount.spelling.matches("../").count())
            .max()
            .unwrap_or(0)
            + 2,
        "two levels deeper, two more `../`: {deeper}"
    );
    assert!(
        deeper.ends_with("host/src/control/object/button/button.rs"),
        "and it ends at the host's own source: {deeper}"
    );
    let (from, to) = super::remap_for(&deeper, "control/object/button/button.rs");
    assert_eq!(to, "", "the prefix maps to nothing");
    assert_eq!(
        format!("{from}control/object/button/button.rs"),
        deeper,
        "the invariant: the spelling is the prefix plus the source"
    );
    let _ = fs::remove_dir_all(root.parent().expect("a parent"));
}

/// The planner's spelling rule, reached the way the module reaches it.
/// 规划器的拼写规则，按模块自己的方式到达它。
fn crate_plan_spelling(
    host_dir: &Path,
    ghost_dir: &Path,
    source: &str,
    module_path: &str,
) -> String {
    super::spelling_for(host_dir, ghost_dir, source, module_path)
}
