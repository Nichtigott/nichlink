//! Pins for the release shape: self-contained packages, and no remap anywhere (audit `M7`, §M7.42).
//! 发布形状的钉子：自包含的包，以及哪里都不需要 remap（审计 `M7`，§M7.42）。

use std::fs;
use std::path::{Path, PathBuf};

use super::{plan_facade, plan_ghost};
use crate::build_method::crate_plan::{PlannedCrate, PlannedMount};
use nichlink_kernel::identity::NodeId;

/// A throwaway host package with the manifest a plan reads.
/// 一个一次性宿主包，带规划要读的清单。
fn host(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir()
        .join("nichlink-scratch")
        .join(module_path!().replace("::", "-"))
        .join(format!(
            "nichlink-release-{label}-{}-{sequence}",
            std::process::id()
        ));
    let _ = std::fs::remove_dir_all(&root);
    for file in [
        "src/panel/frame/frame.rs",
        "src/panel/frame/widget/widget.rs",
        "src/panel/frame/registry_rule/registry_rule.rs",
    ] {
        let path = root.join(file);
        fs::create_dir_all(path.parent().expect("a parent")).expect("the host's sources");
        fs::write(&path, "// a face\n").expect("a face file");
    }
    fs::write(
        root.join("add_crates.rs"),
        "pub const SHAPE: Shape = Shape {};\n",
    )
    .expect("the declaration");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"host\"\nversion = \"9.9.9\"\nedition = \"2021\"\n\n\
         [dependencies]\nnichlink-toolchain = { path = \"../toolchain\" }\n\
         fast-widget = { path = \"../fast-widget\" }\n\n\
         [build-dependencies]\nnichlink-toolchain = { path = \"../toolchain\" }\n",
    )
    .expect("the host manifest");
    root
}

/// One planned ghost, as the planner would have produced it for a two-face fragment.
/// 一个已规划的幽灵，形状与规划器为一个两个面的碎片产出的相同。
/// The faces the plan was computed from, as the CLI hands them to the planner.
/// 规划据以计算的那些注册面，按 CLI 交给规划器的形状。
fn faces() -> Vec<(String, String, NodeId)> {
    vec![
        (
            "panel/frame/frame.rs".to_owned(),
            "panel::frame".to_owned(),
            NodeId::from_bytes(b"frame"),
        ),
        (
            "panel/frame/widget/widget.rs".to_owned(),
            "panel::frame::widget".to_owned(),
            NodeId::from_bytes(b"widget"),
        ),
    ]
}

fn planned(root: &Path) -> PlannedCrate {
    PlannedCrate {
        name: "widgets".to_owned(),
        package: "host-widgets".to_owned(),
        directory: root.parent().expect("a parent").join("host-widgets"),
        namespace: "host".to_owned(),
        subtrees: vec!["panel::frame".to_owned()],
        ancestors: Vec::new(),
        mounts: vec![
            PlannedMount {
                module_path: "panel::frame".to_owned(),
                source: "panel/frame/frame.rs".to_owned(),
                spelling: String::new(),
            },
            PlannedMount {
                module_path: "panel::frame::widget".to_owned(),
                source: "panel/frame/widget/widget.rs".to_owned(),
                spelling: String::new(),
            },
        ],
        lib_rs: String::new(),
        build_rs: String::new(),
        cargo_toml: String::new(),
        config_patch: String::new(),
        remap: vec![("unused".to_owned(), String::new())],
    }
}

/// A release ghost carries the fragment's own files, and its manifest says which shape it is.
/// 发布幽灵携带碎片自己的文件，而它的清单说明了它是哪一个形状。
#[test]
fn a_release_ghost_carries_the_files_its_build_reads() {
    let root = host("ghost");
    let ghost = plan_ghost(&root, &planned(&root), &faces()).expect("the ghost plans");

    // Every mounted face is copied, at the same relative source the host's records name — that path is
    // what makes the copied file's identity the one the host baked, with no remap.
    // 每个被挂载的面都被复制，且用宿主记录点名的同一个相对源码——正是那条路径让被复制的文件保住宿主烤进去的
    // 身份，不需要任何 remap。
    let copies: Vec<&String> = ghost.copies.iter().map(|(_, to)| to).collect();
    assert!(
        copies.contains(&&"src/panel/frame/frame.rs".to_owned()),
        "{copies:?}"
    );
    assert!(
        copies.contains(&&"src/panel/frame/widget/widget.rs".to_owned()),
        "{copies:?}"
    );
    // The declaration comes along: the pipeline needs it to plan the same shells the host's shape did.
    // 声明一起过来：管线需要它来规划与宿主形状相同的壳。
    assert!(copies.contains(&&"add_crates.rs".to_owned()), "{copies:?}");
    for (source, _) in &ghost.copies {
        assert!(
            source.starts_with(&root),
            "every copy comes from the host: {}",
            source.display()
        );
    }

    // The shape fingerprint, the host it was split out of, and **no** `publish = false`.
    // 形状指纹、它是从哪个宿主拆出来的，以及**没有** `publish = false`。
    assert!(ghost.cargo_toml.contains("shape = \"release\""));
    assert!(ghost.cargo_toml.contains("host = \"host\""));
    assert!(!ghost.cargo_toml.contains("publish = false"));
    assert!(
        ghost
            .cargo_toml
            .contains("fast-widget = { path = \"../fast-widget\" }"),
        "the host's dependency table comes verbatim: {}",
        ghost.cargo_toml
    );

    // The build reads **this package's** root and asks for the fragment; nothing points at the host.
    // 构建读的是**本包自己**的根，并只要那个碎片；没有任何东西指向宿主。
    assert!(ghost.build_rs.contains("env!(\"CARGO_MANIFEST_DIR\")"));
    assert!(ghost.build_rs.contains("run_for_partition"));
    assert!(!ghost.build_rs.contains("unsafe") && !ghost.build_rs.contains("set_var"));
    assert!(ghost.build_rs.contains("\"panel::frame\""));
    assert!(
        !ghost.build_rs.contains(&root.display().to_string()),
        "a release build script must not name the host's directory: {}",
        ghost.build_rs
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A release facade carries no sources: its build resolves the host as a dependency, which is the only
/// spelling that survives publishing.
/// 发布 facade 不携带源码：它的构建把宿主当依赖解析，而那是唯一能在发布后成立的拼写。
#[test]
fn a_release_facade_resolves_its_host_instead_of_carrying_it() {
    let root = host("facade");
    let planned = vec![planned(&root)];
    let facade = plan_facade(&root, "host", "host", "host", &planned).expect("the facade plans");

    assert!(
        facade.copies.is_empty(),
        "no sources are duplicated: {:?}",
        facade.copies
    );
    assert!(facade.cargo_toml.contains("shape = \"release\""));
    assert!(!facade.cargo_toml.contains("publish = false"));
    // The sibling dependencies are still spelled relatively, which is what a published manifest turns
    // into a registry requirement — and why the build script cannot read the host from that path.
    // 同级依赖仍用相对路径拼写，发布后的清单会把它变成注册局要求——这也正是构建脚本不能从那个路径读宿主的原因。
    assert!(
        facade.cargo_toml.contains("host = { path = "),
        "the host is a sibling dependency: {}",
        facade.cargo_toml
    );
    assert!(
        facade
            .build_rs
            .contains("\"metadata\", \"--format-version\", \"1\""),
        "the host root is resolved through cargo: {}",
        facade.build_rs
    );
    assert!(facade.build_rs.contains("run_for_partition"));
    assert!(!facade.build_rs.contains("unsafe") && !facade.build_rs.contains("set_var"));
    assert!(
        !facade.build_rs.contains(&root.display().to_string()),
        "the facade's build script must not bake the author's host path: {}",
        facade.build_rs
    );

    // A declaration with no crates has nothing for a facade to carry, and says so.
    // 没有 crate 的声明没有东西让 facade 承载，它会说出来。
    let refused = plan_facade(&root, "host", "host", "host", &[]).expect_err("refused");
    assert!(
        refused.contains("at least one generated package"),
        "{refused}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
