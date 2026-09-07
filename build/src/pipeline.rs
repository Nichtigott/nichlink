use super::diagnostics::BuildDiagnostics;
use super::{
    aggregate_contract_errors, aggregate_parent_macro_errors, aggregate_requirements,
    aggregate_stable_name_errors, cache_directory, discover_root, emit_rerun_paths,
    materialize_sources, prime_node_id_cache, render_lib, static_plan, update_discovery_cache,
    write_function_manifest, write_if_changed, write_pruning_manifest, write_source_scope_manifest,
    BuildInput, SourceScope,
};

pub(crate) fn run(input: &BuildInput) {
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
    let generated = render_lib(
        src,
        &nodes,
        &compile_errors,
        &demo_errors,
        &scope,
        &static_faces,
    );
    let out_dir = &input.out_dir;
    write_if_changed(
        &out_dir.join("discovery.fingerprint"),
        &discovery_fingerprint,
    );
    println!(
        "cargo:warning=nichlink discovery cache {cache_state} ({})",
        discovery_fingerprint
    );
    materialize_sources(src, &nodes, out_dir);
    write_pruning_manifest(src, &nodes, out_dir);
    write_function_manifest(src, &nodes, out_dir);
    write_source_scope_manifest(src, &nodes, &scope, out_dir);
    write_if_changed(&out_dir.join("generated_lib.rs"), &generated);
    emit_rerun_paths(src, &nodes);
    println!("cargo:rerun-if-env-changed=NICH_LINK_SCOPE");
    println!("cargo:rerun-if-env-changed=NICH_LINK_ENTRY");
    println!(
        "cargo:rerun-if-changed={}",
        manifest.join("Cargo.toml").display()
    );
}

/// Keep the generated diagnostic stream deterministic and compact.
/// 保持生成的诊断流稳定且紧凑。
fn append_error(target: &mut BuildDiagnostics, addition: BuildDiagnostics) {
    target.extend(addition);
}
