use super::diagnostics::{BuildDiagnostic, BuildDiagnostics};
use super::{
    BuildInput, SourceScope, aggregate_contract_errors, aggregate_parent_macro_errors,
    aggregate_requirements, aggregate_stable_name_errors, cache_directory, discover_root_reporting,
    emit_rerun_paths, face_syntax_errors, graft_plan_check, prime_node_id_cache, render_lib,
    static_plan, unplaced_face_errors, update_discovery_cache, write_file_manifest,
    write_function_manifest, write_generation, write_graft_manifest, write_graph_manifest,
    write_if_changed, write_pruning_manifest, write_shape_manifest, write_source_scope_manifest,
};
use nichlink_kernel::lexicon;

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
    let discovery_fingerprint = super::discovery_fingerprint(src, scan, &nodes);
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
    // The crate shape is an input to the **render** as well as to the release checks: a subtree the
    // declaration hands to another crate must not be compiled here a second time, so the host's
    // generated tree emits no module for it while the records below still carry every face (the tree
    // is unchanged; what changes is which crate compiles it). Read once, used twice — audit `M7`, P3.2.
    // crate 形状既是发布校验的输入，也是**渲染**的输入：被声明交给另一个 crate 的子树不能在这里再编译一遍，
    // 因此宿主的生成树不为它发射模块，而下面的记录仍带着每一个面（树没变，变的是"由哪个 crate 编译"）。
    // 读一次、用两次——审计 `M7`，P3.2。
    let shape = match super::shape_decl::read_shape_declaration(&layout.package_root) {
        Ok(shape) => shape,
        Err(refusal) => {
            compile_errors.push(BuildDiagnostic::new("add-crates", refusal));
            None
        }
    };
    // The render mode comes first, because it decides **which** subtrees this run must not render: a
    // ghost is the crate the declared subtrees were handed to, so it hands nothing away.
    // 先定渲染模式，因为它决定本次运行**不得**渲染哪些子树：幽灵正是那些被交出去的子树所交给的 crate，因此它
    // 什么都不交出去。
    let only = std::env::var(nichlink_kernel::lexicon::SHAPE_ONLY_ENV)
        .ok()
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .map(str::to_owned)
                .collect::<Vec<String>>()
        });
    let cut_out = super::crate_plan::cut_out_for(shape.as_ref(), only.is_some());
    // The plan is computed **here**, before the render, because the render needs what it produces: the
    // mount spelling per face. A ghost's files live in the host package, so the spelling has to walk
    // out of the ghost and into the host's `src` — and `file!()` reports that spelling, which makes it
    // part of the identity (audit `M7`, P3.2). A plan this build cannot make is refused by name here,
    // which is also where the generated tree can carry the refusal.
    // 规划在**这里**、渲染之前算，因为渲染需要它的产物：每个面的挂载拼写。幽灵的文件住在宿主包里，因此拼写
    // 必须走出幽灵、走进宿主的 `src`——而 `file!()` 报告的就是那个拼写，于是它是身份的一部分（审计 `M7`，
    // P3.2）。本次构建做不出的规划在这里被点名拒绝，而生成树也正好能承载那条拒绝。
    let shape_faces: Vec<(String, String)> = static_faces
        .iter()
        .map(|face| (face.source.clone(), face.module.clone()))
        .collect();
    let mounts: Vec<(String, String)> = match &shape {
        Some(shape) => match super::crate_plan::plan(
            &layout.package_root,
            &super::registry_identity::package_namespace(),
            shape,
            &shape_faces,
        ) {
            Ok(planned) => planned
                .iter()
                .flat_map(|crate_plan| {
                    crate_plan
                        .mounts
                        .iter()
                        .map(|mount| (mount.module_path.clone(), mount.spelling.clone()))
                })
                .collect(),
            Err(refusal) => {
                compile_errors.push(BuildDiagnostic::new("add-crates", refusal));
                Vec::new()
            }
        },
        None => Vec::new(),
    };
    // Both modes come from one shape, read once: a host renders all but what it hands away, a ghost
    // renders only its fragment (audit `M7`, P3.2).
    // 两种模式出自同一份形状，只读一次：宿主渲染除交出去之外的全部，幽灵只渲染自己的碎片（审计 `M7`，P3.2）。
    let generated = render_lib(
        src,
        &nodes,
        &compile_errors,
        &demo_errors,
        &scope,
        &static_faces,
        &graft_entries.enabled,
        if only.is_none() && cut_out.is_empty() {
            // No declaration at all: this crate renders the whole tree, which is what every host did
            // before `add_crates.rs` existed.
            // 完全没有声明：本 crate 渲染整棵树，也就是 `add_crates.rs` 存在之前每个宿主的样子。
            super::renderer::ShapeRender::whole()
        } else {
            super::renderer::ShapeRender {
                cut_out: &cut_out,
                only: only.as_deref(),
                mounts: &mounts,
            }
        },
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
    if input.emit_cargo_directives
        && let Some(status) = cache_status_line(
            &cache_state,
            &discovery_fingerprint,
            build_output_is_verbose(),
        )
    {
        println!("cargo:warning={status}");
    }
    // The payloads are published first; the fingerprint is written below, once
    // every one of them has landed. The fingerprint is the token
    // `build_output_is_current` reads, so only a clean run may write it: a failed
    // run publishes no token at all, and a reader then asks the build instead of
    // trusting output that run left behind. Writing the token first — which is what
    // this did — leaves a new token beside a manifest that is old or missing when a
    // payload write fails, and the next reader calls that mixed generation
    // `current` and trusts its rows; the pruning column is what a maintainer reads
    // before a release prunes. The prose here always required "a clean run", but the
    // check looked at the diagnostics only, never at the writes.
    // 载荷先发布；指纹在下面、它们全部落地之后才写。指纹是 `build_output_is_current` 读取的那枚
    // 凭据，因此只有干净的一次运行才能写它：失败的一次运行不发布任何凭据，读取方于是去问构建，而不是
    // 相信那次运行留下的产物。先写凭据——也就是这里过去做的事——会在某份载荷写失败时留下"新凭据 +
    // 旧或缺的清单"，下一个读取方把这种混代产物称作 `current` 并相信它的行；而修剪列正是维护者在发布
    // 剪枝前读的东西。这段说明一直要求"干净的一次运行"，但检查只看诊断，从不看写入。
    // The record's rows are produced **once** and published twice: `pruning_manifest.tsv` is their
    // record form, and `graph_edges.tsv` is the same rows seen as the graph (audit `M7`, P1.1).
    // Reading the manifest back to build the graph would be a second parse of the same fifty
    // thousand lines, and it would also let the two files disagree about a face.
    // 记录的行**只产出一次**、发布两次：`pruning_manifest.tsv` 是它们的记录形态，而 `graph_edges.tsv`
    // 是同一批行按图来看的样子（审计 `M7`，P1.1）。把清单读回来建图等于把同一批五万行解析第二遍，而且会让
    // 两份文件对一个面产生分歧。
    // One writer per tree for the duration of the **payload set** (audit `M7`, P3.4-lock): each file
    // below is atomic on its own, but the set is not, and two processes publishing one tree would let
    // the later one win file by file. The lock is taken here rather than around the whole run because
    // everything above is computation (discovery, parsing, rendering) — idempotent, and the expensive
    // part — while what must not interleave is this write phase.
    // 在**载荷集合**的整个期间一棵树只有一个写者（审计 `M7`，P3.4-lock）：下面每一份文件各自原子，但这一组
    // 不是，而两个进程发布同一棵树会让后写者逐份取胜。锁在这里取、而不是包住整次运行，因为上面的全是计算
    // （发现、解析、渲染）——幂等，而且是昂贵的部分——不许交错的是这段写入。
    let publish_lock = match super::publish_lock::acquire(out_dir) {
        Ok(lock) => Some(lock),
        Err(refusal) => {
            // A run that cannot take the tree's lock publishes **nothing**: half a generation beside
            // another run's files is exactly the state the lock exists to prevent.
            // 拿不到这棵树的锁的运行**什么都不发布**：半代产物躺在另一次运行的文件旁边，正是这把锁存在的
            // 理由。
            compile_errors.push(BuildDiagnostic::new("publish-lock", refusal));
            None
        }
    };
    if let Some(lock) = &publish_lock
        && let Some(pid) = lock.stolen_from
        && input.emit_cargo_directives
    {
        println!(
            "cargo:warning=removed a stale publish lock left by pid {pid} (that process is gone)"
        );
    }
    if publish_lock.is_some() {
        let mut pruning_rows = Vec::new();
        match write_pruning_manifest(src, &nodes, out_dir) {
            Ok(rows) => pruning_rows = rows,
            Err(error) => write_errors.push(error),
        }
        // The declarations are the second half of the graph's raw material: a called name that exactly
        // one file declares is a dependency on that file, and the graph carries that edge rather than
        // making every reader re-derive it (audit `M7`, P1.1).
        // 声明是图的另一半原料：一个恰好被一份文件声明的被调用名字，就是对那份文件的一处依赖，而图直接带上
        // 那条边，而不是让每个读者重新推导（审计 `M7`，P1.1）。
        let mut file_rows = Vec::new();
        match write_file_manifest(src, &nodes, out_dir) {
            Ok(rows) => file_rows = rows,
            Err(error) => write_errors.push(error),
        }
        let mut graph = None;
        for result in [
            write_function_manifest(src, &nodes, out_dir),
            write_source_scope_manifest(src, &nodes, &scope, out_dir),
            write_shape_manifest(src, &nodes, out_dir),
            write_graft_manifest(out_dir, &graft_entries.enabled),
            write_if_changed(&out_dir.join(lexicon::GENERATED_LIB_FILE), &generated),
        ] {
            if let Err(error) = result {
                write_errors.push(error);
            }
        }
        match write_graph_manifest(out_dir, &pruning_rows, &file_rows, &graft_entries.enabled) {
            Ok(header) => graph = Some(header),
            Err(error) => write_errors.push(error),
        }
        // The crate-shape declaration is a build input like any face: a shape that does not hold
        // together fails the build here, before anything is published, and its lock is written the same
        // way. A host without a declaration is not an error — it is the one-crate package every host was
        // before this file existed.
        // crate 形状声明与任何注册面一样是构建输入：不成立的形状在这里、在发布任何东西之前让构建失败，而它的锁
        // 以同样的方式写下。没有声明的宿主不是错误——它就是这份文件存在之前每个宿主的样子：一个 crate 的包。
        if compile_errors.is_empty()
            && let Some(shape) = &shape
        {
            // The declaration is validated first (does every claim name a subtree?), then planned
            // (can every claimed subtree become a crate that compiles?). Refusing a declaration this
            // build cannot turn into a working crate is the difference between a named refusal now
            // and a subtree nobody compiles, which rustc reports later as `cannot find 'object' in
            // 'control'` (audit `M7`, P3.2).
            // 先校验声明（每条认领都点名了一棵子树吗？），再规划它（每条认领的子树都能变成一个编译得过的
            // crate 吗？）。拒绝一份本次构建变不成可用 crate 的声明，就是"现在得到一句点名拒绝"与"留下一棵
            // 没人编译的子树、以后由 rustc 报 `cannot find 'object' in 'control'`"之间的差别（审计 `M7`，P3.2）。
            // The plan was already made before the render (it produced the mount spellings); here the
            // declaration's claims are checked against the rows, which is the half that needs them.
            // 规划已在渲染之前做过（它产出了挂载拼写）；这里拿行对账声明里的认领，那是需要行的那一半。
            if let Err(refusal) = super::shape_decl::check_shape(shape, out_dir, &pruning_rows) {
                compile_errors.push(BuildDiagnostic::new("add-crates", refusal));
            }
        }
        if compile_errors.is_empty() && write_errors.is_empty() {
            if let Err(error) = write_if_changed(
                &out_dir.join("discovery.fingerprint"),
                &discovery_fingerprint,
            ) {
                write_errors.push(error);
            }
            // The readiness record is the **last** thing a clean run writes (audit `M7`, P2.1): a reader
            // that finds it knows this run finished, and one that does not knows the previous run (or
            // none) is what is on disk. It is written after the fingerprint for the same reason the
            // fingerprint is written last among the payloads.
            // 就绪记录是干净的一次运行写的**最后**一样东西（审计 `M7`，P2.1）：找到它的读者知道这次运行完成了，
            // 而找不到的读者知道磁盘上是上一次（或者没有）。它排在指纹之后，理由与指纹排在载荷之后相同。
            if let Some(header) = &graph
                && let Err(error) = write_generation(out_dir, &layout.package_root, header)
            {
                write_errors.push(error);
            }
        } else {
            // A token that cannot be removed still claims this output describes the
            // current sources, so the failure is reported rather than discarded: the
            // previous `let _ =` left exactly that claim standing.
            // 删不掉的凭据仍然声称这份产物描述的是当前源码，因此这次失败被报出而不是被丢弃：过去的
            // `let _ =` 恰恰让那句话继续成立。
            match std::fs::remove_file(out_dir.join("discovery.fingerprint")) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => write_errors.push(format!(
                    "cannot remove {}: {error}",
                    out_dir.join("discovery.fingerprint").display()
                )),
            }
        }
    } else {
        // Nothing was published, and the diagnostic above says which lock to clear.
        // 什么都没发布，上面那条诊断说明了该清哪把锁。
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
        // The demo directory's diagnostics are gated behind this feature, so a host
        // that declares it would otherwise be told the cfg is unexpected. rustc's
        // `check-cfg` takes the value-list form for a feature name; the name comes
        // from the same constant the renderer writes into the generated tree.
        // 演示目录的诊断挂在这个特性之后，因此声明了它的宿主否则会被通知该 cfg 是意外的。对特性名，
        // rustc 的 `check-cfg` 采用值列表形式；这个名字取自渲染器写进生成树的同一个常量。
        println!(
            "cargo::rustc-check-cfg=cfg(feature, values({:?}))",
            crate::build_time::DEMO_ONLY_FEATURE
        );
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
        panic!("{}", diagnostics.render_build_diagnostics());
    }
    Some(diagnostics)
}

#[cfg(test)]
#[path = "pipeline_tests.rs"]
mod pipeline_tests;
