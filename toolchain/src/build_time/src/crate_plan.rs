//! What a crate-shape declaration turns into: the ghost crates, planned before anything is written.
//! crate 形状声明会变成什么：幽灵 crate，在写下任何东西**之前**先把它们规划出来。
//!
//! A partition needs a crate that compiles a subtree the host no longer compiles (audit `M7`, P3.2).
//! That crate mounts the host's face files with `#[path]` — it does not copy them, because a copy is a
//! second file with the same declarations and a chance for the two to drift. Three facts make the
//! mounting work, and all three are decided here rather than at write time:
//! 一次分区需要一个 crate 去编译宿主不再编译的那棵子树（审计 `M7`，P3.2）。那个 crate 用 `#[path]` 挂载
//! 宿主的**面文件**——不复制它们，因为复制品是第二份带着同样声明的文件，也是一次两边漂移的机会。让挂载成立
//! 的有三件事，而三件都在这里、而不是在写下时才决定：
//!
//! 1. **The namespace is the host's**, so every identity the ghost computes is the id the host's build
//!    baked (a `#![allow]`-free `pub const NICHLINK_NAMESPACE` — the constant P3.3 introduced exactly
//!    for this). Without it the ghost would hash its own package name and silently rename every face.
//!    命名空间是**宿主的**，因此幽灵算出的每个身份就是宿主的构建烤进去的那个 id（一个普通
//!    `pub const NICHLINK_NAMESPACE`——P3.3 正是为它引入的常量）。没有它，幽灵会散列自己的包名，静默
//!    重命名每一个面。
//! 2. **The source path spelling is remapped back to the host's**, because identity is
//!    `hash(namespace, source path, name)` and `file!()` reports the `#[path]` spelling the ghost used.
//!    Each mount therefore carries the prefix to remap away, leaving `file!()` byte-identical to the
//!    host-relative source the host's records name.
//!    源码路径的拼写要**映射回宿主的**，因为身份是 `hash(命名空间, 源码路径, 名字)`，而 `file!()` 报告的
//!    是幽灵所用的 `#[path]` 拼写。因此每个挂载都带着要映射掉的前缀，让 `file!()` 与宿主记录点名的
//!    宿主相对源码逐字节相同。
//! 3. **Nothing outside the fragment is reachable**, which is the one thing a mount cannot fix: the ghost
//!    has inline container modules and the mounted leaves, and no `control.rs` above them. A face that
//!    says `use crate::control::ControlHandle` would not resolve there — and the honest answer is to
//!    **refuse by name** and name the two ways forward (make the fragment self-contained, or partition a
//!    node that contains the definition) rather than emit a crate that does not compile.
//!
//! **碎片之外的东西都够不到**，而这正是挂载修不了的一件事：幽灵有的是内联容器模块与被挂载的叶子，没有它们
//! 上面的 `control.rs`。写着 `use crate::control::ControlHandle` 的面在那里解析不到——而诚实的做法是**点名
//! 拒绝**并说出两条出路（让碎片自足，或分区一个包含该定义的节点），而不是发射一个编译不过的 crate。
//!
//! This module **writes nothing**. It answers "what would be written, and is it sound?" — which is the
//! `--check` half of the authoring action and the input of its `--write` half.
//! 本模块**什么都不写**。它回答"会写下什么、这样成立吗"——那是创作动作的 `--check` 一半，也是它
//! `--write` 一半的输入。

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::registry_identity::NodeId;
use nichlink_kernel::lexicon;

use super::shape_decl::ShapeDeclaration;

/// One ghost crate: the package to create, and the mounts that make it the same faces.
/// 一个幽灵 crate：要创建的包，以及让它成为同一批面的那些挂载。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlannedCrate {
    /// The declared name inside the host's shape (`widgets`).
    /// 宿主形状里的声明名（`widgets`）。
    pub(crate) name: String,
    /// The package name: `<package_prefix>-<name>`.
    /// 包名：`<package_prefix>-<name>`。
    pub(crate) package: String,
    /// Where the ghost package goes: a sibling of the host package.
    /// 幽灵包的位置：宿主包的**同级**。
    pub(crate) directory: PathBuf,
    /// The identity namespace the ghost must define: the host's, so every face keeps its id.
    /// 幽灵必须定义的身份命名空间：**宿主的**，这样每个面都保住自己的 id。
    pub(crate) namespace: String,
    /// The subtree module paths this crate claims.
    /// 这个 crate 认领的子树模块路径。
    pub(crate) subtrees: Vec<String>,
    /// The proper prefixes of the claims that are **faces**, with their identities: a shell module
    /// needs them so a fragment's `parent: crate::<ancestor>::NODE_ID` still resolves in a crate that
    /// does not compile the ancestor. Nothing else of the ancestor's is offered — a trait defined up
    /// there stays unreachable, which is the honest boundary this planner refuses on.
    /// 认领的**是注册面**的真前缀，连同它们的身份：壳模块需要它们，好让碎片里的
    /// `parent: crate::<祖先>::NODE_ID` 在一个不编译祖先的 crate 里仍然解析得到。祖先的其它东西一概不提供
    /// ——定义在上面的 trait 仍不可达，那是本规划器据以拒绝的诚实边界。
    pub(crate) ancestors: Vec<(String, NodeId)>,
    /// One mount per face file below those subtrees.
    /// 那些子树之下的每份面文件一项挂载。
    pub(crate) mounts: Vec<PlannedMount>,
    /// The ghost's `src/lib.rs`: the namespace constant and the include of its own generated tree.
    /// 幽灵的 `src/lib.rs`：命名空间常量，以及它自己那棵生成树的 include。
    pub(crate) lib_rs: String,
    /// The ghost's `build.rs`: run the host's pipeline, rendering only this fragment.
    /// 幽灵的 `build.rs`：跑宿主的管线，只渲染这个碎片。
    pub(crate) build_rs: String,
    /// The ghost's `Cargo.toml`: the host's dependencies, copied verbatim.
    /// 幽灵的 `Cargo.toml`：宿主的依赖，逐字照抄。
    pub(crate) cargo_toml: String,
    /// The `rustflags` entries the **workspace root's** `.cargo/config.toml` has to carry, with the
    /// reason they cannot live in this package's own config (a rustflag is per invocation, and cargo
    /// finds config from the current directory rather than from the package).
    /// **工作区根** `.cargo/config.toml` 必须携带的 `rustflags` 条目，并写明它们为何不能住在这个包自己的
    /// config 里（rustflag 是每次调用的，而 cargo 从当前目录而不是从包出发找 config）。
    pub(crate) config_patch: String,
    /// The mount prefix to remap away, once per distinct prefix (`host/src/` as seen from a leaf's
    /// directory). These are the pairs `--remap-path-prefix` receives, mapped to the empty string so
    /// `file!()` reads as the host-relative source.
    /// 要映射掉的挂载前缀，每个不同的前缀一项（从叶子所在目录看过去的 `host/src/`）。这些就是
    /// `--remap-path-prefix` 收到的对，映射到空串，好让 `file!()` 读起来就是宿主相对源码。
    pub(crate) remap: Vec<(String, String)>,
}

/// One `#[path] pub mod …;` in the ghost's generated tree.
/// 幽灵生成树里的一条 `#[path] pub mod …;`。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlannedMount {
    /// The module path the face must keep (`control::object::button`).
    /// 该面必须保持的模块路径（`control::object::button`）。
    pub(crate) module_path: String,
    /// The host-relative source (`control/object/button/button.rs`) — what the records name.
    /// 宿主相对源码（`control/object/button/button.rs`）——记录点名的那个。
    pub(crate) source: String,
    /// The `#[path]` spelling, written relative to the directory the inline module sits in.
    /// `#[path]` 拼写，写成相对内联模块所在目录的形式。
    pub(crate) spelling: String,
}

/// The line every generated ghost file carries, so a revert can tell its own work from a hand-written
/// package. One home, because the writer that puts it there and the reader that checks it must agree.
/// 每份生成的幽灵文件都带着的那一行，好让撤回能把自己的产物与手写的包区分开。只有一处住址，因为写下它的写入方
/// 与检查它的读取方必须一致。
pub(crate) const GENERATED_MARKER: &str = "Generated by NichLink";

/// The ghost's `src/lib.rs`.
/// 幽灵的 `src/lib.rs`。
///
/// Two items, and the first one is the whole point of P3.3: the ghost declares the **host's** identity
/// namespace, so every face it compiles keeps the id the host's build baked. Its own generated tree
/// carries the mounts; its own `src/` has no faces at all.
/// 两项，而第一项正是 P3.3 的全部意义：幽灵声明的是**宿主的**身份命名空间，因此它编译的每个面都保住宿主构建
/// 烤进去的那个 id。它自己的生成树携带挂载；它自己的 `src/` 一个面都没有。
fn ghost_lib_rs(namespace: &str) -> String {
    format!(
        "//! {marker}: the crate that compiles one fragment of a host's registration\n\
         //! tree. Do not edit — the declaration lives in the host's `add_crates.rs`.\n\
         //! 由 NichLink 生成：编译宿主注册树某一个碎片的 crate。请勿手工修改——声明住在宿主的\n\
         //! `add_crates.rs` 里。\n\n\
         pub const NICHLINK_NAMESPACE: &str = {namespace:?};\n\n\
         include!(concat!(env!(\"OUT_DIR\"), \"/generated_lib.rs\"));\n",
        marker = GENERATED_MARKER
    )
}

/// The ghost's `build.rs`.
/// 幽灵的 `build.rs`。
///
/// It runs the **host's** manifest through the same pipeline and writes this crate's `OUT_DIR`, asking
/// for the fragment only (`NICH_LINK_SHAPE_ONLY`): the sources are the host's, the identities are the
/// host's, and what lands here is the part of the tree this crate is the one to compile.
/// 它把**宿主的**清单交给同一条管线、写本 crate 的 `OUT_DIR`，并且只要那个碎片（`NICH_LINK_SHAPE_ONLY`）：
/// 源码是宿主的、身份是宿主的，落在这里的是这棵树中由本 crate 负责编译的那部分。
fn ghost_build_rs(package_root: &Path, namespace: &str, subtrees: &[String]) -> String {
    let src = package_root.join("src");
    let declaration = package_root.join(lexicon::ADD_CRATES_FILE);
    format!(
        "//! {marker}: build this fragment out of the host's sources.\n\
         //! 由 NichLink 生成：从宿主的源码构建这个碎片。\n\n\
         fn main() {{\n\
         {i}println!(\"cargo:rerun-if-changed={src}\");\n\
         {i}println!(\"cargo:rerun-if-changed={declaration}\");\n\
         {i}let out = std::path::PathBuf::from(std::env::var(\"OUT_DIR\").expect(\"OUT_DIR\"));\n\
         {i}// A build script is single-threaded at this point, and the value is read by the run below.\n\
         {i}// 构建脚本此刻是单线程的，而这个值由下面的那次运行读取。\n\
         {i}unsafe {{ std::env::set_var({env:?}, {claims:?}) }};\n\
         {i}nichlink_toolchain::build_time::run_for(\n\
         {i}    std::path::Path::new({root:?}),\n\
         {i}    &out,\n\
         {i}    {namespace:?},\n\
         {i})\n\
         {i}.expect(\"nichlink\");\n\
         }}\n",
        i = "    ",
        src = src.display(),
        declaration = declaration.display(),
        env = lexicon::SHAPE_ONLY_ENV,
        claims = subtrees.join(","),
        root = package_root.display(),
        namespace = namespace,
        marker = GENERATED_MARKER,
    )
}

/// The ghost's `Cargo.toml`: the host's dependencies, **copied verbatim**.
/// 幽灵的 `Cargo.toml`：宿主的依赖，**逐字照抄**。
///
/// Copied rather than re-derived because the mounted files are the host's: whatever they import has to
/// resolve here exactly as it resolves there, and a hand-written dependency list is a second answer to
/// "what does this source need". `publish = false`: a ghost is a workspace-local build shape, and what
/// a published partition looks like is P3.5's question, not this planner's.
/// 照抄而不是重新推导，因为被挂载的文件就是宿主的：它们 import 什么，在这里就必须和在那里一样解析得到，而
/// 手写的依赖清单是"这份源码需要什么"的第二个答案。`publish = false`：幽灵是工作区本地的构建形状，而发布出去
/// 的分区长什么样是 P3.5 的问题、不是本规划器的。
fn ghost_cargo_toml(package_root: &Path, package: &str) -> Result<String, String> {
    let manifest = package_root.join("Cargo.toml");
    let text = fs::read_to_string(&manifest).map_err(|error| {
        format!(
            "add_crates: cannot read {} to copy its dependencies: {error}",
            manifest.display()
        )
    })?;
    let version = toml_value(&text, "version").unwrap_or_else(|| "0.1.0".to_owned());
    let edition = toml_value(&text, "edition").unwrap_or_else(|| "2024".to_owned());
    let mut output = format!(
        "# Generated by NichLink: the dependencies are the host's, copied verbatim.\n\
         # 由 NichLink 生成：依赖是宿主的，逐字照抄。\n\
         [package]\nname = {package:?}\nversion = {version:?}\nedition = {edition:?}\n\
         publish = false\n"
    );
    for section in ["dependencies", "build-dependencies"] {
        if let Some(block) = toml_section(&text, section) {
            output.push_str(&format!("\n[{section}]\n{block}"));
        }
    }
    Ok(output)
}

/// One `key = "value"` from a manifest's top-level lines, without quotes.
/// 清单顶层某行 `key = "value"` 的值，不含引号。
pub(crate) fn toml_value(text: &str, key: &str) -> Option<String> {
    text.lines()
        .find_map(|line| line.trim().strip_prefix(&format!("{key} = ")))
        .map(|value| value.trim().trim_matches('"').to_owned())
}

/// One manifest section's body, header excluded and trailing blank lines trimmed.
/// 清单里某一节的正文，不含表头、去掉尾部空行。
pub(crate) fn toml_section(text: &str, section: &str) -> Option<String> {
    let header = format!("[{section}]");
    let start = text.lines().position(|line| line.trim() == header)? + 1;
    let body: Vec<&str> = text
        .lines()
        .skip(start)
        .take_while(|line| !line.trim_start().starts_with('['))
        .collect();
    let body = body.join("\n");
    let trimmed = body.trim_end();
    (!trimmed.is_empty()).then(|| format!("{trimmed}\n"))
}

/// The `rustflags` the workspace root has to carry, and why it is the workspace root.
/// 工作区根必须携带的 `rustflags`，以及为什么是工作区根。
fn config_patch(remap: &[(String, String)]) -> String {
    let mut output = String::from(
        "# Add these to the **workspace root's** `.cargo/config.toml` (merge into an existing\n\
         # `rustflags` array; do not overwrite the file). A rustflag is per **invocation**, and cargo\n\
         # finds config from the current directory rather than from the package, so a config inside a\n\
         # ghost package only applies when cargo is run from that directory.\n\
         # 把下面这些**合并**进**工作区根**的 `.cargo/config.toml`（合并到已有的 `rustflags` 数组里，\n\
         # 不要覆盖整个文件）。rustflag 是每次调用生效的，而 cargo 从当前目录而不是从包出发找 config，\n\
         # 因此放在幽灵包里的 config 只在从那个目录跑 cargo 时才生效。\n\
         [build]\n",
    );
    let entries: Vec<String> = remap
        .iter()
        .map(|(from, to)| format!("{from}={to}"))
        .map(|pair| {
            format!(
                "\"--remap-path-prefix={pair}\"",
                pair = pair.replace('"', "")
            )
        })
        .collect();
    output.push_str(&format!("rustflags = [{}]\n", entries.join(", ")));
    output
}

/// Which subtrees **this** run must not render: the ones the declaration hands to another crate.
/// 本次运行**不得**渲染哪些子树：声明交给另一个 crate 的那些。
///
/// A host hands its claimed subtrees away and renders the rest; a **ghost** is the crate those
/// subtrees were handed to, so it renders exactly them and hands nothing away — clearing the list is
/// what keeps the ghost from skipping the fragment it exists to compile (found by the end-to-end
/// demonstration: the render, not the plan, was where the two modes disagreed).
/// 宿主把它认领的子树交出去、渲染其余；**幽灵**正是那些子树被交给的 crate，因此它渲染的恰好是它们、什么都不
/// 交出去——清空这份清单，才不会让幽灵跳过它存在的理由（由端到端演示发现：两种模式分歧的地方在渲染，不在规划）。
pub(crate) fn cut_out_for(declaration: Option<&ShapeDeclaration>, ghost: bool) -> Vec<String> {
    match (ghost, declaration) {
        (true, _) | (false, None) => Vec::new(),
        (false, Some(declaration)) => declaration.cut_subtrees(),
    }
}

/// Plan every crate a declaration asks for, or refuse by name.
/// 规划声明要求的每个 crate，或者点名拒绝。
///
/// `faces` is the tree's face list as `(host-relative source, module path)` pairs, handed in rather
/// than re-derived: the caller already holds it (the pipeline's `static_faces`, computed **before**
/// the render, which is where the mounts are needed), and a second derivation is a second chance to
/// disagree about which module a file declares.
/// `faces` 是这棵树的面清单，形如 `(宿主相对源码, 模块路径)` 对，由调用方交进来而不是重新推导：调用方手里
/// 已经有了（管线的 `static_faces`，在**渲染之前**算好，而渲染正是需要挂载的地方），而第二次推导就是第二次
/// 对"某文件声明了哪个模块"产生分歧的机会。
pub(crate) fn plan(
    package_root: &Path,
    namespace: &str,
    declaration: &ShapeDeclaration,
    faces: &[(String, String, NodeId)],
) -> Result<Vec<PlannedCrate>, String> {
    let parent = package_root.parent().unwrap_or(package_root);
    let mut planned = Vec::new();
    for (name, subtrees) in &declaration.crates {
        let package = format!("{}-{name}", declaration.package_prefix);
        let directory = parent.join(&package);
        let mut mounts = Vec::new();
        let mut files = Vec::new();
        for subtree in subtrees {
            // The claim names a subtree **rooted at** that node, so the face at the node itself is
            // part of the fragment too: the host hands the whole module away (its render skips the
            // module and everything under it), and a fragment whose root face went to neither crate
            // would be a face nobody compiles.
            // 认领点名的是**以该节点为根**的子树，因此该节点自己的面也属于碎片：宿主把整个模块交出去（它的渲染
            // 跳过该模块及其下的一切），而根面若不归任何一方，就是一个没人编译的面。
            for (source, module_path, _) in faces.iter().filter(|(_, module, _)| {
                *module == *subtree || module.starts_with(&format!("{subtree}::"))
            }) {
                mounts.push(PlannedMount {
                    spelling: spelling_for(package_root, source),
                    module_path: module_path.clone(),
                    source: source.clone(),
                });
                files.push(source.clone());
            }
        }
        mounts.sort_by(|left, right| left.module_path.cmp(&right.module_path));
        files.sort();
        files.dedup();
        // The proper prefixes of the claims that are **faces**: their identities are what a shell can
        // carry so a fragment's `parent:` still resolves (a non-face prefix is a plain directory and
        // has no identity for anyone to name).
        // 认领里**是注册面**的真前缀：它们的身份正是壳可以携带、好让碎片的 `parent:` 仍然解析的东西（不是面的
        // 前缀只是普通目录，没有任何身份可被点名）。
        let mut ancestors: Vec<(String, NodeId)> = Vec::new();
        for subtree in subtrees {
            let mut prefix = String::new();
            for segment in subtree.split("::") {
                if !prefix.is_empty()
                    && let Some((_, _, id)) = faces.iter().find(|(_, module, _)| *module == prefix)
                {
                    ancestors.push((prefix.clone(), *id));
                }
                prefix = if prefix.is_empty() {
                    segment.to_owned()
                } else {
                    format!("{prefix}::{segment}")
                };
            }
        }
        ancestors.sort_by(|left, right| left.0.cmp(&right.0));
        ancestors.dedup();
        // The reachability check comes **after** both, so a refusal can name every file it read and
        // speak about the crate as a whole (a reference between two subtrees of one crate is fine, and
        // so is an ancestor's `NODE_ID`, which the shell carries).
        // 可及性检查排在两者之后，这样拒绝能点名它读过的每份文件、并就整个 crate 说话（同一个 crate 的两棵子树
        // 之间的引用是允许的，祖先的 `NODE_ID` 也允许——壳携带它）。
        let ancestor_modules: Vec<String> =
            ancestors.iter().map(|(module, _)| module.clone()).collect();
        for file in &files {
            check_reachability(package_root, name, file, subtrees, &ancestor_modules)?;
        }
        let mut remap: Vec<(String, String)> = mounts
            .iter()
            .map(|mount| remap_for(&mount.spelling, &mount.source))
            .collect();
        remap.sort();
        remap.dedup();
        let lib_rs = ghost_lib_rs(namespace);
        let build_rs = ghost_build_rs(package_root, namespace, subtrees);
        let cargo_toml = ghost_cargo_toml(package_root, &package)?;
        let config_patch = config_patch(&remap);
        planned.push(PlannedCrate {
            name: name.clone(),
            package,
            namespace: namespace.to_owned(),
            directory,
            subtrees: subtrees.clone(),
            ancestors,
            mounts,
            remap,
            lib_rs,
            build_rs,
            cargo_toml,
            config_patch,
        });
    }
    Ok(planned)
}

/// `(module path, crate name)` for every module in the tree: the crate that compiles it.
/// `(模块路径, crate 名)` 对，覆盖树里每个模块：编译它的那个 crate。
///
/// The facade needs this to rewrite `crate::<module>` into `<owner>::<module>`, because `crate::` inside
/// a facade means the facade itself. One implementation, two callers (the render pass and the facade
/// planner): a claim, its **root face** and its ancestor shells all belong to that ghost's package, and
/// every other module — including the host's own faces — belongs to the host.
/// facade 需要它来把 `crate::<模块>` 改写成 `<属主>::<模块>`，因为 facade 里的 `crate::` 指的是 facade
/// 自己。一处实现、两个调用方（渲染 pass 与 facade 规划器）：一条认领、它的**根面**与它的祖先壳都归那个
/// 幽灵的包，其余每个模块——**包括宿主自己的面**——归宿主。
pub(crate) fn owners(
    host_package: &str,
    faces: &[(String, String, NodeId)],
    planned: &[PlannedCrate],
) -> Vec<(String, String)> {
    let host_crate = host_package.replace('-', "_");
    let mut owner_map: BTreeMap<String, String> = faces
        .iter()
        .map(|(_, module, _)| (module.clone(), host_crate.clone()))
        .collect();
    for planned in planned {
        let owner = planned.package.replace('-', "_");
        for mount in &planned.mounts {
            owner_map.insert(mount.module_path.clone(), owner.clone());
        }
        for (module, _) in &planned.ancestors {
            owner_map.insert(module.clone(), owner.clone());
        }
    }
    owner_map.into_iter().collect()
}

/// The `#[path]` spelling one mounted face file needs: the host's **absolute** path.
/// 一份被挂载的面文件所需的 `#[path]` 拼写：宿主的**绝对**路径。
///
/// It used to be a relative walk from the ghost's own directory, and that cannot work: the generated
/// tree lives in the build script's `OUT_DIR`, which cargo chooses, so `#[path]` resolves relative to a
/// directory the planner never sees. The first end-to-end build measured exactly that
/// (`<out>/panel/frame/../../../app/src/…`: three `../` short of a file that is seven levels away).
/// The host's own generated tree already spells its mounts absolutely, for the same reason. Identity is
/// unaffected because the remap strips the host's `src` prefix back off, so `file!()` still reads as the
/// host-relative source the records name (audit `M7`, P3.2/§M7.15).
/// 它过去是"从幽灵自己目录出发的相对走法"，而那条路不成立：生成树住在构建脚本的 `OUT_DIR`（由 cargo 决定），
/// 因此 `#[path]` 相对一个规划器根本看不见的目录解析。第一次端到端构建量到的正是这个
/// （`<out>/panel/frame/../../../app/src/…`：离那个文件差了七层里的三层）。宿主自己的生成树早就用绝对路径
/// 拼它的挂载，原因相同。身份不受影响，因为 remap 会把宿主的 `src` 前缀摘回去，于是 `file!()` 读起来仍是记录
/// 点名的那个宿主相对源码（审计 `M7`，P3.2/§M7.15）。
fn spelling_for(host_dir: &Path, source: &str) -> String {
    host_dir.join("src").join(source).display().to_string()
}

/// The walk from one directory to one file, both absolute, as a `/`-separated relative path.
/// 从一个目录走到一个文件的相对路径（两者都是绝对路径），用 `/` 分隔。
// Only the authoring CLI face needs this (the facade spells its sibling dependencies relatively);
// the build path no longer walks at all, because a mount is an absolute path (audit `M7`, §M7.37).
// 只有创作面的 CLI 需要它（facade 用相对路径写它的同级依赖）；构建路径不再走任何相对路，因为挂载是绝对路径
// （审计 `M7`，§M7.37）。
#[cfg(feature = "cli")]
pub(crate) fn relative_walk(from_dir: &Path, to_file: &Path) -> Option<String> {
    let from: Vec<_> = from_dir.components().collect();
    let to: Vec<_> = to_file.components().collect();
    let common = from
        .iter()
        .zip(&to)
        .take_while(|(left, right)| left == right)
        .count();
    let mut walk: Vec<String> = Vec::new();
    for _ in common..from.len() {
        walk.push("..".to_owned());
    }
    for component in &to[common..] {
        walk.push(component.as_os_str().to_string_lossy().into_owned());
    }
    (!walk.is_empty()).then(|| walk.join("/"))
}

/// The pair `--remap-path-prefix` receives: the spelling minus the source itself, mapped to nothing.
/// `--remap-path-prefix` 收到的那一对：拼写里去掉源码本身的那部分，映射为空。
fn remap_for(spelling: &str, source: &str) -> (String, String) {
    match spelling.strip_suffix(source) {
        Some(prefix) => (prefix.to_owned(), String::new()),
        None => (spelling.to_owned(), String::new()),
    }
}

/// Refuse a face that reaches **outside** the fragment: the ghost has no `control.rs` above it, so
/// such a path cannot resolve, and a crate that does not compile is worse than a named refusal.
/// 拒绝一个**够到碎片之外**的面：幽灵在它上面没有 `control.rs`，因此那样的路径解析不到，而一个编译不过的
/// crate 比一句点名拒绝更坏。
fn check_reachability(
    package_root: &Path,
    crate_name: &str,
    source: &str,
    subtrees: &[String],
    ancestors: &[String],
) -> Result<(), String> {
    let path = package_root.join("src").join(source);
    let Ok(text) = fs::read_to_string(&path) else {
        return Ok(());
    };
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") {
            continue;
        }
        for (at, _) in line.match_indices("crate::") {
            let rest = &line[at + "crate::".len()..];
            // The trailing `::` of a braced path (`crate::panel::{Gauge}`) is not part of the module
            // the reader has to see, and a message that ends in one reads like a cut-off sentence.
            // 花括号路径（`crate::panel::{Gauge}`）末尾的 `::` 不属于读者需要看见的那个模块，而以此结尾的
            // 消息读起来像被截断的句子。
            let head: String = rest
                .chars()
                .take_while(|character| {
                    character.is_alphanumeric() || *character == '_' || *character == ':'
                })
                .collect::<String>()
                .trim_end_matches(':')
                .to_owned();
            // A `crate::…!` path is a **macro invocation**, not a module lookup: the declaration macro
            // aliases (`crate::root_object!`, `crate::control_object!`) are emitted into every crate's
            // own generated tree, so the ghost has them exactly as the host does. They are also the
            // reason this check cannot be "does the path resolve" — text cannot answer that; what it
            // can answer is "does this reach a *module* outside the fragment".
            // `crate::…!` 路径是**宏调用**，不是模块查找：声明宏别名（`crate::root_object!`、
            // `crate::control_object!`）会发射进每个 crate 自己的生成树，因此幽灵和宿主一样拥有它们。
            // 它们也正是这条检查不能写成"路径能不能解析"的原因——文本答不了那个问题；它能答的是"这里够到了
            // 碎片之外的**模块**吗"。
            if rest[head.len()..].trim_start().starts_with('!') {
                continue;
            }
            if reachable(&head, subtrees, ancestors) {
                continue;
            }
            return Err(format!(
                "add_crates: `{crate_name}` would not compile: {source}:{} reaches `crate::{head}`, \
                 which lives outside the subtree(s) this crate claims ({}), and a ghost crate mounts \
                 only those files — there is no module above them to resolve it against.\n\
                 way forward: partition a node that contains the definition (a crate is a subtree, so \
                 a larger subtree is a legal answer), or make the fragment self-contained by moving \
                 the item into it",
                index + 1,
                subtrees.join(", ")
            ));
        }
    }
    Ok(())
}

/// Whether a `crate::` path stays inside the fragment (or names something every crate has).
/// 一条 `crate::` 路径是否留在碎片之内（或者点名了每个 crate 都有的东西）。
fn reachable(head: &str, subtrees: &[String], ancestors: &[String]) -> bool {
    // The constants and helpers the declaration macros themselves expand to.
    // 声明宏自己展开出来的那些常量与帮手。
    const ALWAYS: &[&str] = &["NICHLINK_NAMESPACE", "root_node_id", "ROOT_NODE_ID"];
    if ALWAYS.contains(&head) {
        return true;
    }
    if subtrees
        .iter()
        .any(|subtree| head == subtree || head.starts_with(&format!("{subtree}::")))
    {
        return true;
    }
    // An ancestor is reachable as a **module** and for its `NODE_ID` alone: the shell carries that
    // one constant so a fragment's `parent:` still resolves, and carries nothing else — a trait
    // defined above the fragment is exactly what this planner refuses on.
    // 祖先作为**模块**、以及仅为它的 `NODE_ID` 可达：壳携带那一个常量，好让碎片的 `parent:` 仍然解析，别的
    // 什么都不带——定义在碎片之上的 trait 正是本规划器据以拒绝的东西。
    ancestors
        .iter()
        .any(|ancestor| head == ancestor || head == format!("{ancestor}::NODE_ID"))
}

#[cfg(test)]
#[path = "crate_plan_tests.rs"]
mod crate_plan_tests;
