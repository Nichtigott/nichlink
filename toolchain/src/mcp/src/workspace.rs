//! What the configured root is, and the merged answer a virtual manifest needs.
//! 配置的根是什么，以及虚拟清单所需要的合并答案。
//!
//! Every identity in this bridge is hashed over `(namespace, source path, declared
//! name)`, and the namespace is the *package's own name* — that is the design, not a
//! coincidence: `env!("CARGO_PKG_NAME")` is what the declaration macros bake in, so
//! "package = namespace" is exactly what a built host compiled. A workspace root is
//! not a package, so it has no namespace of its own, and every tree tool used to
//! refuse there with Cargo's own words. The machine was there; the entrance was not.
//! 本桥里的每个身份都是对 `(命名空间, 源码路径, 声明名)` 取散列，而命名空间就是**包自己的名字**
//! ——这是设计而不是巧合：`env!("CARGO_PKG_NAME")` 正是声明宏烤进去的值，因此"包 = 命名空间"恰恰
//! 就是已构建宿主编译出的那一个。工作区根不是一个包，因此它没有自己的命名空间，而每个树级工具过去
//! 都在那里用 Cargo 自己的话拒绝。机器在，入口不在。
//!
//! This module is that entrance. When the root is a virtual manifest, `cargo metadata`
//! enumerates the members, and each one gets its own identity context — its own package
//! name as namespace — so `registry`, `grafts`, `diff` and `search` answer across the
//! packages instead of refusing. The merged view groups by package, and **every member
//! appears with its status**: `queried` (it has faces, which are reported), `no faces`
//! (a framework crate declares none, and saying so is the answer), or `unresolvable`
//! (with the reason). Neither silence nor a whole-answer error is an answer: a
//! framework member must not be dressed up as an empty tree, and a nested package
//! Cargo cannot resolve must say why rather than vanish.
//! 本模块就是那个入口。根是虚拟清单时，`cargo metadata` 枚举成员，每个成员拿到自己的身份上下文
//! ——以它自己的包名为命名空间——于是 `registry`、`grafts`、`diff` 与 `search` 跨包作答，而不是
//! 拒绝。合并视图按包分组，而且**每个成员都带着它的状态出现**：`queried`（它有面，面会被报出）、
//! `no faces`（框架 crate 不声明任何面，说出来就是答案）、`unresolvable`（带上原因）。沉默与整条
//! 报错都不是答案：框架成员不许被装扮成一棵空树，Cargo 解析不了的嵌套包必须说出原因而不是消失。
//!
//! The override still names one tree. `NICH_LINK_NAMESPACE` wins before Cargo is asked,
//! so with it set a virtual root is read as the package that name denotes — the one case
//! where a caller can give a workspace root an identity by hand.
//! 覆盖仍然只命名一棵树。`NICH_LINK_NAMESPACE` 在询问 Cargo 之前胜出，因此设置它之后，虚拟根会被
//! 读作那个名字所指的包——这是调用方唯一能手工给工作区根一个身份的情形。

use std::path::{Path, PathBuf};

use nichlink_kernel::lexicon;
use serde_json::Value;

use crate::build_time::FaceView;
use crate::mcp::registry::namespace_from;
use crate::mcp::resolve::derived_faces;

/// What the configured root names.
/// 配置的根命名的是什么。
pub(crate) enum Scope {
    /// One package; the string is its identity namespace.
    /// 一个包；字符串就是它的身份命名空间。
    Package(String),
    /// A virtual manifest; these are its members, each with its own context.
    /// 一份虚拟清单；这些是它的成员，各自带着自己的上下文。
    Workspace(Vec<Member>),
    /// A manifest Cargo cannot resolve; the reason it gave.
    /// 一份 Cargo 解析不了的清单；它给出的原因。
    Unresolvable(String),
}

/// One workspace member and the tree its own context derived.
/// 一个工作区成员，以及它自己的上下文推导出的那棵树。
pub(crate) struct Member {
    /// The package name, which is this member's identity namespace.
    /// 包名，也就是这个成员的身份命名空间。
    pub(crate) name: String,
    /// The member's package directory, which is what its tools take as `root`.
    /// 成员的包目录，也就是它的工具接受的 `root`。
    pub(crate) dir: PathBuf,
    /// Whether that member's tree could be derived, and what came out.
    /// 该成员的树能否被推导，以及推导出了什么。
    pub(crate) state: MemberState,
}

/// What a member's own derivation produced.
/// 一个成员自己的推导产出了什么。
pub(crate) enum MemberState {
    /// The derivation ran. An empty `faces` is a status of its own, not a failure.
    /// 推导跑过了。空的 `faces` 是一种独立的状态，而不是失败。
    Derived {
        /// The faces the member declares.
        /// 该成员声明的面。
        faces: Vec<FaceView>,
        /// The `unparsable faces N` line, or the empty string.
        /// `unparsable faces N` 那一行，或空串。
        unparsable: String,
    },
    /// The derivation could not run; the reason it gave.
    /// 推导跑不了；它给出的原因。
    Unresolvable(String),
}

impl MemberState {
    /// The one word this member's row is labelled with.
    /// 这个成员那一行所用的唯一一个词。
    pub(crate) fn status(&self) -> &'static str {
        match self {
            Self::Derived { faces, .. } if faces.is_empty() => "no faces",
            Self::Derived { .. } => "queried",
            Self::Unresolvable(_) => "unresolvable",
        }
    }
}

impl Member {
    /// The one word this member's row is labelled with.
    /// 这个成员那一行所用的唯一一个词。
    pub(crate) fn status(&self) -> &'static str {
        self.state.status()
    }
}

/// Resolve what the root is, building a per-member identity context when it is a
/// workspace.
/// 解析根是什么；当它是工作区时，逐成员建好身份上下文。
///
/// A directory with no manifest at all is still an error rather than a scope: there is
/// nothing there to be wrong about, and the refusal names the way out
/// (`NICH_LINK_NAMESPACE`). A directory that *has* a manifest Cargo cannot resolve is a
/// different answer — it is a real root whose tree cannot be named — so it comes back as
/// [`Scope::Unresolvable`] and the tools report the reason in the body instead of
/// failing the call.
/// 完全没有清单的目录仍然是一个错误而不是一种范围：那里没有任何东西可以被弄错，而拒绝会点名出路
/// （`NICH_LINK_NAMESPACE`）。**有**清单而 Cargo 解析不了的目录是另一个答案——它是一个真实存在、
/// 却无法被命名的根——因此它以 [`Scope::Unresolvable`] 回来，工具在正文里报出原因，而不是让调用失败。
pub(crate) fn scope(root: &Path) -> Result<Scope, String> {
    let configured = std::env::var(lexicon::NAMESPACE_ENV).ok();
    match namespace_from(configured.as_deref(), root) {
        Ok(namespace) => Ok(Scope::Package(namespace)),
        Err(refusal) => {
            let manifest = root.join("Cargo.toml");
            if !manifest.is_file() {
                return Err(refusal);
            }
            let members = metadata_members(&manifest)
                .into_iter()
                .map(|(dir, name)| member(dir, name))
                .collect::<Vec<_>>();
            if members.is_empty() {
                return Ok(Scope::Unresolvable(refusal));
            }
            Ok(Scope::Workspace(members))
        }
    }
}

/// The header every merged tree answer opens with: the root, the member census, and one
/// row per member with its status.
/// 每份合并树级答案开头的表头：根、成员普查，以及逐成员一行带状态。
///
/// The census is over *every* member, and each row is a fact about the tree that was
/// read rather than a result row: it is what keeps a framework crate from reading as an
/// empty host and a broken member from reading as an absent one.
/// 普查覆盖**每一个**成员，而每一行都是关于"被读到的那棵树"的事实而不是结果行：正是它让框架 crate
/// 不会被读成一棵空的宿主树，也让坏掉的成员不会被读成不存在的成员。
pub(crate) fn roster(root: &Path, members: &[Member]) -> String {
    let count = |word: &str| members.iter().filter(|m| m.status() == word).count();
    let mut output = format!(
        "workspace {} (virtual manifest: a workspace root is not a package, so it has no identity \
         namespace of its own; each member below has one)\nmembers {}  queried {}  no faces {}  \
         unresolvable {}\n",
        root.display(),
        members.len(),
        count("queried"),
        count("no faces"),
        count("unresolvable"),
    );
    for member in members {
        output.push_str(&format!(
            "  {:<13} {:<30} {}\n",
            member.status(),
            member.name,
            detail(member),
        ));
    }
    output
}

/// Run one tool's per-package body for every member, under the roster.
/// 在表头之下，为每个成员跑一次某工具的逐包主体。
///
/// The body is the tool's own single-package implementation, handed the member's
/// directory, namespace, and already-derived faces — so a merged answer and a
/// single-package answer cannot disagree, because they are the same code. A member whose
/// derivation failed gets the `tree unavailable (reason)` sentence in place of a body:
/// the tool has no tree there, and it says so where the reader is looking.
/// 主体就是该工具自己的单包实现，接收成员的目录、命名空间与已经推导好的面——因此合并答案与单包答案
/// 不可能不一致，因为它们是同一段代码。推导失败的成员在主体的位置得到 `tree unavailable (原因)`
/// 那一句：那里没有这棵树，而工具就在读者看的地方说出来。
pub(crate) fn merge<F>(
    root: &Path,
    members: &[Member],
    arguments: &Value,
    mut body: F,
) -> Result<String, String>
where
    F: FnMut(&Path, &str, &[FaceView], &str, &Value) -> Result<String, String>,
{
    let mut output = roster(root, members);
    for member in members {
        output.push_str(&format!("\n== {} ({})\n", member.name, member.status()));
        match &member.state {
            MemberState::Derived { faces, unparsable } => {
                output.push_str(&body(
                    &member.dir,
                    &member.name,
                    faces,
                    unparsable,
                    arguments,
                )?);
            }
            MemberState::Unresolvable(reason) => {
                output.push_str(&format!("tree unavailable ({reason})\n"));
            }
        }
    }
    output.push_str(
        "detail: pass `root` as one member's directory (relative to this root) for that package's \
         own answer\n",
    );
    Ok(output)
}

/// The body a tool answers with when the root itself has a manifest Cargo cannot resolve.
/// 根本身有一份 Cargo 解析不了的清单时，工具作答所用的正文。
///
/// This is the nested-package case: the fixture package under a test tree is a package in
/// its own right, but its manifest names path dependencies that no longer exist, so
/// `cargo metadata` fails and Cargo cannot name the namespace. Reporting the reason beats
/// both silence and a bare error, and the sentence names the two ways out.
/// 这就是嵌套包的情形：测试树下的夹具包本身是个包，但它的清单点名了已不存在的路径依赖，因此
/// `cargo metadata` 失败、Cargo 说不出命名空间。报出原因胜过沉默与光秃秃的错误，而这句话点名了两条出路。
pub(crate) fn unresolvable(root: &Path, reason: &str) -> String {
    format!(
        "unresolvable {}\n{reason}\nthis root has a Cargo.toml, and a tree answer needs the identity \
         namespace Cargo reports — every NodeId is a hash over it — so there is no tree here to \
         report. Set {} to name the tree explicitly, or point `root` at a resolvable package.\n",
        root.display(),
        lexicon::NAMESPACE_ENV,
    )
}

/// Why a member can hold no faces at all, spelled once for every census row and section.
/// 一个成员为什么可以一个面都没有；普查行与小节只拼一次。
pub(crate) const NO_FACES_REASON: &str = "no registration face under src/ — for a framework crate \
                                         that empty list is the answer rather than a missing one";

/// One member's row detail: its face count, why it has none, or why it could not be read.
/// 一个成员那一行的细节：它的面数、它为什么一个都没有，或它为什么读不了。
///
/// A derivation that dropped a registration file says so here too: `queried, 3 faces` over a
/// package whose fourth file nothing could parse would read as a whole tree, and this row is
/// where a reader looks first.
/// 丢弃过注册面文件的推导在这里也要说出来：一个还有第四个文件谁也解析不了的包，若只写
/// `queried, 3 faces`，那就会被读成一棵完整的树，而这一行正是读者先看的地方。
fn detail(member: &Member) -> String {
    match &member.state {
        MemberState::Derived { faces, unparsable } => {
            let head = if faces.is_empty() {
                NO_FACES_REASON.to_owned()
            } else {
                format!("{} faces", faces.len())
            };
            if unparsable.is_empty() {
                head
            } else {
                format!("{head}; {}", unparsable.trim())
            }
        }
        MemberState::Unresolvable(reason) => format!("tree unavailable ({})", one_line(reason)),
    }
}

/// The members `cargo metadata` reports for a manifest, in Cargo's own order.
/// `cargo metadata` 为一份清单报告的成员，按 Cargo 自己的顺序。
///
/// A failure comes back as an empty list rather than an error, because the caller already
/// holds the reason: `package_name` ran the same command and its message names the
/// manifest and Cargo's stderr, which is the sentence worth reporting.
/// 失败以空清单而不是错误回来，因为调用方已经握有原因：`package_name` 跑过同一条命令，它的消息点名
/// 清单与 Cargo 的 stderr，那才是值得报出的句子。
fn metadata_members(manifest: &Path) -> Vec<(PathBuf, String)> {
    let Ok(output) = std::process::Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .arg("--manifest-path")
        .arg(manifest)
        .output()
    else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    let Ok(metadata) = serde_json::from_slice::<Value>(&output.stdout) else {
        return Vec::new();
    };
    let (Some(packages), Some(members)) = (
        metadata["packages"].as_array(),
        metadata["workspace_members"].as_array(),
    ) else {
        return Vec::new();
    };
    members
        .iter()
        .filter_map(|id| {
            let id = id.as_str()?;
            let package = packages
                .iter()
                .find(|package| package["id"].as_str() == Some(id))?;
            let name = package["name"].as_str()?;
            let dir = Path::new(package["manifest_path"].as_str()?).parent()?;
            Some((dir.to_path_buf(), name.to_owned()))
        })
        .collect()
}

/// Build one member's identity context and derive its tree.
/// 建好一个成员的身份上下文并推导它的树。
fn member(dir: PathBuf, name: String) -> Member {
    let state = match derived_faces(&dir, &name) {
        Ok((faces, unparsable)) => MemberState::Derived { faces, unparsable },
        Err(reason) => MemberState::Unresolvable(reason),
    };
    Member { name, dir, state }
}

/// Fold a possibly multiline reason into the one line a roster row can carry.
/// 把可能多行的原因折成表头一行装得下的那一行。
fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
#[path = "workspace_tests.rs"]
mod workspace_tests;
