//! Where one face stands relative to the build's own manifest.
//! 一个面相对构建自己的清单处于什么状态。
//!
//! `nichlink.diff` states the whole delta and `nichlink.search` annotates each hit
//! with the same verdict, and the verdict is one rule: a face the build published
//! is `ok`, a face it never saw is `added since build`, and a face whose *source*
//! the build recorded under a different identity is `re-identified`. Two copies of
//! that rule would let the diff and the search disagree about the same face, which
//! is exactly the drift the tree vocabulary exists to prevent — so it lives here,
//! once, and both tools read it.
//! `nichlink.diff` 说出整份差异，`nichlink.search` 给每个命中标注同一个结论，而这个结论是一条规则：
//! 构建发布过的面是 `ok`，构建从未见过的面是 `added since build`，而构建把它的**源码**记在另一个身份
//! 之下的面是 `re-identified`。这条规则有两份副本，就会让 diff 与 search 对同一个面给出不同说法——
//! 而这正是这棵树的词汇要消除的漂移——因此它只住在这里，两个工具都读它。

use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use crate::build_time::{FaceView, PruningRow, build_output_is_current, read_pruning_manifest};
use nichlink_kernel::identity::NodeId;

use crate::mcp::build_evidence::out_dir;

/// What the build says about one face the sources declare right now.
/// 构建对源码此刻声明的某个面给出的说法。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FaceStatus {
    /// The build published this exact identity.
    /// 构建发布的正是这个身份。
    Ok,
    /// The build has no face at this source under any identity.
    /// 构建在这个源码位置没有任何身份的面。
    AddedSinceBuild,
    /// The build recorded this source under a different identity — a `kind`
    /// change under an unmoved file, which no text diff sees.
    /// 构建把这段源码记在另一个身份之下——文件没动而 `kind` 变了，文本 diff 看不见。
    Reidentified(NodeId),
}

impl FaceStatus {
    /// The word for the `ok` status.
    /// `ok` 状态的那个词。
    pub(crate) const OK: &'static str = "ok";
    /// The word for a face the build never saw.
    /// 构建从未见过的面的那个词。
    pub(crate) const ADDED_SINCE_BUILD: &'static str = "added since build";
    /// The word for a face the build recorded under another identity.
    /// 构建记在另一个身份之下的面的那个词。
    pub(crate) const REIDENTIFIED: &'static str = "re-identified";

    /// The word this status is printed as, everywhere it is printed.
    /// 这个状态在任何被打印的地方所用的那个词。
    ///
    /// `status` unifies the *rule*, and this unifies the *word*: without it the
    /// same face came back `reidentified` from `nichlink.diff`'s count line and
    /// `re-identified` from `nichlink.search`'s annotation, which is the drift
    /// the tree vocabulary exists to prevent. The count line names its buckets
    /// before any face is in hand, so it reads the constants above — the words
    /// still live here and nowhere else.
    /// `status` 统一的是**规则**，这里统一的是**词形**：没有它，同一个面会从 `nichlink.diff`
    /// 的计数行回来成 `reidentified`、从 `nichlink.search` 的标注回来成 `re-identified`——
    /// 而这正是这棵树的词汇要消除的漂移。计数行在任何面到手之前就说出它的桶名，因此它读上面的
    /// 常量；词形仍然只住在这里。
    pub(crate) fn label(&self) -> &'static str {
        match self {
            Self::Ok => Self::OK,
            Self::AddedSinceBuild => Self::ADDED_SINCE_BUILD,
            Self::Reidentified(_) => Self::REIDENTIFIED,
        }
    }
}

/// The built side of the source-versus-build comparison, read once.
/// 源码对构建比较中"构建那一侧"，只读一次。
pub(crate) struct TreeDelta {
    /// Whether the published output still describes these sources.
    /// 已发布的产物是否仍在描述这批源码。
    pub(crate) current: bool,
    /// Whether a pruning manifest was readable at all.
    /// 是否读到了修剪清单。
    pub(crate) known: bool,
    /// The identities the build published.
    /// 构建发布过的身份。
    ids: BTreeSet<NodeId>,
    /// The identity the build published per source path.
    /// 构建为每个源码路径发布过的身份。
    by_source: HashMap<String, NodeId>,
    /// The manifest rows, for the caller that reports a removal by source.
    /// 清单行，供按源码报告移除的调用方使用。
    pub(crate) rows: Vec<PruningRow>,
}

impl TreeDelta {
    /// Read the build's manifest for this package.
    /// 读取本包构建的清单。
    ///
    /// A missing manifest is data, not an error: it is the answer to "why does
    /// nothing here know whether it ships". An output that no longer describes
    /// these sources is still read — the rows are the build's own record — and
    /// `current` carries the caveat so each caller states it in its own words.
    /// 清单缺失是数据而不是错误：它正是"这里为什么没有东西知道自己发不发布"的答案。不再描述这批源码
    /// 的产物仍会被读取——那些行是构建自己的记录——而 `current` 携带那声提醒，让每个调用方用自己的话
    /// 说出它。
    pub(crate) fn read(root: &Path) -> Self {
        let out = out_dir(root);
        let current = build_output_is_current(root, &out);
        match read_pruning_manifest(&out) {
            Ok(rows) => {
                let ids = rows.iter().map(|row| row.id).collect();
                let by_source = rows
                    .iter()
                    .map(|row| (row.source.clone(), row.id))
                    .collect();
                Self {
                    current,
                    known: true,
                    ids,
                    by_source,
                    rows,
                }
            }
            Err(_) => Self {
                current,
                known: false,
                ids: BTreeSet::new(),
                by_source: HashMap::new(),
                rows: Vec::new(),
            },
        }
    }

    /// Where one face stands, in the one vocabulary both tools use.
    /// 一个面的状态，用两个工具共用的那一套词汇。
    pub(crate) fn status(&self, face: &FaceView) -> FaceStatus {
        if let Some(previous) = self.by_source.get(&face.source) {
            // The identity is a hash over the source path and the kind, so a
            // recorded source whose identity differs is a changed `kind` — the one
            // piece of an identity that can move without the file moving.
            // 身份是对源码路径与 kind 的散列，因此记录了同一路径而身份不同，就是 kind 变了——
            // 这是身份输入里唯一能在文件不动的情况下移动的。
            return if *previous == face.id {
                FaceStatus::Ok
            } else {
                FaceStatus::Reidentified(*previous)
            };
        }
        if self.ids.contains(&face.id) {
            FaceStatus::Ok
        } else {
            FaceStatus::AddedSinceBuild
        }
    }
}
