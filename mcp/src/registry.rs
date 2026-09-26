//! The registration tree this package declares, as the build sees it.
//! 本包声明的注册树，按构建所见。
//!
//! The bridge used to answer registry questions by re-deriving them: an agent
//! grepped for macro names and reconstructed the tree itself, which is the drift
//! the registry exists to remove. The rows here come from the build's own
//! derivation (`nichlink_build_method::face_views`), so a path or an identity
//! this tool reports is the one the host actually compiled.
//! 本桥过去靠重新推导来回答注册问题：代理 grep 宏名，自己重建那棵树——这正是注册树要消除
//! 的漂移。这里的行来自构建自己的推导（`nichlink_build_method::face_views`），因此本工具
//! 报告的路径或身份就是宿主真正编译出的那一个。

use std::path::Path;

use nichlink::lexicon;
use nichlink_build_method::{FaceView, face_views, package_name};

/// Report every registration face declared under the package root `root`.
/// 报告 `root` 这个包根下声明的每个注册面。
///
/// `root` is a package root, because that is what the derivation needs: the
/// faces' identities are hashed over paths relative to the package's `src/`, and
/// their namespace is the package's own name.
/// `root` 是包根，因为推导需要它：面的身份是对相对该包 `src/` 的路径取散列，而它们的命名空间
/// 就是这个包自己的名字。
pub(crate) fn registry(root: &Path) -> Result<String, String> {
    let namespace = namespace(root)?;
    let faces = face_views(root, &namespace)?;
    Ok(render(&namespace, &faces))
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
/// `NICH_LINK_NAMESPACE` wins verbatim — the override every surface reads, so
/// setting it moves the build and every reader together — and otherwise the
/// package name Cargo reports is the namespace, because that is exactly what the
/// declaration macros bake in as `env!("CARGO_PKG_NAME")`.
/// `NICH_LINK_NAMESPACE` 一旦设置就原样胜出——每个执行面都读这个覆盖，设置它会把构建与所有
/// 读取方一起挪动——否则 Cargo 报告的包名就是命名空间，因为声明宏烤进去的
/// `env!("CARGO_PKG_NAME")` 正是它。
pub(crate) fn namespace(root: &Path) -> Result<String, String> {
    namespace_from(std::env::var(lexicon::NAMESPACE_ENV).ok().as_deref(), root)
}

/// The namespace decision, taking the configured value as a parameter so the
/// override and the fallback can be tested without the process environment.
/// 命名空间的判断；配置值作为参数传入，因此覆盖与回落都能不依赖进程环境地测试。
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
fn namespace_from(configured: Option<&str>, root: &Path) -> Result<String, String> {
    if let Some(configured) = configured {
        return Ok(configured.to_owned());
    }
    package_name(&root.join("Cargo.toml")).map_err(|error| {
        format!(
            "cannot learn the identity namespace of {}: {error}; \
             set {} to name it explicitly",
            root.display(),
            lexicon::NAMESPACE_ENV
        )
    })
}

/// One line per face, plus the namespace the identities live in.
/// 每个面一行，外加这些身份所属的命名空间。
///
/// The namespace heads the report because every id below it is meaningless
/// without it: a reader comparing these rows against a built tree has to know
/// which identity domain they are in.
/// 命名空间写在报告开头，因为下面的每个 id 离开它都没有意义：把这些行与已构建的树对照的读取方
/// 必须知道它们处在哪个身份域。
fn render(namespace: &str, faces: &[FaceView]) -> String {
    let mut output = format!("namespace {namespace}\nfaces {}\n", faces.len());
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

#[cfg(test)]
#[path = "registry_tests.rs"]
mod registry_tests;
