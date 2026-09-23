use super::diagnostics::BuildDiagnostics;
use super::{
    BuildInput, SourceScope, aggregate_contract_errors, aggregate_parent_macro_errors,
    aggregate_requirements, aggregate_stable_name_errors, cache_directory, discover_root,
    emit_rerun_paths, graft_plan_check, prime_node_id_cache, render_lib, static_plan,
    update_discovery_cache, write_function_manifest, write_graft_manifest, write_if_changed,
    write_pruning_manifest, write_source_scope_manifest,
};
use nichlink::lexicon;

pub(crate) fn run(input: &BuildInput) -> Option<BuildDiagnostics> {
    let manifest = &input.manifest;
    let src = &input.src;
    let nodes = discover_root(src);
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
    let entry = super::host_entry_from_environment(src, &nodes);
    let scope = SourceScope::from_environment(src, &nodes, &entry);
    let cache_units = cache_directory(manifest).join("units");
    let cache_state = update_discovery_cache(manifest, src, &nodes, &discovery_fingerprint);

    let mut compile_errors = aggregate_requirements(src, &nodes, false, &scope, Some(&cache_units));
    let contract_errors = aggregate_contract_errors(src, &nodes, false, &scope);
    append_error(&mut compile_errors, contract_errors);
    let stable_errors = aggregate_stable_name_errors(src, &nodes);
    append_error(&mut compile_errors, stable_errors);
    let parent_macro_errors = aggregate_parent_macro_errors(src, &nodes);
    append_error(&mut compile_errors, parent_macro_errors);
    let demo_errors = aggregate_requirements(src, &nodes, true, &scope, Some(&cache_units));
    let (static_faces, static_errors) = static_plan(src, &nodes, &scope);
    append_error(&mut compile_errors, static_errors);
    let graft_entries = super::host_graft_entries(&entry);
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
    write_if_changed(
        &out_dir.join("discovery.fingerprint"),
        &discovery_fingerprint,
    );
    if input.emit_cargo_directives
        && let Some(status) = cache_status_line(
            &cache_state,
            &discovery_fingerprint,
            build_output_is_verbose(),
        )
    {
        println!("cargo:warning={status}");
    }
    write_pruning_manifest(src, &nodes, out_dir);
    write_function_manifest(src, &nodes, out_dir);
    write_source_scope_manifest(src, &nodes, &scope, out_dir);
    write_graft_manifest(out_dir, &graft_entries.enabled);
    write_if_changed(&out_dir.join(lexicon::GENERATED_LIB_FILE), &generated);
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
}
