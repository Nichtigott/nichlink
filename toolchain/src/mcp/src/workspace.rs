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
//! enumerates the members, and each one is read **from its own published records
//! first** (`<member>/target/nichlink/out`, through `build_method`'s readers — see
//! `published.rs`). Deriving a member's faces is the *fallback*, taken only when it
//! published nothing, because deriving every member is what made a workspace answer
//! cost the sum of every member's source walk. The merged view groups by package,
//! and **every member appears with its status**: `published` (its own records, with
//! freshness), `not built` (no records, so the tree below was derived now and says
//! so), `no faces` (the record — or the derived tree — declares none, and saying so
//! is the answer), or `unresolvable` (with the reason). Neither silence nor a
//! whole-answer error is an answer: a framework member must not be dressed up as an
//! empty tree, and a nested package Cargo cannot resolve must say why rather than
//! vanish.
//! 本模块就是那个入口。根是虚拟清单时，`cargo metadata` 枚举成员，而每个成员**先按它自己已发布的
//! 记录**读取（`<member>/target/nichlink/out`，经 `build_method` 的读取器——见 `published.rs`）。
//! 推导一个成员的面是**回落**路径，只在它什么都没发布时才走，因为逐成员推导正是让一份工作区答案
//! 等于每个成员源码遍历之和的原因。合并视图按包分组，而且**每个成员都带着它的状态出现**：
//! `published`（它自己的记录，带新鲜度）、`not built`（没有记录，因此下面的树是现推的，并且会说出来）、
//! `no faces`（记录——或推导出的树——不声明任何面，说出来就是答案）、或 `unresolvable`（带上原因）。
//! 沉默与整条报错都不是答案：框架成员不许被装扮成一棵空树，Cargo 解析不了的嵌套包必须说出原因而不是消失。
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
use crate::mcp::build_evidence::out_dir;
use crate::mcp::published::{self, FACES_UNKNOWN, Publication, PublishedTree};
use crate::mcp::registry::namespace_from;
use crate::mcp::resolve::derived_faces;
use nichlink_kernel::identity::NodeId;

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

/// One member of a workspace, and what its own records or sources say.
/// 工作区的一个成员，以及它自己的记录或源码说了什么。
pub(crate) struct Member {
    /// The package name, which is this member's identity namespace.
    /// 包名，也就是这个成员的身份命名空间。
    pub(crate) name: String,
    /// The member's package directory, which is what its tools take as `root`.
    /// 成员的包目录，也就是它的工具接受的 `root`。
    pub(crate) dir: PathBuf,
    /// Whether that member published records, had to be derived, or could not be read.
    /// 该成员发布了记录、不得不现推，还是读不了。
    pub(crate) state: MemberState,
}

/// What one member's own evidence is.
/// 一个成员自己的证据是什么。
pub(crate) enum MemberState {
    /// The member's own `target/nichlink/out`, read through `build_method`'s readers.
    /// 该成员自己的 `target/nichlink/out`，经 `build_method` 的读取器读取。
    Published(PublishedTree),
    /// No readable published records, plus the reason — and the tree derived now, so
    /// the answer carries the same face half a single-package answer would.
    /// 没有可读的已发布记录，加上原因——以及现推的那棵树，因此答案带着与单包答案相同的面那一半。
    Unpublished {
        /// Why nothing was published, in the reader's own words.
        /// 为什么什么都没发布，用读取方自己的话。
        reason: String,
        /// The faces the derivation found.
        /// 推导找到的面。
        faces: Vec<FaceView>,
        /// The `unparsable faces N` line, or the empty string.
        /// `unparsable faces N` 那一行，或空串。
        unparsable: String,
    },
    /// The derivation could not run; the reason it gave.
    /// 推导跑不了；它给出的原因。
    Unresolvable(String),
}

/// The tree half of one member's answer, as the member holds it.
/// 一个成员答案里的树那一半，按该成员持有的样子。
///
/// Borrowed rather than owned, because the two halves live in different places:
/// a published tree was read into the member, and a derived one either was derived
/// at scope time or has to be derived per answer. A body that names faces by
/// logical path takes [`Member::derived_tree`] instead, and says so in its reply.
/// 借用而不是拥有，因为两半住在不同的地方：发布树是读进成员里的，而推导出的树要么在解析作用域时
/// 就已推导，要么得逐答案推导。按逻辑路径点名面的主体改用 [`Member::derived_tree`]，并在回复里
/// 说出来。
pub(crate) enum Tree<'a> {
    /// The member's own published records.
    /// 该成员自己已发布的记录。
    Published(&'a PublishedTree),
    /// The sources, derived: at scope time when nothing was published, now otherwise.
    /// 源码推导出的树：什么都没发布时在解析作用域时推导，否则此刻推导。
    Derived {
        /// The faces the derivation found.
        /// 推导找到的面。
        faces: &'a [FaceView],
        /// The `unparsable faces N` line, or the empty string.
        /// `unparsable faces N` 那一行，或空串。
        unparsable: &'a str,
    },
}

impl MemberState {
    /// The one word this member's row is labelled with.
    /// 这个成员那一行所用的唯一一个词。
    ///
    /// Two axes meet here, and the words keep them apart. `published` and
    /// `not built` say where the tree came from; `no faces` and `unresolvable` say
    /// what it is. `no faces` wins over the source axis when it applies, because
    /// "this package declares nothing" is the answer a reader needs and the
    /// evidence's provenance is the footnote — and a member that was never built
    /// still must not be dressed up as an empty tree, which is why the empty
    /// derived tree of an unbuilt member is `no faces` with the reason beside it
    /// rather than `not built` alone.
    /// 两个轴在此相遇，而词汇把它们分得开。`published` 与 `not built` 说树从哪里来；`no faces`
    /// 与 `unresolvable` 说它是什么。适用时 `no faces` 胜过来源轴，因为"这个包什么都不声明"才是
    /// 读取方需要的答案，证据的出处只是脚注——而一个从未构建过的成员仍然不许被装扮成一棵空树，这
    /// 正是为什么没构建过的成员的空推导树是 `no faces`、旁边带着原因，而不是光一个 `not built`。
    pub(crate) fn status(&self) -> &'static str {
        match self {
            Self::Published(tree) if tree.records_no_faces() => "no faces",
            Self::Published(..) => "published",
            Self::Unpublished { faces, .. } if faces.is_empty() => "no faces",
            Self::Unpublished { .. } => "not built",
            Self::Unresolvable(_) => "unresolvable",
        }
    }
}

impl Member {
    /// One package read as its own member, so a single-package answer and a merged
    /// one go through the same body with the same evidence.
    /// 把一个包当作它自己的成员来读，因此单包答案与合并答案经同一个主体、带着同一份证据。
    pub(crate) fn package(dir: &Path, name: String) -> Self {
        member(dir.to_path_buf(), name)
    }

    /// The one word this member's row is labelled with.
    /// 这个成员那一行所用的唯一一个词。
    pub(crate) fn status(&self) -> &'static str {
        self.state.status()
    }

    /// The tree half as this member holds it: its records, or the tree derived at
    /// scope time. Never reads a source file.
    /// 该成员持有的树那一半：它自己的记录，或解析作用域时推导出的树。绝不读源码文件。
    ///
    /// `Err` is the derivation that could not run, which is the answer a caller
    /// must report rather than swallow.
    /// `Err` 是跑不了的推导，那是调用方必须报出而不是吞掉的答案。
    pub(crate) fn tree(&self) -> Result<Tree<'_>, String> {
        match &self.state {
            MemberState::Published(tree) => Ok(Tree::Published(tree)),
            MemberState::Unpublished {
                faces, unparsable, ..
            } => Ok(Tree::Derived { faces, unparsable }),
            MemberState::Unresolvable(reason) => Err(reason.clone()),
        }
    }

    /// The faces this member names by logical path, derived now when the member
    /// published records — because the record carries no `path` and no `kind`.
    /// 这个成员按逻辑路径点名的面；成员发布过记录时此刻推导——因为记录不携带 `path` 与 `kind`。
    ///
    /// This is the fallback the published path exists to avoid, and it is taken
    /// only by answers whose question the record cannot answer. Every caller
    /// states that it took it (`evidence_line`), so a reply never leaves a reader
    /// guessing which of the two trees it is looking at.
    /// 这就是发布路径要避免的那次回落，只有记录答不了其问题的答案才会走。每个调用方都会说出自己走了
    /// 它（`evidence_line`），因此回复从不让读取方猜自己看的是两棵树里的哪一棵。
    pub(crate) fn derived_tree(&self) -> Result<(Vec<FaceView>, String), String> {
        match &self.state {
            MemberState::Unpublished {
                faces, unparsable, ..
            } => Ok((faces.clone(), unparsable.clone())),
            MemberState::Published(..) => derived_faces(&self.dir, &self.name),
            MemberState::Unresolvable(reason) => Err(reason.clone()),
        }
    }

    /// The member's published records, when it has readable ones.
    /// 成员的已发布记录（当它有可读的记录时）。
    pub(crate) fn published(&self) -> Option<&PublishedTree> {
        match &self.state {
            MemberState::Published(tree) => Some(tree),
            _ => None,
        }
    }

    /// Whether the published record alone can say this member owns `id`.
    /// 仅凭已发布的记录能否说明这个成员拥有 `id`。
    ///
    /// A positive answer is final — the build wrote that identity into this
    /// package's manifest, so no other package holds it. A negative one is not:
    /// a face added since the build is in the sources and not in the record, so a
    /// caller that needs "no" still has to derive.
    /// 肯定答案是终局——构建把那个身份写进了这个包的清单，因此别的包不会持有它。否定答案不是：
    /// 构建之后新增的面在源码里而不在记录里，因此需要"没有"的调用方仍然必须推导。
    pub(crate) fn publishes(&self, id: NodeId) -> bool {
        self.published().is_some_and(|tree| tree.has_identity(id))
    }

    /// The line naming which of the two trees this answer used.
    /// 说出这份答案用的是两棵树里哪一棵的那一行。
    ///
    /// A published answer names the directory and the fingerprint. A derived one
    /// says why it had to derive, in the record reader's own words — and a body
    /// that derived *over* a published member passes `because`, which is the
    /// question the record could not answer, so the line never leaves a reader
    /// guessing which tree it is looking at.
    /// 发布答案点名目录与指纹。推导出的答案用记录读取方自己的话说出为什么不得不推导——而在发布过的
    /// 成员之上**仍然**推导的主体传入 `because`，也就是记录答不了的那个问题，因此这一行从不让读取方
    /// 猜自己看的是哪棵树。
    pub(crate) fn evidence_line(&self, because: &str) -> String {
        match &self.state {
            MemberState::Published(tree) if !because.is_empty() => format!(
                "tree derived now ({because}); {} was read first and does not carry what this \
                 answer names faces by\n",
                tree.out.display(),
            ),
            MemberState::Published(tree) => tree.evidence_line(),
            MemberState::Unpublished { reason, .. } => format!(
                "tree derived now (no published records at {}; {})\n",
                out_dir(&self.dir).display(),
                one_line(reason),
            ),
            MemberState::Unresolvable(reason) => {
                format!("tree unavailable ({})\n", one_line(reason))
            }
        }
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
/// empty host and a broken member from reading as an absent one. Its first four counts
/// are what this census has always said; `published` and `not built` are the evidence
/// axis, added so "read the build's records" and "had to derive" are told apart instead
/// of both reading as `queried`.
/// 普查覆盖**每一个**成员，而每一行都是关于"被读到的那棵树"的事实而不是结果行：正是它让框架 crate
/// 不会被读成一棵空的宿主树，也让坏掉的成员不会被读成不存在的成员。它的前四个计数是这个普查一直以来
/// 说的东西；`published` 与 `not built` 是证据轴，加进来是为了让"读了构建的记录"与"不得不推导"被
/// 区分开，而不是都读成 `queried`。
pub(crate) fn roster(root: &Path, members: &[Member]) -> String {
    let count = |word: &str| members.iter().filter(|m| m.status() == word).count();
    let with_faces = members
        .iter()
        .filter(|member| match &member.state {
            MemberState::Published(tree) => !tree.faces().is_empty(),
            MemberState::Unpublished { faces, .. } => !faces.is_empty(),
            MemberState::Unresolvable(_) => false,
        })
        .count();
    let mut output = format!(
        "workspace {} (virtual manifest: a workspace root is not a package, so it has no identity \
         namespace of its own; each member below has one)\nmembers {}  queried {}  no faces {}  \
         unresolvable {}  published {}  not built {}\n",
        root.display(),
        members.len(),
        with_faces,
        count("no faces"),
        count("unresolvable"),
        count("published"),
        count("not built"),
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
/// The body is the tool's own single-package implementation, handed the member —
/// its directory, its namespace, and the evidence this module already read — so a
/// merged answer and a single-package answer cannot disagree, because they are the
/// same code. A member whose derivation failed gets the `tree unavailable (reason)`
/// sentence in place of a body: the tool has no tree there, and it says so where
/// the reader is looking.
/// 主体就是该工具自己的单包实现，接收成员——它的目录、它的命名空间，以及本模块已经读到的证据——
/// 因此合并答案与单包答案不可能不一致，因为它们是同一段代码。推导失败的成员在主体的位置得到
/// `tree unavailable (原因)` 那一句：那里没有这棵树，而工具就在读者看的地方说出来。
pub(crate) fn merge<F>(
    root: &Path,
    members: &[Member],
    arguments: &Value,
    mut body: F,
) -> Result<String, String>
where
    F: FnMut(&Member, &Value) -> Result<String, String>,
{
    let mut output = roster(root, members);
    for member in members {
        output.push_str(&format!("\n== {} ({})\n", member.name, member.status()));
        match &member.state {
            MemberState::Unresolvable(reason) => {
                output.push_str(&format!("tree unavailable ({reason})\n"));
            }
            _ => output.push_str(&body(member, arguments)?),
        }
    }
    output.push_str(DETAIL);
    Ok(output)
}

/// The closing line every merged tree answer ends with, and the one place that spells it.
/// 每份合并树级答案结尾都会带的那一行，以及拼出它的唯一地方。
///
/// Two shapes of merged answer exist — the one [`merge`] renders, and the narrower ones
/// that resolve one owner or ask every member for its own answer — and they are read by
/// the same caller, so the sentence telling that caller how to get a single package's
/// answer has to read the same in all of them.
/// 合并答案有两种形状——[`merge`] 渲染的那一种，以及只解析出一个拥有者、或逐成员索取各自答案的更窄的
/// 那几种——而读它们的是同一个调用方，因此告诉它"怎么拿到单个包的答案"的那句话在它们之间必须读起来
/// 一样。
pub(crate) const DETAIL: &str = "detail: pass `root` as one member's directory (relative to this \
                                 root) for that package's own answer\n";

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
/// where a reader looks first. The evidence is named in the same row, because "3 faces" is a
/// different fact depending on whether the build published them or this answer just derived
/// them.
/// 丢弃过注册面文件的推导在这里也要说出来：一个还有第四个文件谁也解析不了的包，若只写
/// `queried, 3 faces`，那就会被读成一棵完整的树，而这一行正是读者先看的地方。证据在同一行里点名，
/// 因为"3 个面"是不同的事实，取决于它们是构建发布的还是这份答案刚推导出来的。
fn detail(member: &Member) -> String {
    match &member.state {
        MemberState::Published(tree) => {
            let head = if tree.records_no_faces() {
                NO_FACES_REASON.to_owned()
            } else if tree.faces_unknown() {
                FACES_UNKNOWN.to_owned()
            } else {
                format!("{} faces", tree.faces().len())
            };
            format!("{head}; build {}", tree.freshness())
        }
        MemberState::Unpublished {
            faces,
            unparsable,
            reason,
        } => {
            let head = if faces.is_empty() {
                NO_FACES_REASON.to_owned()
            } else {
                format!("{} faces", faces.len())
            };
            let head = if unparsable.is_empty() {
                head
            } else {
                format!("{head}; {}", unparsable.trim())
            };
            format!("{head}; not built ({})", one_line(reason))
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

/// Read one member's own evidence: its published records when it has them, and the
/// derived tree — the reason it has none — only when it does not.
/// 读取一个成员自己的证据：有已发布记录时读记录，没有时才推导那棵树——以及它为什么没有。
///
/// This is the whole performance decision, in one function. Deriving here would
/// cost the member's entire source walk; reading the record costs a few files. The
/// derivation is not removed, it is made a *fallback*, and the state it produces
/// says which one the answer used.
/// 整个性能决定就在这一个函数里。在这里推导会花掉该成员的整趟源码遍历；读记录只花几个文件。
/// 推导没有被删掉，它变成了**回落**，而它产出的状态会说出答案用的是哪一种。
fn member(dir: PathBuf, name: String) -> Member {
    let state = match published::read(&dir) {
        Publication::Published(tree) => MemberState::Published(tree),
        Publication::NotBuilt(reason) => match derived_faces(&dir, &name) {
            Ok((faces, unparsable)) => MemberState::Unpublished {
                reason,
                faces,
                unparsable,
            },
            Err(reason) => MemberState::Unresolvable(reason),
        },
    };
    Member { name, dir, state }
}

/// Fold a possibly multiline reason into the one line a roster row can carry.
/// 把可能多行的原因折成表头一行装得下的那一行。
///
/// It is `pub(crate)` because the ownership entrance renders the same kind of line for a
/// member that derived its tree but could not answer this request: `one_line` here, not a
/// second folding rule there, is what keeps a two-line Cargo message from breaking the
/// layout of a body that both entrances write.
/// 它是 `pub(crate)`，因为归属入口会为"推导出了树、但答不了这次请求"的成员渲染同一类行：这里
/// 的 `one_line`、而不是那里的第二条折叠规则，才是让两行的 Cargo 消息不会弄坏两个入口所写正文排版的
/// 原因。
pub(crate) fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
#[path = "workspace_tests.rs"]
mod workspace_tests;
