//! Source-scope derivation tests.
//! 源码范围推导测试。
//!
//! A separate page so the module it tests stays inside the size ratchet: a test
//! module is excluded from it, and this one had grown as large as the code.
//! 独立一页，使被测模块留在尺寸棘轮之内：测试模块不受棘轮约束，而这一份已经长到
//! 与被测代码相当。

use crate::discovery::discover_root;

/// The face files the scope proved live, named relative to `src`.
/// 作用域证明存活的注册面文件，路径相对 `src`。
fn selected_sources(
    src: &std::path::Path,
    nodes: &[super::Node],
    scope: &super::SourceScope,
) -> Vec<String> {
    let roots = scope.roots.as_ref().expect("the fixture narrows");
    let mut sources = super::collect_faces(src, nodes)
        .into_iter()
        .filter(|face| roots.contains(&face.id))
        .map(|face| super::relative_display(src, &face.source))
        .collect::<Vec<_>>();
    sources.sort();
    sources
}

/// A typed cut narrows the scope to the face it names, and a face nobody
/// declared is pruned: declaring the slot is what ships the face.
/// 类型化切口把作用域收窄到它命名的注册面，没人声明的面会被剪掉：
/// 声明槽位才是这个面被发布出来的原因。
#[test]
fn a_typed_cut_narrows_the_scope_to_the_declared_slot() {
    let root = std::env::temp_dir().join(format!(
        "nichlink-scope-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let src = root.join("src");
    let entry = src.join("lib.rs");
    for module in ["a", "b"] {
        std::fs::create_dir_all(src.join(module)).expect("fixture dir");
    }
    std::fs::write(
        &entry,
        "nichlink_run_method::host!();\n\
             nichlink_run_method::static_graft_plan!(\n\
                 FRAMEWORK,\n\
                 cut(crate::a::NODE_ID) graft(\"a_fast\"),\n\
             );\n",
    )
    .expect("host entry");
    for (module, kind) in [("a", "A"), ("b", "B")] {
        std::fs::write(
            src.join(format!("{module}/{module}.rs")),
            format!("crate::root_object! {{\n    kind: {kind},\n}}\n"),
        )
        .expect("face file");
    }
    let nodes = discover_root(&src);
    let scope = super::SourceScope::auto_from_entry(&src, &nodes, &entry);

    assert_eq!(
        selected_sources(&src, &nodes, &scope),
        ["a/a.rs"],
        "only the declared slot stays live"
    );
    assert_eq!(scope.reason, "auto", "the scope was narrowed, not given up");

    std::fs::remove_dir_all(&root).expect("cleanup");
}

/// The scope and the generated cut table read the entry the build resolved,
/// not two independently resolved ones.
/// 作用域与生成的切口表读的是构建解析出的同一个入口，而不是各自独立解析出的两个。
///
/// The configured value is passed in explicitly because the value itself is
/// what the two readers must agree on; reading the process environment here
/// would race with every other test in this binary. The fixture makes the two
/// entries name different slots, so a reader that ignores the configured value
/// keeps `beta` live and reports `beta`'s cut, while a reader that follows it
/// keeps `alpha` live and reports `alpha`'s.
/// 被指定的值以显式参数传入，因为两个读取者必须一致的就是这个值；在这里读进程环境
/// 会与同一二进制里的其他测试竞争。夹具让两个入口声明不同的槽位，因此忽略指定值的
/// 读取者会让 `beta` 存活并报告 `beta` 的切口，跟随它的读取者则让 `alpha` 存活并
/// 报告 `alpha` 的切口。
#[test]
fn a_configured_entry_drives_the_scope_and_the_cut_table() {
    let root = std::env::temp_dir().join(format!(
        "nichlink-scope-configured-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let src = root.join("src");
    for module in ["alpha", "beta", "preview"] {
        std::fs::create_dir_all(src.join(module)).expect("fixture dir");
    }
    // The convention entry — the file that calls `host!()` — declares `beta`.
    // 约定入口（调用 `host!()` 的文件）声明 `beta`。
    std::fs::write(
        src.join("lib.rs"),
        "nichlink_run_method::host!();\n\
             nichlink_run_method::static_graft_plan!(\n\
                 FRAMEWORK,\n\
                 cut(crate::beta::NODE_ID) graft(\"beta_fast\"),\n\
             );\n",
    )
    .expect("convention entry");
    // The configured entry declares `alpha` instead.
    // 被指定的入口改为声明 `alpha`。
    std::fs::write(
        src.join("preview/preview.rs"),
        "nichlink_run_method::static_graft_plan!(\n\
                 FRAMEWORK,\n\
                 cut(crate::alpha::NODE_ID) graft(\"alpha_fast\"),\n\
             );\n",
    )
    .expect("configured entry");
    for (module, kind) in [("alpha", "Alpha"), ("beta", "Beta")] {
        std::fs::write(
            src.join(format!("{module}/{module}.rs")),
            format!("crate::root_object! {{\n    kind: {kind},\n}}\n"),
        )
        .expect("face file");
    }

    let nodes = discover_root(&src);
    let entry = crate::entry::resolve_host_entry(
        &src,
        &nodes,
        Some(std::path::PathBuf::from("src/preview/preview.rs")),
    );
    assert_eq!(entry.path(), src.join("preview/preview.rs"));

    let cuts = crate::host_graft_entries(&entry).enabled;
    assert_eq!(cuts.len(), 1, "{cuts:?}");
    assert_eq!(cuts[0].cut, "crate::alpha::NODE_ID");

    let scope = super::SourceScope::auto_from_entry(&src, &nodes, entry.path());
    assert_eq!(
        selected_sources(&src, &nodes, &scope),
        ["alpha/alpha.rs"],
        "pruning follows the same entry the cut table read"
    );
    assert_eq!(scope.reason, "auto", "the scope was narrowed, not given up");

    std::fs::remove_dir_all(&root).expect("cleanup");
}

/// A typed cut whose Rust path names no discovered face is an unreadable
/// declaration: the whole tree is kept rather than pruned wrongly.
/// 类型化切口的 Rust 路径指不到任何已发现注册面时，声明就是读不懂的：
/// 保留整棵树，而不是错误裁剪。
#[test]
fn an_unplaceable_typed_cut_keeps_the_whole_tree() {
    let root = std::env::temp_dir().join(format!(
        "nichlink-scope-unrecognized-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let src = root.join("src");
    let entry = src.join("lib.rs");
    std::fs::create_dir_all(src.join("a")).expect("fixture dir");
    std::fs::write(
        &entry,
        "nichlink_run_method::host!();\n\
             nichlink_run_method::static_graft_plan!(\n\
                 FRAMEWORK,\n\
                 cut(crate::missing::NODE_ID) graft(\"a_fast\"),\n\
             );\n",
    )
    .expect("host entry");
    std::fs::write(
        src.join("a/a.rs"),
        "crate::root_object! {\n    kind: A,\n}\n",
    )
    .expect("face file");
    let nodes = discover_root(&src);
    let scope = super::SourceScope::auto_from_entry(&src, &nodes, &entry);

    assert_eq!(scope.roots, None);
    assert_eq!(scope.reason, "graft-typed-cut-unrecognized");

    std::fs::remove_dir_all(&root).expect("cleanup");
}

/// A scope value the build refuses is a diagnostic, not a panic: `check
/// --json` gets a document and the fallback prunes nothing.
/// 构建拒绝的范围取值是诊断而不是 panic：`check --json` 拿到文档，回退什么都不剪。
#[test]
fn a_refused_scope_value_is_a_diagnostic() {
    let src = std::env::temp_dir().join("nichlink-scope-diagnostics");
    let entry = super::HostEntry::Convention(src.join("lib.rs"));
    let cases = [
        (
            "an identity schema the build does not speak",
            "v99:0".to_owned(),
            "identity schema",
        ),
        (
            "a value that is not a node identity",
            "not-a-node-identity".to_owned(),
            "32-digit node identity",
        ),
        (
            "an identity no node in this tree has",
            format!("v{}:{}", super::IDENTITY_SCHEMA, "0".repeat(32)),
            "unknown node identity",
        ),
    ];
    for (case, raw, expected) in cases {
        let mut errors = super::BuildDiagnostics::default();
        let scope = super::SourceScope::from_raw(&raw, &src, &[], &entry, &mut errors);
        assert!(
            errors.iter().any(|diagnostic| {
                diagnostic.phase == "scope" && diagnostic.message.contains(expected)
            }),
            "{case}: {:#?}",
            errors.iter().collect::<Vec<_>>()
        );
        assert!(
            scope.roots.is_none(),
            "{case}: every refusal falls back to the full tree"
        );
    }
}
