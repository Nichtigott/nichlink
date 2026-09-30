//! The registration tree this package declares, as the build sees it.
//! 本包声明的注册树，按构建所见。
//!
//! The bridge used to answer registry questions by re-deriving them: an agent
//! grepped for macro names and reconstructed the tree itself, which is the drift
//! the registry exists to remove. The rows here now come from the build's **own
//! published records** (`<package>/target/nichlink/out`, read through
//! `build_method`'s readers — `published.rs`), so a workspace-rooted answer costs
//! the records instead of every member's source walk; the derivation
//! (`crate::build_time::face_views`) is the fallback for a member that published
//! nothing, and every answer says which of the two it used.
//! 本桥过去靠重新推导来回答注册问题：代理 grep 宏名，自己重建那棵树——这正是注册树要消除
//! 的漂移。这里的行现在来自构建**自己已发布的记录**（`<package>/target/nichlink/out`，经
//! `build_method` 的读取器读取——`published.rs`），因此工作区根上的一次答案花的是记录而不是
//! 每个成员的源码遍历；推导（`crate::build_time::face_views`）是给什么都没发布的成员的回落，
//! 而每一份答案都会说出自己用的是哪一种。
//!
//! What the published rows carry is `node`, `source` and the symbol release
//! pruning tracks, plus the scope verdict — the record's own columns. `path`,
//! `kind`, `registry_name` and `parent` are *derived* facts and are not in it, so
//! an answer built from the record says so and names `nichlink.explain` (the one
//! report that derives a per-face projection) as where to read them.
//! 已发布的行携带的是 `node`、`source` 与发布剪枝跟踪的符号，外加作用域结论——也就是记录自己的
//! 列。`path`、`kind`、`registry_name` 与 `parent` 是**推导**出来的事实、不在其中，因此由记录
//! 构成的答案会说出来，并点名 `nichlink.explain`（唯一推导逐面投影的报告）为读取它们的地方。

use std::path::Path;

use crate::build_time::FaceView;
use nichlink_kernel::lexicon;
use serde_json::Value;

use crate::mcp::published::{FACES_UNKNOWN, PublishedTree};
use crate::mcp::workspace::{self, Member, Scope, Tree};

/// Report every registration face declared under the package root `root`.
/// 报告 `root` 这个包根下声明的每个注册面。
///
/// `root` is a package root, because that is what an identity needs: the faces'
/// identities are hashed over paths relative to the package's `src/`, and their
/// namespace is the package's own name. A **virtual manifest** names no package,
/// so it is answered as the workspace it is: one section per member, each under its own
/// name, with every member's status in the census above them.
/// `root` 是包根，因为身份需要它：面的身份是对相对该包 `src/` 的路径取散列，而它们的命名空间
/// 就是这个包自己的名字。**虚拟清单**不命名任何包，因此它按它实际的样子——工作区——作答：逐成员
/// 一节，各自在自己的名字之下，而它们上方是每个成员的状态普查。
pub(crate) fn registry(root: &Path) -> Result<String, String> {
    match workspace::scope(root)? {
        Scope::Package(namespace) => {
            let member = Member::package(root, namespace);
            registry_body(&member, &serde_json::json!({}))
        }
        Scope::Workspace(members) => {
            let arguments = serde_json::json!({});
            workspace::merge(root, &members, &arguments, registry_body)
        }
        Scope::Unresolvable(reason) => Ok(workspace::unresolvable(root, &reason)),
    }
}

/// One package's registry report, as the merged view and the single-package view both
/// call it.
/// 一个包的注册树报告；合并视图与单包视图都调用它。
///
/// The evidence is chosen once, here, and the two renderings below share nothing
/// but the namespace line: they answer from different facts and say so. A body
/// that quietly derived over a published member would make the perf note on this
/// page false, so the branch is a `match` a reader can see.
/// 证据在这里被选一次，下面两种渲染只共享命名空间那一行：它们用不同的事实作答，并且说出来。
/// 一个在发布过的成员之上悄悄推导的主体会让本页的性能注记变成假话，因此这个分支是一个读者看得见
/// 的 `match`。
pub(crate) fn registry_body(member: &Member, _arguments: &Value) -> Result<String, String> {
    match member.tree()? {
        Tree::Published(tree) => Ok(render_published(&member.name, tree)),
        Tree::Derived { faces, unparsable } => Ok(render_registry(
            &member.name,
            faces,
            unparsable,
            &member.evidence_line(""),
        )),
    }
}

/// The identity namespace this package's faces were stamped with.
/// 本包的注册面被盖下的身份命名空间。
///
/// Shared with the write path: an edit must author under the same namespace the
/// query reports, or the face it writes lands in an identity domain the host never
/// compiled.
/// 与写入路径共用：编辑必须在查询所报告的同一个命名空间之下创作，否则它写下的面会落在宿主从未
/// 编译过的身份域里。
///
/// `NICH_LINK_NAMESPACE` wins verbatim when it is set, and otherwise the package name Cargo reports
/// is the namespace, because that is exactly what the declaration macros bake in as
/// `env!("CARGO_PKG_NAME")`.
/// `NICH_LINK_NAMESPACE` 一旦设置就原样胜出，否则 Cargo 报告的包名就是命名空间，因为声明宏烤进去的
/// `env!("CARGO_PKG_NAME")` 正是它。
///
/// What this is **not**: a rule every surface reads. Studio reads the same override, the CLI reads
/// none of it (it always asks Cargo for the package name), and no build-side code reads it either —
/// the namespace is baked in at compile time. So with the variable set this query reports a
/// namespace, and `NodeId`s, that `nichlink explain --json` and the built host do not use. The
/// variable's documented purpose is a *reader's* override for trace artifacts; whether it should
/// keep applying to host identity is the maintainer's decision, and this note exists so the
/// divergence is a known one rather than a claim of agreement (audit `S12`).
/// 这**不是**什么：不是每个执行面都读的规则。Studio 读同一个覆盖，CLI 完全不读（它始终问 Cargo 要
/// 包名），构建侧也不读——命名空间在编译期就烤好了。因此设置该变量后，这条查询报告的命名空间与
/// `NodeId`，正是 `nichlink explain --json` 与已构建宿主**不用的**那一套。该变量文档化的用途是 trace
/// artifact 的**读取者覆盖**；它是否应继续作用于宿主身份由维护者决定，而这段注记的存在是为了让这处分歧
/// 成为已知事项，而不是一句"彼此一致"的声称（审计 `S12`）。
pub(crate) fn namespace(root: &Path) -> Result<String, String> {
    namespace_from(std::env::var(lexicon::NAMESPACE_ENV).ok().as_deref(), root)
}

/// The namespace decision, taking the configured value as a parameter so the
/// override and the fallback can be tested without the process environment.
/// 命名空间的判断；配置值作为参数传入，因此覆盖与回落都能不依赖进程环境地测试。
///
/// It is `pub(crate)` because the workspace entrance asks the same question first: a
/// configured override names one tree, so a virtual root under it is that package rather
/// than a workspace (`workspace::scope`).
/// 它是 `pub(crate)`，因为工作区入口先问同一个问题：配置的覆盖命名一棵树，因此它之下的虚拟根是那个
/// 包而不是一个工作区（`workspace::scope`）。
///
/// The documented default is deliberately **not** a fallback here, and the
/// asymmetry with authoring is the point: authoring *creates* a tree, so
/// `nichlink.default` is a real answer for a project nobody has built yet, while
/// this tool *reports* a tree that already exists. Answering under
/// `nichlink.default` would publish identities the host never compiled — every
/// `NodeId` is a hash over the namespace — and an agent would carry them into a
/// graft record or a trace lookup that cannot resolve. A refusal it can act on
/// (`set NICH_LINK_NAMESPACE`) beats an identity that is wrong everywhere.
/// 这里有意**不**回落到文档化的默认值，而与创作侧的不对称正是关键：创作是在**创建**一棵树，
/// 对一个还没人构建过的项目，`nichlink.default` 是真实答案；而本工具报告的是**已经存在**的
/// 树。在 `nichlink.default` 之下作答会发布宿主从未编译过的身份——每个 `NodeId` 都是对命名
/// 空间的散列——而代理会带着它们去做无法解析的 graft 记录或 trace 查找。一个它能据以行动的
/// 拒绝（`set NICH_LINK_NAMESPACE`）胜过到处都错的身份。
pub(crate) fn namespace_from(configured: Option<&str>, root: &Path) -> Result<String, String> {
    crate::build_time::identity_namespace(configured, &root.join("Cargo.toml")).map_err(|error| {
        format!(
            "cannot learn the identity namespace of {}: {error}; \
             set {} to name it explicitly",
            root.display(),
            lexicon::NAMESPACE_ENV
        )
    })
}

/// One line per derived face, plus the namespace the identities live in.
/// 每个推导出的面一行，外加这些身份所属的命名空间。
///
/// The namespace heads the report because every id below it is meaningless
/// without it: a reader comparing these rows against a built tree has to know
/// which identity domain they are in.
/// 命名空间写在报告开头，因为下面的每个 id 离开它都没有意义：把这些行与已构建的树对照的读取方
/// 必须知道它们处在哪个身份域。
fn render_registry(
    namespace: &str,
    faces: &[FaceView],
    unparsable: &str,
    evidence: &str,
) -> String {
    let mut output = format!(
        "namespace {namespace}\n{evidence}{unparsable}faces {}\n",
        faces.len()
    );
    for face in faces {
        // An unresolved parent is named rather than hidden: the face is real,
        // and the fact that its parent is not is the answer to "why is this node
        // not in my tree".
        // 未解析的父级被点名而不是藏起来：这个面是真的，而"它的父级不是"正是"为什么这个节点不在
        // 我的树里"的答案。
        let unresolved = if face.parent_resolved {
            ""
        } else {
            "  parent-unresolved"
        };
        output.push_str(&format!(
            "{:<40} {:<14} {:<38} {}{}\n",
            face.path, face.kind, face.source, face.id, unresolved
        ));
    }
    if faces.is_empty() {
        output.push_str("no registration face is declared under the package's src/\n");
    }
    output
}

/// The record's own rows: an identity, a source, a symbol, and the scope's verdict.
/// 记录自己的行：一个身份、一条源码、一个符号，以及作用域的结论。
///
/// The face list is `pruning_manifest.tsv`'s, which the build writes for **every**
/// face it found — not `source_scope.tsv`'s, which lists only the *roots* a
/// narrowed scope selected (two rows for a three-face package, measured on
/// `examples/control-button`). Reading the scope's list as the tree would
/// under-report it, and the last line says where the derived projection lives so
/// nobody has to guess why a `path` is missing.
/// 面清单是 `pruning_manifest.tsv` 的，构建为它找到的**每个**面都写这一份——而不是
/// `source_scope.tsv` 的，后者只列收窄作用域选中的**根**（在 `examples/control-button` 上实测：
/// 三个面的包只有两行）。把作用域那份清单读成树会少报，而最后一行说出推导投影住在哪里，因此没人
/// 需要猜为什么没有 `path`。
fn render_published(namespace: &str, tree: &PublishedTree) -> String {
    let mut output = format!(
        "namespace {namespace}\n{}{}\n",
        tree.evidence_line(),
        tree.freshness()
    );
    output.push_str(&format!(
        "scope mode={} all={} reason={} selected_ids={} selected_sources={} selected_modules={}\n",
        tree.scope.mode,
        tree.scope.all,
        tree.scope.reason.as_deref().unwrap_or("-"),
        tree.scope.selected_ids.len(),
        tree.scope.selected_sources.len(),
        tree.scope.selected_modules.len(),
    ));
    let rows = tree.faces();
    if tree.faces_unknown() {
        output.push_str(FACES_UNKNOWN);
        output.push('\n');
    } else {
        output.push_str(&format!("faces {}\n", rows.len()));
    }
    for row in rows {
        let verdict = if tree.selected(row) {
            "selected"
        } else {
            "not-selected"
        };
        output.push_str(&format!(
            "  {:<13} {:<38} {:<44} {}\n",
            verdict, row.id, row.source, row.symbol
        ));
    }
    if rows.is_empty() && !tree.faces_unknown() {
        output.push_str("no registration face is recorded for this package — the sources may still declare `external_object!` faces, which this package's generated tree deliberately does not contain; `nichlink.search` derives and names them\n");
    }
    output.push_str(
        "note: these are the build's published rows (node, source, tracked symbol, scope verdict). \
         `path`, `kind`, `registry_name` and `parent` are derived facts and are not in the record — \
         `nichlink.explain` reports that derived projection.\n",
    );
    output
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod registry_tests;
