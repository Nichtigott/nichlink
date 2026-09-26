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
    // Two bases, resolved once: `scan` is the tree the walk reads, and `src` is
    // what an identity path is taken relative to. They are the same directory for
    // every package that keeps its sources under `src/`, and differ for one whose
    // library target lives elsewhere (`source_layout` explains why, and why the
    // identity answer has to match the declaration macros).
    // 两个基准只解析一次：`scan` 是遍历读取的树，`src` 是身份路径所相对的基准。对每个把源码放在
    // `src/` 下的包，它们是同一个目录；而库目标住在别处的包会让它们分开（`source_layout` 解释了
    // 原因，以及为什么身份那一半必须与声明宏一致）。
    let layout = match &input.layout {
        Ok(layout) => layout,
        // A layout the resolver had to refuse — a `[lib] path` naming no file — is
        // reported exactly where a missing tree is, and with the same
        // consequences: `check --json` writes the document, the build script stops
        // with the rendered text.
        // 解析器不得不拒绝的布局——`[lib] path` 指不到文件——报到"源树缺失"所报的同一处，后果也
        // 相同：`check --json` 写出文档，构建脚本带着渲染后的文本停下。
        Err(message) => return layout_diagnostic(input, message.clone()),
    };
    let scan = &layout.scan_root;
    let src = &layout.identity_base;
    // A package with no source tree — or with something that is not a directory
    // where one is expected — is a layout diagnostic, and nothing below runs on a
    // tree that cannot be read. This used to reach
    // `expect("src directory must exist")` inside discovery: the build script died
    // with exit 101 and `check --json` printed nothing at all, which is the
    // contract that command was fixed to keep.
    // 没有源树（或该有目录的地方放着别的东西）的包得到一条布局诊断，下面任何步骤都不会在一个
    // 读不到的树上运行。这里过去会走到 discovery 里的 `expect("src directory must exist")`：
    // 构建脚本以退出 101 死掉，而 `check --json` 什么都不打印——而那正是那条命令被修好要守住的
    // 契约。
    if !scan.is_dir() {
        return layout_diagnostic(
            input,
            format!(
                "{} is not a source directory; the build reads registration faces from it, \
                 and a missing tree is not an empty one",
                scan.display()
            ),
        );
    }
    let mut unplaced = Vec::new();
    let nodes = discover_root_reporting(scan, &mut unplaced);
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
    let entry = super::host_entry_from_environment(layout, &nodes, &mut early_errors);
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
        emit_rerun_paths(scan, &nodes);
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

/// Report a source layout the build cannot read, the way both callers need it.
/// 以两种调用方各自需要的方式报告构建读不了的源码布局。
///
/// A build script has no stdout contract to keep and no generated tree to carry
/// the message — the tree is what would have carried it — so it fails loudly with
/// the rendered diagnostic, the way the write path below fails. What it replaces is
/// a bare `expect("src directory must exist")` that named neither the tree nor the
/// reason.
/// 构建脚本没有 stdout 契约要守，也没有生成树可以承载这条消息——生成树正是本该承载它的东西——
/// 因此它带着渲染后的诊断响亮失败，与下面的写出路径一致。它取代的是一条既没说清哪棵树、也没说清
/// 原因的裸 `expect("src directory must exist")`。
fn layout_diagnostic(input: &BuildInput, message: String) -> Option<BuildDiagnostics> {
    let mut diagnostics = BuildDiagnostics::default();
    diagnostics.push(BuildDiagnostic::new("face-layout", message));
    if input.emit_cargo_directives {
        panic!("{}", diagnostics.render());
    }
    Some(diagnostics)
}

#[cfg(test)]
#[path = "pipeline_tests.rs"]
mod pipeline_tests;
