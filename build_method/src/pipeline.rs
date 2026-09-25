use super::diagnostics::{BuildDiagnostic, BuildDiagnostics};
use super::{
    BuildInput, SourceScope, aggregate_contract_errors, aggregate_parent_macro_errors,
    aggregate_requirements, aggregate_stable_name_errors, cache_directory, discover_root_reporting,
    emit_rerun_paths, face_syntax_errors, graft_plan_check, prime_node_id_cache, render_lib,
    static_plan, unplaced_face_errors, update_discovery_cache, write_function_manifest,
    write_graft_manifest, write_if_changed, write_pruning_manifest, write_source_scope_manifest,
};
use nichlink::lexicon;

pub(crate) fn run(input: &BuildInput) -> Option<BuildDiagnostics> {
    let manifest = &input.manifest;
    let src = &input.src;
    let mut unplaced = Vec::new();
    let nodes = discover_root_reporting(src, &mut unplaced);
    prime_node_id_cache(manifest, src, &nodes);
    let discovery_fingerprint = super::discovery_fingerprint(src, &nodes);
    // One entry, resolved once, for both readers below. Pruning and the generated
    // cut table must describe the same file, and they silently stopped doing so
    // when each resolved the entry on its own — `NICH_LINK_ENTRY` reached only
    // pruning, so the release could prune one file's slots while the runtime cut
    // table described another's. `host_entry_from_environment` also panics here
    // if the variable names something that is not a file, rather than letting one
    // reader fall back while the other follows it.
    // 入口只解析一次，供下面两个读取者共用。剪枝与生成的切口表必须描述同一个文件，
    // 而它们各自解析入口时就静默地不再一致——`NICH_LINK_ENTRY` 只作用于剪枝，发布态
    // 可能剪掉一个文件的槽位，运行期切口表却在描述另一个文件。若变量指的不是文件，
    // `host_entry_from_environment` 也在这里直接 panic，而不是让一个读取者回退、另一个
    // 跟随。
    // Resolving the entry and the scope can refuse a configured value, and those
    // refusals are diagnostics now: they used to panic, which took the build
    // script down and left `check --json` printing nothing at all.
    // 解析入口与范围可能拒绝一个被配置的取值，而这些拒绝现在是诊断：它们过去会 panic，
    // 那会打死构建脚本，并让 `check --json` 什么都不打印。
    let mut early_errors = BuildDiagnostics::default();
    let entry = super::host_entry_from_environment(src, &nodes, &mut early_errors);
    let scope = SourceScope::from_environment(src, &nodes, &entry, &mut early_errors);
    let cache_units = cache_directory(manifest).join("units");
    let cache_state = update_discovery_cache(manifest, src, &nodes, &discovery_fingerprint);

    // Faces the build cannot place or parse are reported first: the stages below
    // decode fields from a parsed face, and they skip a file they cannot parse, so
    // this diagnostic is the only thing a reader would otherwise never see. Before
    // this existed they panicked instead, which took the whole build script down
    // and left `check --json` with an empty stdout.
    // 构建安放不了或解析不了的面先被报告：下面的阶段从已解析的面里解码字段，它们会跳过
    // 自己解析不了的文件，因此这条诊断是读者本来永远看不到的东西。在这之前它们会 panic，
    // 那会打死整个构建脚本，并让 `check --json` 的 stdout 一片空白。
    let mut compile_errors = early_errors;
    append_error(&mut compile_errors, unplaced_face_errors(&unplaced));
    append_error(&mut compile_errors, face_syntax_errors(src, &nodes));
    append_error(
        &mut compile_errors,
        aggregate_requirements(src, &nodes, false, &scope, Some(&cache_units)),
    );
    let contract_errors = aggregate_contract_errors(src, &nodes, false, &scope);
    append_error(&mut compile_errors, contract_errors);
    let stable_errors = aggregate_stable_name_errors(src, &nodes);
    append_error(&mut compile_errors, stable_errors);
    let parent_macro_errors = aggregate_parent_macro_errors(src, &nodes);
    append_error(&mut compile_errors, parent_macro_errors);
    let demo_errors = aggregate_requirements(src, &nodes, true, &scope, Some(&cache_units));
    let (static_faces, static_errors) = static_plan(src, &nodes, &scope);
    append_error(&mut compile_errors, static_errors);
    let graft_entries = super::host_graft_entries(&entry, &mut compile_errors);
    // A plan file is an authoring record the build never reads, so a plan whose
    // slot no declaration names would otherwise be discovered at runtime, long
    // after the build pruned it. This is the one case that is unambiguous *and*
    // decidable here: the entry's full declaration list is in hand, so a slot
    // that only a gated declaration names is not reported (that configuration is
    // legitimate — the gate is the author's), while a slot nothing can name is a
    // build failure rather than a warning, because shipping a binary whose graft
    // silently never happens is exactly what must not leave the build.
    // 计划文件是构建从不读取的创作记录，因此没有任何声明命名其槽位的计划，只会在运行期
    // 才被发现——那时构建早已把它剪掉。这里是唯一既无歧义、又能在构建期判定的一类：入口
    // 的完整声明清单就在手里，因此只有门控声明命名的槽位不会被上报（那是合法配置——门控
    // 是作者的事），而没有任何声明可能命名的槽位是构建失败而不只是警告，因为"发布一个
    // 嫁接静默地从未发生的二进制"正是绝不能离开构建的东西。
    let plan_errors = graft_plan_check::undeclared_plan_errors(
        &graft_plan_check::planned_slots(manifest),
        &graft_entries
            .declared
            .iter()
            .map(super::declared_graft_view)
            .collect::<Vec<_>>(),
        |id| graft_plan_check::slot_module(src, &nodes, id),
    );
    append_error(&mut compile_errors, plan_errors);
    let generated = render_lib(
        src,
        &nodes,
        &compile_errors,
        &demo_errors,
        &scope,
        &static_faces,
        &graft_entries.enabled,
    );
    let out_dir = &input.out_dir;
    // Write failures are collected rather than fatal here. They are the one
    // failure the generated tree cannot carry, because the file that would carry
    // it is exactly what could not be written, so the two callers are told
    // differently below: a build script stops with the reason, and a structured
    // caller (`check --json`) reports it with the rest of the diagnostics.
    // 写失败在这里被收集而不是立即致命。它们是生成树唯一无法承载的失败——本该承载它的文件
    // 正是写不成的那个——因此下面按两种调用方分别处理：构建脚本带着原因停下，
    // 结构化调用方（`check --json`）把它与其余诊断一起报出。
    let mut write_errors = Vec::new();
    // The fingerprint is the token `build_output_is_current` reads, so only a clean
    // run writes it: a failed run publishes no token at all, and a reader then asks
    // the build instead of trusting output that run left behind. The other
    // manifests stay where they are — without the token nothing reads them as
    // describing the current sources.
    // 指纹是 `build_output_is_current` 读取的那枚凭据，因此只有干净的一次运行才写下它：失败的
    // 一次运行不发布任何凭据，读取方于是去问构建，而不是相信那次运行留下的产物。其余清单留在
    // 原处——没有那枚凭据，没有任何东西会把它们读作"描述了当前源码"。
    if compile_errors.is_empty() {
        if let Err(error) = write_if_changed(
            &out_dir.join("discovery.fingerprint"),
            &discovery_fingerprint,
        ) {
            write_errors.push(error);
        }
    } else {
        let _ = std::fs::remove_file(out_dir.join("discovery.fingerprint"));
    }
    if input.emit_cargo_directives
        && let Some(status) = cache_status_line(
            &cache_state,
            &discovery_fingerprint,
            build_output_is_verbose(),
        )
    {
        println!("cargo:warning={status}");
    }
    for result in [
        write_pruning_manifest(src, &nodes, out_dir),
        write_function_manifest(src, &nodes, out_dir),
        write_source_scope_manifest(src, &nodes, &scope, out_dir),
        write_graft_manifest(out_dir, &graft_entries.enabled),
        write_if_changed(&out_dir.join(lexicon::GENERATED_LIB_FILE), &generated),
    ] {
        if let Err(error) = result {
            write_errors.push(error);
        }
    }
    if !write_errors.is_empty() {
        let message = format!(
            "nichlink could not write its generated tree: {}",
            write_errors.join("; ")
        );
        if input.emit_cargo_directives {
            // The build-script path has nowhere to render this: the generated tree
            // is what would carry it. A build script fails by panicking, which is
            // how a missing `OUT_DIR` write has always failed here.
            // 构建脚本路径没有地方渲染它：生成树正是本该承载它的东西。构建脚本以 panic
            // 失败，这也是此处 `OUT_DIR` 写不了时一向的失败方式。
            panic!("{message}");
        }
        for error in write_errors {
            compile_errors.push(BuildDiagnostic::new("out-dir", error));
        }
    }
    if input.emit_cargo_directives {
        emit_rerun_paths(src, &nodes);
        // The generated tree carries `cfg(rust_analyzer)` declarations that give
        // rust-analyzer a top-level view of every nested face file. rustc reads
        // none of them, so declare the cfg name to keep `unexpected_cfgs` quiet.
        // 生成树带有 `cfg(rust_analyzer)` 声明，用于给 rust-analyzer 提供每个嵌套
        // 面文件的顶层视角。rustc 一条都不会读，因此声明该 cfg 名以免
        // `unexpected_cfgs` 报警。
        println!("cargo::rustc-check-cfg=cfg(rust_analyzer)");
        println!("cargo:rerun-if-env-changed={}", lexicon::SCOPE_ENV);
        println!("cargo:rerun-if-env-changed={}", lexicon::ENTRY_ENV);
        println!("cargo:rerun-if-env-changed={}", lexicon::BUILD_VERBOSE_ENV);
        println!(
            "cargo:rerun-if-changed={}",
            manifest.join("Cargo.toml").display()
        );
        // A plan file only feeds the cross-check above, so a new or edited plan
        // has to bring the build back or its diagnostic would never appear.
        // 计划文件只喂给上面那条交叉校验，因此新增或改动计划必须让构建重跑，否则它的
        // 诊断永远不会出现。
        println!(
            "cargo:rerun-if-changed={}",
            manifest
                .join(lexicon::NICHLINK_DIR)
                .join(lexicon::EXTERNAL_GRAFT_DIR)
                .display()
        );
    }
    if compile_errors.is_empty() {
        None
    } else {
        Some(compile_errors)
    }
}

fn build_output_is_verbose() -> bool {
    std::env::var_os(lexicon::BUILD_VERBOSE_ENV).is_some_and(|value| !value.is_empty())
}

fn cache_status_line(cache_state: &str, fingerprint: &str, verbose: bool) -> Option<String> {
    verbose.then(|| format!("nichlink discovery cache {cache_state} ({fingerprint})"))
}

/// Keep the generated diagnostic stream deterministic and compact.
/// 保持生成的诊断流稳定且紧凑。
fn append_error(target: &mut BuildDiagnostics, addition: BuildDiagnostics) {
    target.extend(addition);
}

#[cfg(test)]
mod tests {
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
            let root =
                std::env::temp_dir().join(format!("nichlink-hidden-{tag}-{suffix}-{sequence}"));
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
        let error = crate::run_for(&manifest, &out, "test-host")
            .expect_err("duplicate stable_name must fail");
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
}
