use super::diagnostics::BuildDiagnostics;
use super::{
    BuildInput, SourceScope, aggregate_contract_errors, aggregate_parent_macro_errors,
    aggregate_requirements, aggregate_stable_name_errors, cache_directory, discover_root,
    emit_rerun_paths, materialize_sources, prime_node_id_cache, render_lib, static_plan,
    update_discovery_cache, write_function_manifest, write_graft_manifest, write_if_changed,
    write_pruning_manifest, write_source_scope_manifest,
};

pub(crate) fn run(input: &BuildInput) -> Option<String> {
    let manifest = &input.manifest;
    let src = &input.src;
    let nodes = discover_root(src);
    prime_node_id_cache(manifest, src, &nodes);
    let discovery_fingerprint = super::discovery_fingerprint(src, &nodes);
    let scope = SourceScope::from_environment(src, &nodes);
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
    let grafts = super::host_graft_entries(src, &nodes);
    let generated = render_lib(
        src,
        &nodes,
        &compile_errors,
        &demo_errors,
        &scope,
        &static_faces,
        &grafts,
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
    materialize_sources(src, &nodes, out_dir);
    write_pruning_manifest(src, &nodes, out_dir);
    write_function_manifest(src, &nodes, out_dir);
    write_source_scope_manifest(src, &nodes, &scope, out_dir);
    write_graft_manifest(out_dir, &grafts);
    write_if_changed(&out_dir.join("generated_lib.rs"), &generated);
    if input.emit_cargo_directives {
        emit_rerun_paths(src, &nodes);
        println!("cargo:rerun-if-env-changed=NICH_LINK_SCOPE");
        println!("cargo:rerun-if-env-changed=NICH_LINK_ENTRY");
        println!("cargo:rerun-if-env-changed=NICH_LINK_BUILD_VERBOSE");
        println!(
            "cargo:rerun-if-changed={}",
            manifest.join("Cargo.toml").display()
        );
    }
    if compile_errors.is_empty() {
        None
    } else {
        Some(compile_errors.render())
    }
}

fn build_output_is_verbose() -> bool {
    std::env::var_os("NICH_LINK_BUILD_VERBOSE").is_some_and(|value| !value.is_empty())
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

    #[test]
    fn successful_build_is_silent_unless_verbose_output_is_requested() {
        assert_eq!(cache_status_line("hit", "abc", false), None);
        assert_eq!(
            cache_status_line("hit", "abc", true).as_deref(),
            Some("nichlink discovery cache hit (abc)")
        );
    }

    #[test]
    fn run_for_validates_outside_cargo_and_reports_diagnostics() {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("nichlink-run-for-{suffix}"));
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
