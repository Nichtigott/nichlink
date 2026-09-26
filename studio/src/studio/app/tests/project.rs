//! Project root, manifest, and new-project wizard tests.
//! 项目根、清单与新项目向导测试。

use super::super::state::new_project_field;
use super::*;
use nichlink_run_method::face_field;

#[test]
fn standalone_studio_resolves_one_project_root_and_manifest() {
    let root = package_root();
    assert!(root.is_dir());
    assert_eq!(
        host_manifest().file_name().and_then(|name| name.to_str()),
        Some("Cargo.toml")
    );
}

/// A throwaway project directory holding one manifest and a library target, so
/// Cargo can be asked about it: `cargo metadata` refuses a manifest with no
/// target, and the name's authority is Cargo.
/// 一个只含一份清单与一个库目标的一次性项目目录，因此可以向 Cargo 询问它：`cargo metadata`
/// 拒绝没有目标的清单，而包名的权威是 Cargo。
fn temp_project(label: &str, manifest: &str) -> std::path::PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("nichlink-studio-namespace-{label}-{suffix}"));
    std::fs::create_dir_all(root.join("src")).expect("project directory");
    std::fs::write(root.join("Cargo.toml"), manifest).expect("manifest");
    std::fs::write(root.join("src/lib.rs"), "// host entry\n").expect("library target");
    root
}

/// A launched session authors under the host crate's own package name, read from
/// the manifest it adopted — the value the host's build script stamps as
/// `CARGO_PKG_NAME`, and therefore the identity domain its recorded node ids live
/// in.
/// 已启动的会话在宿主 crate 自己的包名之下创作，读自它采纳的清单——也就是宿主构建脚本盖成
/// `CARGO_PKG_NAME` 的那个值，因此也是它记录的节点身份所在的域。
#[test]
fn a_launched_session_authors_under_the_host_crates_package_name() {
    let root = temp_project(
        "launched",
        "[package]\nname = \"demo-app\"\nversion = \"0.1.0\"\n",
    );
    let resolved = resolve_project(Some(&root)).expect("the fixture is a project");
    assert_eq!(resolved, root);
    assert_eq!(host_manifest(), root.join("Cargo.toml"));
    // The environment override still wins verbatim, exactly as the build side reads
    // it; without one the manifest names the namespace.
    let expected = std::env::var("NICH_LINK_NAMESPACE").unwrap_or_else(|_| "demo-app".to_owned());
    assert_eq!(package_namespace(), expected);
    // The adopted context is thread-local and the harness reuses threads, so the
    // fixture directory stays: a later test on this thread that falls back to
    // `package_root()` must still find a directory.
    // 采纳的上下文是线程局部的，而测试框架会复用线程，因此夹具目录留在原处：本线程上随后
    // 回落到 `package_root()` 的测试仍须找到一个存在的目录。
}

/// The namespace comes from Cargo's answer for the adopted manifest, not from a
/// literal line scan. The load-bearing case is TOML's dotted form: it is the same
/// table as `[package]`, a line scan sees no header and answers nothing, and
/// Studio then authored the project under `nichlink.default` — an identity domain
/// none of that project's recorded ids live in, so a trace or a graft record on
/// disk could not resolve. The remaining cases are the ones the old scan already
/// had to get right, kept so the authority change cannot quietly lose them.
/// 命名空间来自 Cargo 对已采纳清单的回答，而不是字面逐行扫描。承重的情形是 TOML 的点式写法：
/// 它与 `[package]` 是同一张表，逐行扫描看不到任何表头、什么也答不出，于是 Studio 会在
/// `nichlink.default` 之下创作该项目——而该项目记录的任何 id 都不住在那个身份域里，落盘的
/// trace 或 graft 记录因此解析不了。其余情形是旧扫描本来就必须答对的那些，保留它们是为了让这次
/// 权威更替不会悄悄丢掉它们。
#[test]
fn the_namespace_comes_from_cargo_not_from_a_literal_manifest_scan() {
    let default = nichlink_run_method::lexicon::DEFAULT_NAMESPACE;
    let cases: &[(&str, &str)] = &[
        (
            "[package]\nname = \"demo-app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
            "demo-app",
        ),
        (
            "[package]\nname = 'single-quoted'\nversion = \"0.1.0\"\nedition = \"2021\"\n",
            "single-quoted",
        ),
        (
            "# name = \"commented\"\n[package]\nname = \"real\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
            "real",
        ),
        (
            "[package]\nname = \"inline\" # trailing comment\nversion = \"0.1.0\"\nedition = \"2021\"\n",
            "inline",
        ),
        (
            "package.name = \"dotted-name\"\npackage.version = \"0.1.0\"\npackage.edition = \"2021\"\n",
            "dotted-name",
        ),
        ("[dependencies]\nname = \"another-package\"\n", default),
        ("[package]\nname.workspace = true\n", default),
        ("[workspace]\nmembers = [\"app\"]\n", default),
    ];
    for (index, (manifest, expected)) in cases.iter().enumerate() {
        let root = temp_project(&format!("cargo-{index}"), manifest);
        assert_eq!(
            namespace_for(&root.join("Cargo.toml"), None),
            *expected,
            "{manifest:?}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}

/// Every write goes through the selected project. With none selected the wrapper
/// refuses; it never resolves a root from the environment or the working
/// directory, which is how a delete once moved a module out of the wrong project.
/// 每次写入都经过已选中的项目。没有选中时包装函数拒绝执行，绝不从环境或工作目录解析根——正是
/// 那样一次解析让删除把模块从错误的项目里搬走了。
#[test]
fn a_write_without_a_selected_project_is_refused() {
    clear_project_context();
    assert_eq!(
        with_selected_project(|| Ok::<_, String>("wrote")),
        Err("no project is selected; open a project before writing to it".to_owned()),
        "a write must not fall back to a guessed root"
    );
    assert!(selected_package_root().is_err());

    let root = temp_project(
        "writers",
        "[package]\nname = \"writers-app\"\nversion = \"0.1.0\"\n",
    );
    select_project(root.clone(), root.join("Cargo.toml"), "writers-app");
    assert_eq!(
        with_selected_project(|| Ok::<_, String>("wrote")),
        Ok("wrote")
    );
    assert_eq!(selected_package_root(), Ok(root.clone()));
    // The context a write runs in carries the selected project's namespace, so
    // `delete_module` and friends resolve ids in the domain the reader opened.
    // 写入所运行的上下文带着已选中项目的命名空间，因此 `delete_module` 之类的调用在读者打开的
    // 那个域里解析 id。
    assert_eq!(
        with_selected_project(|| Ok::<_, String>(package_namespace())),
        Ok("writers-app".to_owned())
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A delete with no project selected is refused before it can act on a guessed
/// root. This is the operation the overlay's own comment remembers: it used to
/// resolve its root through the environment or the working directory and move a
/// module out of whichever project that named.
/// 没有选中项目时的删除会在作用于猜来的根之前被拒绝。这正是删除浮层自己的注释记得的那个操作：
/// 它过去会经环境变量或工作目录解析根，并把模块从那个恰好命名的项目里搬走。
#[test]
fn a_delete_without_a_selected_project_is_refused() {
    clear_project_context();
    let mut app = App::load();
    let target = app.selected;
    app.overlay = Some(Overlay::Delete(target));
    app.handle_key(KeyEvent::from(KeyCode::Char('y')));
    assert!(
        app.event.contains("no project is selected"),
        "a delete must refuse a guessed root: {}",
        app.event
    );
}

/// The namespace follows the manifest unless the environment names one, and a
/// manifest with no `[package]` falls back to the documented default.
/// 命名空间跟随清单，除非环境给出一个；没有 `[package]` 的清单回落到文档化的默认值。
#[test]
fn the_namespace_follows_the_manifest_unless_the_environment_names_one() {
    let host = temp_project(
        "ns-host",
        "[package]\nname = \"demo-app\"\nversion = \"0.1.0\"\n",
    );
    let manifest = host.join("Cargo.toml");
    assert_eq!(namespace_for(&manifest, None), "demo-app");
    assert_eq!(namespace_for(&manifest, Some("pinned")), "pinned");

    let workspace = temp_project("ns-workspace", "[workspace]\nmembers = [\"app\"]\n");
    assert_eq!(
        namespace_for(&workspace.join("Cargo.toml"), None),
        nichlink_run_method::lexicon::DEFAULT_NAMESPACE
    );
    let _ = std::fs::remove_dir_all(&host);
    let _ = std::fs::remove_dir_all(&workspace);
}

#[test]
fn installed_studio_never_exports_cargo_git_cache_paths() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let checkout = std::env::temp_dir().join(format!("cargo-git-checkout-{suffix}"));
    std::fs::create_dir_all(checkout.join("core")).expect("core sibling");
    std::fs::create_dir_all(checkout.join("build_method")).expect("build sibling");
    std::fs::create_dir_all(checkout.join("studio")).expect("studio sibling");

    let source = nichlink_build_method::scaffold::detected_source(
        &checkout.join("studio"),
        &checkout.join("outside-bin/nichlink-studio"),
    );
    let (core, build) = nichlink_build_method::scaffold::dependency_specs(&source);

    assert!(core.contains("git = \"https://github.com/Nichtigott/nichlink\""));
    assert!(build.contains("branch = \"main\""));
    assert!(!core.contains(checkout.to_string_lossy().as_ref()));
    assert!(!build.contains(checkout.to_string_lossy().as_ref()));
    let _ = std::fs::remove_dir_all(checkout);
}

#[test]
fn new_project_and_explicit_root_face_compile() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("nichlink-studio-new-{suffix}"));

    let mut app = App::load();
    app.handle_key(KeyEvent::from(KeyCode::Char('n')));
    let Some(Overlay::NewProject(mut project)) = app.overlay.take() else {
        panic!("n should open the New Project wizard");
    };
    project.values[new_project_field::DIRECTORY] = root.display().to_string();
    project.values[new_project_field::PACKAGE] = "sample-app".to_owned();
    project.values[new_project_field::KIND] = "binary".to_owned();
    app.overlay = Some(Overlay::NewProject(project));
    app.handle_key(KeyEvent::from(KeyCode::Char('s')));

    assert!(root.join("Cargo.toml").is_file());
    assert!(root.join("build.rs").is_file());
    assert!(root.join("src/main.rs").is_file());
    assert!(
        std::fs::read_to_string(root.join("src/main.rs"))
            .expect("generated binary entry")
            .contains("builtin_static_plan().len()")
    );
    let manifest = std::fs::read_to_string(root.join("Cargo.toml")).expect("project manifest");
    assert!(manifest.contains("nichlink-run-method"));
    assert!(manifest.contains("nichlink-build"));
    assert!(app.registry.depth_first().is_empty());

    let root_id = app.registry.id();
    let mut owner = AddState::new(root_id);
    owner.values[face_field::PARENT] = root_id.to_string();
    owner.values[face_field::MODULE] = "workspace".to_owned();
    owner.values[face_field::NEEDS_REGISTRY] = "true".to_owned();
    owner.values[face_field::TREE_SLOT] = "workspace".to_owned();
    owner.values[face_field::KIND] = "Workspace".to_owned();
    app.submit_add(&owner);
    assert!(
        root.join("src/workspace/registry_rule/registry_rule.rs")
            .is_file()
    );
    let workspace =
        std::fs::read_to_string(root.join("src/workspace/workspace.rs")).expect("workspace face");
    assert!(workspace.starts_with("// generated-by=NichLink"));
    assert!(workspace.contains("crate::root_object!"));
    assert!(workspace.contains("crate::root_object! {\n    kind: Workspace,"));

    let workspace_id = app
        .registry
        .depth_first()
        .into_iter()
        .find(|info| info.registry_name == "workspace")
        .unwrap_or_else(|| panic!("explicit workspace registry; event={}", app.event))
        .id;
    let mut child = AddState::new(workspace_id);
    child.values[face_field::PARENT] = workspace_id.to_string();
    child.values[face_field::MODULE] = "panel".to_owned();
    child.values[face_field::NEEDS_REGISTRY] = "true".to_owned();
    child.values[face_field::TREE_SLOT] = "panel".to_owned();
    child.values[face_field::KIND] = "Panel".to_owned();
    app.submit_add(&child);
    assert!(
        root.join("src/workspace/object/panel/registry_rule/registry_rule.rs")
            .is_file()
    );
    let panel = std::fs::read_to_string(root.join("src/workspace/object/panel/panel.rs"))
        .expect("panel face");
    assert!(panel.contains("crate::workspace_object!"));
    // A face that owns a registry does not repeat the rule path: the declaration
    // resolves `registry_rule:` to the canonical module beside the face, and the
    // file written just above is that module.
    // 拥有注册机的面不再重复规则路径：声明把 `registry_rule:` 解析到注册面旁边的规范
    // 模块，而上面刚写下的那个文件就是该模块。
    assert!(panel.contains("needs_registry: true"));
    assert!(
        !panel.contains("registry_rule:"),
        "the derived rule must not be written back: {panel}"
    );
    let panel_id = app
        .registry
        .depth_first()
        .into_iter()
        .find(|info| info.registry_name == "panel")
        .expect("panel registry")
        .id;
    let mut leaf = AddState::new(panel_id);
    leaf.values[face_field::PARENT] = panel_id.to_string();
    leaf.values[face_field::MODULE] = "button".to_owned();
    leaf.values[face_field::KIND] = "Button".to_owned();
    app.submit_add(&leaf);
    let button =
        std::fs::read_to_string(root.join("src/workspace/object/panel/object/button/button.rs"))
            .expect("third-level button face");
    assert!(button.contains("crate::panel_object!"));
    assert!(button.contains("parent: crate::workspace::object::panel::NODE_ID"));

    // `control_object!` is no longer the generic implementation. It is
    // generated only as the declaration name for children of `control`.
    // `control_object!` 不再是通用实现，只作为 control 子对象的声明名生成。
    let mut control = AddState::new(root_id);
    control.values[face_field::PARENT] = root_id.to_string();
    control.values[face_field::MODULE] = "control".to_owned();
    control.values[face_field::NEEDS_REGISTRY] = "true".to_owned();
    control.values[face_field::KIND] = "Control".to_owned();
    app.submit_add(&control);
    let control_id = app
        .registry
        .depth_first()
        .into_iter()
        .find(|info| info.registry_name == "control")
        .expect("control registry")
        .id;
    let mut control_child = AddState::new(control_id);
    control_child.values[face_field::PARENT] = control_id.to_string();
    control_child.values[face_field::MODULE] = "slider".to_owned();
    control_child.values[face_field::KIND] = "Slider".to_owned();
    app.submit_add(&control_child);
    let slider = std::fs::read_to_string(root.join("src/control/object/slider/slider.rs"))
        .expect("control child face");
    assert!(slider.contains("crate::control_object!"));
    assert!(slider.contains("parent: crate::control::NODE_ID"));

    let manifest = std::fs::read_to_string(root.join("Cargo.toml")).expect("project manifest");
    assert!(manifest.contains("edition = \"2024\""));

    let check = std::process::Command::new("cargo")
        .args(["check", "--offline"])
        .current_dir(&root)
        .output()
        .expect("cargo check generated project");
    assert!(
        check.status.success(),
        "generated project failed to compile:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn new_project_wizard_toggles_library_kind() {
    let mut app = App::load();
    app.handle_key(KeyEvent::from(KeyCode::Char('n')));
    app.handle_key(KeyEvent::from(KeyCode::Down));
    app.handle_key(KeyEvent::from(KeyCode::Down));
    app.handle_key(KeyEvent::from(KeyCode::Enter));
    assert!(matches!(app.overlay, Some(Overlay::NewProject(ref project))
        if project.values[new_project_field::KIND] == "library"));
}

#[test]
fn new_project_wizard_creates_library_entrypoint() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("nichlink-studio-lib-{suffix}"));

    let mut app = App::load();
    app.handle_key(KeyEvent::from(KeyCode::Char('n')));
    let Some(Overlay::NewProject(mut project)) = app.overlay.take() else {
        panic!("n should open the New Project wizard");
    };
    project.values[new_project_field::DIRECTORY] = root.display().to_string();
    project.values[new_project_field::PACKAGE] = "sample-framework".to_owned();
    project.values[new_project_field::KIND] = "library".to_owned();
    app.overlay = Some(Overlay::NewProject(project));
    app.handle_key(KeyEvent::from(KeyCode::Char('s')));

    assert!(root.join("src/lib.rs").is_file());
    assert!(!root.join("src/main.rs").exists());
    assert!(!root.join("src/control").exists());

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn new_project_starts_with_an_empty_registration_tree() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("nichlink-studio-empty-{suffix}"));

    let mut app = App::load();
    let mut project = super::super::NewProjectState::new();
    project.values[new_project_field::DIRECTORY] = root.display().to_string();
    project.values[new_project_field::PACKAGE] = "empty-app".to_owned();
    app.submit_new_project(&project);

    assert!(!root.join("src/control").exists());
    assert!(app.registry.depth_first().is_empty());

    let _ = std::fs::remove_dir_all(&root);
}

/// A project that cannot be resolved is refused by name, never replaced by the
/// directory this crate was compiled in. That directory is the checkout or the
/// installed crate's sources, and it is exactly where a launch with nothing open
/// used to write the next face.
/// 解析不出的项目按名字被拒绝，绝不由本 crate 编译时所在的目录顶替。那目录是检出目录或
/// 已安装 crate 的源码，而"没打开任何项目"的启动过去正是把下一个注册面写在那里。
#[test]
fn an_unresolvable_project_is_refused_instead_of_falling_back() {
    let missing = std::env::temp_dir().join("nichlink-studio-missing-project");

    let nothing = resolve_project_from(None, None, None, None, false)
        .expect_err("no project at all must be an error");
    assert!(nothing.contains("no project to open"), "{nothing}");
    assert!(
        nothing.contains("NICH_LINK_PACKAGE_ROOT"),
        "the message must say how to point Studio at a project: {nothing}"
    );

    let explicit = resolve_project_from(None, Some(&missing), None, None, false)
        .expect_err("a path argument that is not a directory must be refused");
    assert!(explicit.contains("path argument"), "{explicit}");
    assert!(
        explicit.contains("nichlink-studio-missing-project"),
        "{explicit}"
    );

    let configured = resolve_project_from(None, None, Some(&missing), None, false)
        .expect_err("a configured root that is not a directory must be refused");
    assert!(
        configured.contains("NICH_LINK_PACKAGE_ROOT"),
        "{configured}"
    );

    let selected = resolve_project_from(Some(&missing), None, None, None, false)
        .expect_err("a selected project that vanished must be refused");
    assert!(selected.contains("select_project"), "{selected}");
}

/// The working directory is the only implicit project, and only when it holds a
/// package: a directory without a manifest is not guessed at.
/// 当前目录是唯一的隐式项目，而且只在它持有包时成立；没有清单的目录不会被猜。
#[test]
fn only_a_working_directory_that_holds_a_package_is_opened() {
    let current = std::env::temp_dir();
    assert_eq!(
        resolve_project_from(None, None, None, Some(&current), true),
        Ok(current.clone())
    );
    assert!(
        resolve_project_from(None, None, None, Some(&current), false).is_err(),
        "a directory without a Cargo.toml is not a project"
    );
}

/// A relative candidate resolves against the working directory, so a relative
/// argument or environment value names the directory the shell would name.
/// 相对候选值相对当前目录解析，因此相对参数或相对环境变量指向 shell 会指向的目录。
#[test]
fn a_relative_candidate_resolves_against_the_working_directory() {
    let root = std::env::temp_dir();
    let child = root.join("nichlink-studio-relative");
    std::fs::create_dir_all(&child).expect("fixture directory");
    assert_eq!(
        resolve_project_from(
            None,
            Some(std::path::Path::new("nichlink-studio-relative")),
            None,
            Some(&root),
            false,
        ),
        Ok(child.clone())
    );
    let _ = std::fs::remove_dir_all(&child);
}

/// Build a throwaway package with the requested targets and run the MIR target
/// resolver against it, returning whether the selected target compiled.
/// 构建一个带有所请求 target 的一次性包，对其运行 MIR target 解析器，返回所选 target
/// 是否编译通过。
fn mir_target_resolves(label: &str, library: bool, binary: bool) -> bool {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("nichlink-studio-mir-{label}-{suffix}"));
    std::fs::create_dir_all(root.join("src")).expect("fixture src");
    std::fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"mir-{label}-host\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"
        ),
    )
    .expect("manifest");
    if library {
        std::fs::write(root.join("src/lib.rs"), "pub fn placeholder() {}\n")
            .expect("library entry");
    }
    if binary {
        std::fs::write(root.join("src/main.rs"), "fn main() {}\n").expect("binary entry");
    }
    // `--emit=metadata` is stable, so the pin is the target *selection*; the
    // production call site adds the nightly-only `-Zunpretty=mir`.
    // `--emit=metadata` 在 stable 上可用，因此这条钉子钉的是 target **选择**；生产调用点
    // 才加上仅 nightly 的 `-Zunpretty=mir`。
    let output =
        cargo_rustc_mir(&root.join("Cargo.toml"), &["--emit=metadata"]).expect("cargo runs");
    let compiled = output.status.success();
    if !compiled {
        eprintln!("{label}: {}", String::from_utf8_lossy(&output.stderr));
    }
    let _ = std::fs::remove_dir_all(&root);
    compiled
}

/// A binary-only package — the default output of `nichlink new` — resolves a
/// target for MIR inspection instead of failing on the hardcoded `--lib`.
/// 仅含二进制的包——`nichlink new` 的默认产物——会为 MIR 检视解析出一个 target，而不是在
/// 硬编码的 `--lib` 上失败。
#[test]
fn mir_inspection_resolves_the_target_a_package_actually_has() {
    assert!(
        mir_target_resolves("bin", false, true),
        "a binary-only package must resolve its binary target"
    );
    assert!(
        mir_target_resolves("lib", true, false),
        "a library-only package must still resolve its library target"
    );
    assert!(
        mir_target_resolves("both", true, true),
        "a package with both targets must resolve one of them"
    );
}
