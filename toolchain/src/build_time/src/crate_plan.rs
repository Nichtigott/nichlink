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

use std::fs;
use std::path::{Path, PathBuf};

use super::face_view::PruningRow;
use super::shape_decl::ShapeDeclaration;
use super::static_plan::source_module_path;

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
    /// One mount per face file below those subtrees.
    /// 那些子树之下的每份面文件一项挂载。
    pub(crate) mounts: Vec<PlannedMount>,
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
pub(crate) fn plan(
    package_root: &Path,
    namespace: &str,
    declaration: &ShapeDeclaration,
    rows: &[PruningRow],
) -> Result<Vec<PlannedCrate>, String> {
    let host = package_root
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            format!(
                "add_crates: the package root {} has no directory name to build a sibling crate from",
                package_root.display()
            )
        })?
        .to_owned();
    let parent = package_root.parent().unwrap_or(package_root);
    let mut planned = Vec::new();
    for (name, subtrees) in &declaration.crates {
        let package = format!("{}-{name}", declaration.package_prefix);
        let directory = parent.join(&package);
        let mut mounts = Vec::new();
        let mut files = Vec::new();
        for subtree in subtrees {
            for row in rows
                .iter()
                .filter(|row| source_module_path(&row.source).starts_with(&format!("{subtree}::")))
            {
                let module_path = source_module_path(&row.source);
                mounts.push(PlannedMount {
                    spelling: spelling_for(&host, &row.source, &module_path),
                    module_path,
                    source: row.source.clone(),
                });
                files.push(row.source.clone());
            }
        }
        mounts.sort_by(|left, right| left.module_path.cmp(&right.module_path));
        files.sort();
        files.dedup();
        // The reachability check comes **after** the mounts, so a refusal can name every file it read
        // and speak about the crate as a whole (a reference between two subtrees of one crate is fine).
        // 可及性检查放在挂载之后，这样拒绝能点名它读过的每份文件，并就整个 crate 说话（同一个 crate 的两棵
        // 子树之间的引用是允许的）。
        for file in &files {
            check_reachability(package_root, name, file, subtrees)?;
        }
        let mut remap: Vec<(String, String)> = mounts
            .iter()
            .map(|mount| remap_for(&host, &mount.module_path))
            .collect();
        remap.sort();
        remap.dedup();
        planned.push(PlannedCrate {
            name: name.clone(),
            package,
            namespace: namespace.to_owned(),
            directory,
            subtrees: subtrees.clone(),
            mounts,
            remap,
        });
    }
    Ok(planned)
}

/// The `#[path]` spelling one mounted face file needs, and the prefix to remap away.
/// 一份被挂载的面文件所需的 `#[path]` 拼写，以及要映射掉的前缀。
///
/// `#[path]` resolves relative to the **directory the inline module stands in**, and an inline module
/// adds its own name to that directory — so a mount for `control::object::button` sits in
/// `<ghost>/src/control/object/`, and the spelling walks back out of `src`, out of the ghost package,
/// and into the host's `src`. Written that way the spelling is exactly `<prefix><host-relative
/// source>`, which is what makes the remap pair *derivable* instead of guessed: remapping `<prefix>`
/// to nothing leaves `file!()` reading as `control/object/button/button.rs` — the very spelling the
/// host's records name, and therefore the same identity (audit `M7`, P3.2/§M7.15).
/// `#[path]` 相对**内联模块所在目录**解析，而内联模块会把自己的名字加进那个目录——因此
/// `control::object::button` 的挂载位于 `<ghost>/src/control/object/`，而拼写要走出 `src`、走出幽灵包、走进
/// 宿主的 `src`。这样写出来，拼写恰好是 `<前缀><宿主相对源码>`，这正是让映射对**可推导**而不是靠猜的原因：
/// 把 `<前缀>` 映射为空，`file!()` 读起来就是 `control/object/button/button.rs`——宿主记录点名的那个拼写，
/// 因此也是同一个身份（审计 `M7`，P3.2/§M7.15）。
fn spelling_for(host: &str, source: &str, module_path: &str) -> String {
    let prefix = prefix_for(host, module_path);
    format!("{prefix}{source}")
}

/// The walk-out prefix one mount needs, given how deep its inline module sits.
/// 一次挂载所需的"走出去"前缀，取决于它的内联模块有多深。
///
/// `depth` `../` leave the inline directories and `src`, one more leaves the ghost package, and the
/// host package name walks back in.
/// `depth` 个 `../` 走出内联目录与 `src`，再多一个走出幽灵包，然后由宿主包名走回去。
fn prefix_for(host: &str, module_path: &str) -> String {
    let depth = module_path.split("::").count();
    format!("{}{host}/src/", "../".repeat(depth + 1))
}

/// The pair `--remap-path-prefix` receives: the walk-out prefix, mapped to nothing.
/// `--remap-path-prefix` 收到的那一对：走出去的前缀，映射为空。
fn remap_for(host: &str, module_path: &str) -> (String, String) {
    (prefix_for(host, module_path), String::new())
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
            if reachable(&head, subtrees) {
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
fn reachable(head: &str, subtrees: &[String]) -> bool {
    // The constants and helpers the declaration macros themselves expand to.
    // 声明宏自己展开出来的那些常量与帮手。
    const ALWAYS: &[&str] = &["NICHLINK_NAMESPACE", "root_node_id", "ROOT_NODE_ID"];
    if ALWAYS.contains(&head) {
        return true;
    }
    subtrees
        .iter()
        .any(|subtree| head == subtree || head.starts_with(&format!("{subtree}::")))
}

#[cfg(test)]
#[path = "crate_plan_tests.rs"]
mod crate_plan_tests;
