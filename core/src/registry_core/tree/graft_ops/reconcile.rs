//! Identity/path reconciliation of one graft record against a live tree.
//! 把一条嫁接记录的身份与路径同现存树对账。
//!
//! This page owns the "which slot does this record address" half: it re-derives
//! the current logical path from the durable identity, and decides whether the
//! record survives at all. Whether a surviving record then wins its slot is the
//! application half in `apply`.
//! 本页拥有"这条记录指向哪个槽位"的那一半：它从耐久身份重新推导当前逻辑路径，并判定
//! 记录是否存活。存活记录是否赢得槽位则是 `apply` 中的应用那一半。

// Split decision: "which slot" and "who wins it" are separated because the
// identity/path boundary rules (drift, contradiction, missing slot) are pure
// lookup logic, while precedence and plan construction need the overlay
// machinery. Keeping them apart lets each be reasoned about without the other.
// 拆分决定：把“指向哪个槽位”与“谁赢得它”分开，因为身份/路径边界规则（漂移、矛盾、
// 槽位缺失）是纯查找逻辑，而优先级与计划构建需要覆盖机制。分开后两者都能独立推敲。

use crate::registry_core::declaration::SourceLocation;
use crate::registry_core::diagnostic::{RegistryError, RegistryResult};
use crate::registry_core::identity::NodeId;
use crate::registry_core::plugin::graft::document::GraftPlanDocument;

use super::super::Registry;
use super::RecordedGraft;
use super::reports::RecordReport;

/// The outcome of reconciling one record against the base tree alone.
/// 仅凭原树对一条记录做对账的结果。
///
/// `Unkept` is not an error: the record is skipped and every other record still
/// applies. Two things [`Registry::resolve_record`] refuses outright, because
/// either one would silently change what the author meant: an identity and a path
/// naming *different* faces, and a directory selector disagreeing with the
/// `graft` its plan names.
/// `Unkept` 不是错误：记录被跳过，其余记录照常应用。[`Registry::resolve_record`] 直接
/// 拒绝两种情形，因为任一种都会静默改变作者的本意：身份与路径指向**不同**面，以及目录
/// 选择器与计划里的 `graft` 不一致。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolvedRecord {
    /// The record addresses a slot the base tree still has.
    /// 记录指向原树仍然拥有的槽位。
    Slot {
        /// The live slot the record addresses.
        /// 记录指向的现存槽位。
        slot: NodeId,
        /// The slot's **current** logical path, not the stored `target_path`.
        /// 槽位的**当前**逻辑路径，而不是存储的 `target_path`。
        path: String,
        /// The subtree-replacement flag the record requested.
        /// 记录请求的整棵子树替换标志。
        full: bool,
        /// What reconciliation noticed about this record.
        /// 对账针对该记录注意到的情况。
        reports: Vec<RecordReport>,
    },
    /// No declaration can keep the slot alive; the record is skipped.
    /// 没有任何声明能让槽位活着；记录被跳过。
    Unkept {
        /// What reconciliation noticed about this record.
        /// 对账针对该记录注意到的情况。
        reports: Vec<RecordReport>,
    },
}

impl ResolvedRecord {
    /// Everything noticed while resolving, whether or not a slot was found.
    /// 解析过程中注意到的一切，无论是否找到了槽位。
    pub fn reports(&self) -> &[RecordReport] {
        match self {
            Self::Slot { reports, .. } | Self::Unkept { reports } => reports,
        }
    }

    /// The resolved slot, its current path, and the requested granularity.
    /// 解析出的槽位、其当前路径，以及请求的粒度。
    pub fn into_slot(self) -> Option<(NodeId, String, bool)> {
        match self {
            Self::Slot {
                slot, path, full, ..
            } => Some((slot, path, full)),
            Self::Unkept { .. } => None,
        }
    }
}

impl Registry {
    /// Reconcile one record's stored identity and stored path against this tree.
    /// 把一条记录存储的身份与路径同本树对账。
    ///
    /// The record is applied at the face's **current** logical path
    /// (`self.path_for`), never at the stored `target_path`.
    /// 记录施加在注册面的**当前**逻辑路径（`self.path_for`）上，而绝不是存储的
    /// `target_path` 上。
    ///
    /// Why the obvious approach is wrong / 显而易见的做法为何不对:
    ///
    /// The intuitive reading is "the record already stores `target_path`, so use
    /// it as the cut selector". That text was written when the plan was created,
    /// and a `GraftCut.cut` is a string `resolve_path` must match *exactly* —
    /// there is no leniency for a renamed face. A record that survives one file
    /// rename would therefore miss the very slot it still names by identity, and
    /// the host would silently overlay nothing. The identity is the durable half;
    /// the path is the human half. So the path is re-derived from the identity,
    /// and the stored text is only used when the identity no longer resolves.
    /// 直觉读法是“记录本来就存了 `target_path`，拿它当切口选择器即可”。那段文本是
    /// 计划创建时写下的，而 `GraftCut.cut` 是 `resolve_path` 必须**精确**匹配的字符串，
    /// 对改过名的注册面没有任何宽容。于是记录只要经历一次文件改名，就会错过它仍以身份
    /// 命名的那个槽位，宿主便静默地什么都没覆盖。身份是耐久的一半，路径是给人看的一半。
    /// 因此路径从身份重新推导，存储的文本只在身份不再可解析时才使用。
    ///
    /// Boundary / 边界: identity and path agreeing is clean; one missing is drift
    /// (applied, reported); both missing is `Unkept` (skipped, reported); each of
    /// the two silent-intent-change cases is an `Err` — identity and path naming
    /// different faces, and a directory selector disagreeing with the plan's
    /// `graft` — because picking either candidate would silently change the
    /// author's intent.
    /// 边界：身份与路径一致为干净；缺一个为漂移（应用并报告）；两个都缺为 `Unkept`
    /// （跳过并报告）；两种"会静默改变作者本意"的情形都是 `Err`——身份与路径指向不同的
    /// 面，以及目录选择器与计划里的 `graft` 不一致——因为任选其一都会静默改变作者的
    /// 本意。
    ///
    /// Pinned by `record_tests::identity_and_path_agree_resolves_to_the_target`,
    /// `record_tests::a_missing_identity_resolves_by_path_with_drift`,
    /// `record_tests::contradictory_identity_and_path_are_refused`,
    /// `record_tests::a_record_with_no_live_slot_is_skipped_not_fatal`, and
    /// `record_tests::a_directory_that_disagrees_with_its_plan_is_refused`.
    /// 由 `record_tests::identity_and_path_agree_resolves_to_the_target`、
    /// `record_tests::a_missing_identity_resolves_by_path_with_drift`、
    /// `record_tests::contradictory_identity_and_path_are_refused`、
    /// `record_tests::a_record_with_no_live_slot_is_skipped_not_fatal` 与
    /// `record_tests::a_directory_that_disagrees_with_its_plan_is_refused` 钉住。
    pub fn resolve_record(&self, record: &RecordedGraft) -> RegistryResult<ResolvedRecord> {
        let document = &record.document;
        let mut reports = Vec::new();
        // A renamed directory (or a hand-edited `graft=`) leaves two candidate
        // implementation names and no way to tell which one the author meant. The
        // overlay used to pick the file's and carry on, but "the graft silently
        // addressed a different implementation than its directory says" is not a
        // state an author can see in the source tree, so it is refused.
        // 改名过的目录（或被手工改过的 `graft=`）会留下两个候选实现名，且无法判断作者
        // 指的是哪一个。overlay 过去直接取文件里的那个继续，但"这次嫁接静默地指向了与
        // 目录名不同的实现"是作者在源码树里看不到的状态，因此现在直接拒绝。
        if record.selector != document.graft {
            return Err(Box::new(RegistryError::new(
                document.target,
                self.current_path(document.target, document),
                SourceLocation {
                    file: "<graft>",
                    line: 0,
                    column: 0,
                    function: "Registry::resolve_record",
                },
                format!(
                    "graft record directory `{}` and its plan's `graft={}` disagree; rename the directory or the file so the selector and the implementation name agree",
                    record.selector, document.graft
                ),
            )));
        }

        let by_id = self.find(document.target).map(|info| info.id);
        let by_path = self.resolve_path(&document.target_path);
        match (by_id, by_path) {
            (Some(a), Some(b)) if a == b => Ok(ResolvedRecord::Slot {
                slot: a,
                path: self.current_path(a, document),
                full: document.full,
                reports,
            }),
            (Some(a), None) => {
                reports.push(RecordReport::IdentityDrifted {
                    selector: record.selector.clone(),
                    target: document.target,
                    target_path: document.target_path.clone(),
                });
                Ok(ResolvedRecord::Slot {
                    slot: a,
                    path: self.current_path(a, document),
                    full: document.full,
                    reports,
                })
            }
            (None, Some(b)) => {
                reports.push(RecordReport::IdentityDrifted {
                    selector: record.selector.clone(),
                    target: document.target,
                    target_path: document.target_path.clone(),
                });
                Ok(ResolvedRecord::Slot {
                    slot: b,
                    path: self.current_path(b, document),
                    full: document.full,
                    reports,
                })
            }
            (Some(a), Some(b)) => {
                let id_path = self.current_path(a, document);
                let path_path = self.current_path(b, document);
                Err(Box::new(RegistryError::new(
                    document.target,
                    id_path.clone(),
                    SourceLocation {
                        file: "<graft>",
                        line: 0,
                        column: 0,
                        function: "Registry::resolve_record",
                    },
                    format!(
                        "graft record `{}` is contradictory: target {} names `{}`, but target_path `{}` names `{}`",
                        record.selector, document.target, id_path, document.target_path, path_path
                    ),
                )))
            }
            (None, None) => {
                reports.push(RecordReport::UnkeptSlot {
                    selector: record.selector.clone(),
                    target_path: document.target_path.clone(),
                });
                Ok(ResolvedRecord::Unkept { reports })
            }
        }
    }

    fn current_path(&self, slot: NodeId, document: &GraftPlanDocument) -> String {
        self.path_for(slot)
            .unwrap_or_else(|| document.target_path.clone())
    }
}
