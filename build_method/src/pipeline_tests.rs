//! Tests for the build pipeline's refusal paths.
//! 构建管线拒绝路径的测试。

use super::cache_status_line;

/// Keeps two tests in the same process apart even when the clock resolution
/// collapses their timestamps into one nanosecond; the workspace suite runs
/// them in parallel and a collision silently mixes two fixtures.
/// 即使时钟分辨率把两个测试的时间戳压进同一纳秒，也把同一进程内的两者分开；
/// workspace 套件并行运行它们，撞名会静默把两套夹具混在一起。
static TEMP_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[test]
fn successful_build_is_silent_unless_verbose_output_is_requested() {
    assert_eq!(cache_status_line("hit", "abc", false), None);
    assert_eq!(
        cache_status_line("hit", "abc", true).as_deref(),
        Some("nichlink discovery cache hit (abc)")
    );
}

/// The build-script path has no stdout contract to keep and no generated tree
/// to carry the message, so a missing source tree fails loudly with the
/// rendered diagnostic — not with the bare `expect` it used to hit, which
/// named neither the tree nor the reason.
/// 构建脚本路径没有 stdout 契约要守，也没有生成树可以承载消息，因此源树缺失时带着渲染后的
/// 诊断响亮失败——而不是撞上过去那条裸的 `expect`，它既没说清哪棵树，也没说清原因。
#[test]
#[should_panic(expected = "face-layout")]
fn a_build_script_without_a_source_tree_fails_with_the_layout_diagnostic() {
    let sequence = TEMP_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "nichlink-pipeline-missing-src-{}-{sequence}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).expect("fixture root");
    let input = super::BuildInput {
        manifest: root.clone(),
        src: root.join("src"),
        out_dir: root.join("target/nichlink/out"),
        emit_cargo_directives: true,
    };
    let _ = super::run(&input);
}

/// Discovery skips a directory whose name cannot be a module, so its faces
/// never reach the generated tree. The admission scan reads the same tree,
/// so it must skip them too: a face that can never be compiled must not be
/// able to veto the build.
/// 发现过程会跳过名字无法成为模块的目录，其中的注册面永远进不了生成树。admission
/// 扫描读的是同一棵树，因此也必须跳过它们：永远编译不到的面不该能否决构建。
#[test]
fn an_unrepresentable_directory_cannot_veto_the_build() {
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let sequence = TEMP_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let face = "crate::root_object! {\n    kind: Hidden,\n    requires: [\"missing.capability\" => \"MissingProvider\"],\n}\n";
    let mut outcomes = Vec::new();
    for (tag, dir) in [("valid", "hidden"), ("invalid", "bad-name")] {
        let root = std::env::temp_dir().join(format!("nichlink-hidden-{tag}-{suffix}-{sequence}"));
        let manifest = root.join("host");
        let folder = manifest.join("src").join(dir);
        std::fs::create_dir_all(&folder).expect("face folder");
        std::fs::write(folder.join(format!("{dir}.rs")), face).expect("write face");
        let out = root.join("out");
        outcomes.push(crate::run_for(&manifest, &out, "test-host").is_err());
        let _ = std::fs::remove_dir_all(&root);
    }
    assert_eq!(
        outcomes,
        [true, false],
        "the same face must fail in a representable directory and be ignored in one that cannot be a module"
    );
}

#[test]
fn run_for_validates_outside_cargo_and_reports_diagnostics() {
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let sequence = TEMP_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("nichlink-run-for-{suffix}-{sequence}"));
    let manifest = root.join("host");
    std::fs::create_dir_all(manifest.join("src")).expect("src");
    let out = root.join("out");

    // An empty host has no registration faces; validation succeeds.
    assert!(crate::run_for(&manifest, &out, "test-host").is_ok());

    // Two faces declaring the same stable_name fail with rendered
    // diagnostics instead of `cargo:` directive noise. Faces follow the
    // `<name>/<name>.rs` layout.
    let face = |kind: &str| {
        format!("crate::root_object! {{\n    kind: {kind},\n    stable_name: \"dup\",\n}}\n")
    };
    for (dir, kind) in [("one", "One"), ("two", "Two")] {
        let folder = manifest.join("src").join(dir);
        std::fs::create_dir_all(&folder).expect("face folder");
        std::fs::write(folder.join(format!("{dir}.rs")), face(kind)).expect("write face");
    }
    let error =
        crate::run_for(&manifest, &out, "test-host").expect_err("duplicate stable_name must fail");
    assert!(
        error.contains("duplicate stable_name"),
        "unexpected diagnostics: {error}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// A host directory for one discovery case, removed by the caller.
/// 为某个发现用例建一个宿主目录，由调用方删除。
fn host_root(tag: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let sequence = TEMP_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("nichlink-{tag}-{suffix}-{sequence}"));
    let manifest = root.join("host");
    std::fs::create_dir_all(manifest.join("src")).expect("src");
    (root, manifest)
}

/// An ordinary module beside the faces is not a registration source. Any flat
/// `.rs` under `src/` used to panic with a layout message, which made the tool
/// unusable on a real crate: `src/helpers.rs` is not a mistake.
/// 注册面旁边的普通模块不是注册源。`src/` 下任何平铺 `.rs` 过去都会以布局消息
/// panic，使工具在真实 crate 上不可用：`src/helpers.rs` 并不是错误。
#[test]
fn an_ordinary_flat_module_is_not_a_registration_source() {
    let (root, manifest) = host_root("flat-module");
    std::fs::write(
        manifest.join("src/helpers.rs"),
        "pub fn helper() -> u32 { 1 }\n",
    )
    .expect("write module");
    let outcome = crate::run_for(&manifest, &root.join("out"), "test-host");
    assert!(
        outcome.is_ok(),
        "an ordinary module must be skipped, not refused: {outcome:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A file that *is* a registration face but sits outside `<name>/<name>.rs`
/// is reported with its path. The build can never compile it, so silence
/// would be a face that silently never registers.
/// 确实是注册面、却长在 `<name>/<name>.rs` 之外的文件会带着路径被报告。构建永远编译
/// 不到它，沉默就意味着一个静默地从未注册的面。
#[test]
fn a_face_outside_the_layout_is_reported_with_its_path() {
    let (root, manifest) = host_root("misplaced-face");
    std::fs::write(
        manifest.join("src/flat.rs"),
        "crate::root_object! {\n    kind: Flat,\n}\n",
    )
    .expect("write face");
    let error = crate::run_for(&manifest, &root.join("out"), "test-host")
        .expect_err("a face outside the layout must fail the build");
    assert!(error.contains("flat.rs"), "the file must be named: {error}");
    assert!(
        error.contains("face-layout"),
        "the diagnostic must say what is wrong: {error}"
    );
    assert!(
        error.contains("<name>/<name>.rs"),
        "the diagnostic must say where the file belongs: {error}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A generated tree that cannot be written is a diagnostic, not a panic: a
/// structured caller reports it with the rest, while a build script stops
/// with the reason, having nowhere to render one.
/// 写不成的生成树是诊断而不是 panic：结构化调用方把它与其余诊断一起报出，而构建脚本
/// 带着原因停下——它没有地方渲染诊断。
#[test]
fn an_unwritable_generated_tree_is_reported_not_fatal() {
    let (root, manifest) = host_root("unwritable-out");
    let out = root.join("out");
    std::fs::create_dir_all(&out).expect("out directory");
    // A directory where the fingerprint file belongs: the path exists, so this
    // is a write failure rather than a missing parent.
    // 指纹文件的位置放一个目录：路径存在，因此这是写失败而不是父目录缺失。
    std::fs::create_dir_all(out.join("discovery.fingerprint")).expect("blocking directory");

    let error = crate::run_for(&manifest, &out, "test-host")
        .expect_err("an unwritable generated tree must fail the run");
    assert!(
        error.contains("out-dir"),
        "the failure is a diagnostic: {error}"
    );
    assert!(error.contains("cannot write"), "{error}");
    assert!(error.contains("discovery.fingerprint"), "{error}");

    let _ = std::fs::remove_dir_all(&root);
}

/// A face file that does not parse is reported, not fatal: the same run also
/// reports what it costs the entry reader, and `check --json` gets a document.
/// Before this the entry reader panicked first, so the build died with exit
/// 101 and an empty stdout.
/// 解析不了的注册面文件被报告，而不是致命：同一次运行还会报出它给入口读取器带来的
/// 代价，而 `check --json` 拿到文档。在这之前入口读取器先 panic，于是构建以退出 101
/// 与空白 stdout 死掉。
#[test]
fn a_malformed_face_is_reported_instead_of_aborting() {
    let (root, manifest) = host_root("malformed-face");
    std::fs::create_dir_all(manifest.join("src/broken")).expect("face folder");
    std::fs::write(
        manifest.join("src/broken/broken.rs"),
        "crate::root_object! {\n    kind: Broken\n",
    )
    .expect("write broken face");
    let error = crate::run_for(&manifest, &root.join("out"), "test-host")
        .expect_err("a malformed face must fail the build");
    assert!(
        error.contains("face-syntax"),
        "the face diagnostic must be there: {error}"
    );
    assert!(
        error.contains("broken.rs"),
        "the file must be named: {error}"
    );
    assert!(
        error.contains("entry"),
        "the entry reader must report it too, not abort: {error}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
