//! Advisory reports a recorded-graft reconciliation can emit.
//! 记录嫁接对账可能发出的提示性报告。
//!
//! Reports are evidence, not failures: a host prints them and keeps the effective
//! tree it was handed. Two record defects refuse the overlay instead, and neither
//! is representable here: identity and path naming different faces, and a
//! directory selector that disagrees with the plan's `graft`. The vocabulary
//! lives alone so a new report variant is a change to one page, not to the
//! reconciliation or application loops that merely carry it.
//! 报告是提示性证据而不是失败：宿主打印它们，并使用拿到的有效树。两种记录缺陷会拒绝
//! 整次 overlay，而且都无法在这里表达：身份与路径指向不同的面，以及目录选择器与计划里
//! 的 `graft` 不一致。词表单独成页，因此新增一个报告变体只改这一页，而不会碰到仅仅
//! 携带它的对账或应用循环。

// Split decision: the advisory vocabulary is its own page because it is the one
// part of the record pipeline that grows on its own — a new observation must not
// edit the reconciliation match or the overlay loop that merely forwards it.
// 拆分决定：提示性词表单独成页，因为它是记录管线中唯一会自行增长的部分——新增一个
// 观察不应改动仅仅转发它的对账分支或覆盖循环。

use std::fmt;

use crate::registry_core::identity::NodeId;

/// What reconciliation noticed while turning records into overlay cuts.
/// 把记录变成 overlay 切口时对账注意到的情况。
///
/// Reports are advisory evidence, not failures: a host prints them and keeps the
/// effective tree it was handed. The two record defects that refuse the overlay
/// — identity and path naming different faces, and a directory selector that
/// disagrees with the plan's `graft` — are errors, not variants here.
/// 报告是提示性证据而不是失败：宿主打印它们，并使用拿到的有效树。两种会拒绝整次
/// overlay 的记录缺陷——身份与路径指向不同的面，以及目录选择器与计划里的 `graft`
/// 不一致——是错误，而不是这里的变体。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecordReport {
    /// The stored identity and stored path named the same slot only after one of
    /// them was reinterpreted; the record was applied at the live face.
    /// 存储的身份与存储的路径要经过一次重新解释才指向同一槽位；记录已施加到现存注册面。
    IdentityDrifted {
        /// The directory selector the record was stored under.
        /// 记录存储所用的目录选择器。
        selector: String,
        /// The durable identity the record stored for the face.
        /// 记录为该注册面存储的耐久身份。
        target: NodeId,
        /// The stored logical path, used only while the identity resolves.
        /// 存储的逻辑路径，仅在身份可解析时使用。
        target_path: String,
    },
    /// No declaration keeps the record's slot alive in *this* build, so the record
    /// is skipped: either that declaration is `#[cfg]`-gated off here, or the
    /// record was written after the build that produced the static plan. A plan no
    /// declaration could *ever* name is refused at build time instead.
    /// 本次构建里没有声明让记录的槽位活着，因此记录被跳过：要么那条声明在这里被 `#[cfg]`
    /// 关掉了，要么记录写在产出静态计划的那次构建之后。没有任何声明**可能**命名的计划则会在
    /// 构建期被拒绝。
    UnkeptSlot {
        /// The directory selector the record was stored under.
        /// 记录存储所用的目录选择器。
        selector: String,
        /// The stored path no declaration keeps alive.
        /// 没有任何声明让它活着的存储路径。
        target_path: String,
    },
    /// The record and the declaration disagree about replacing a subtree.
    /// 记录与声明对“是否替换整棵子树”意见不一致。
    GranularityOverridden {
        /// The face whose replacement granularity the record changed.
        /// 记录改变了替换粒度的注册面。
        slot: NodeId,
        /// The subtree-replacement flag the declaration asked for.
        /// 声明请求的整棵子树替换标志。
        declared_full: bool,
        /// The flag the record puts in its place.
        /// 记录取而代之的标志。
        recorded_full: bool,
    },
    /// A string-form declaration was superseded by the record.
    /// 一条字符串形式声明被记录取代。
    DeclarationOverridden {
        /// The face whose string declaration the record replaced.
        /// 记录取代其字符串声明的注册面。
        slot: NodeId,
        /// The implementation name the string declaration wrote.
        /// 字符串声明写下的实现名。
        declared: String,
        /// The implementation name the record puts in its place.
        /// 记录取而代之的实现名。
        recorded: String,
    },
    /// A typed-form declaration was left final and the record ignored.
    /// 一条类型化形式声明保持最终，记录被忽略。
    TypedDeclarationKept {
        /// The face whose typed declaration stays final.
        /// 类型化声明保持最终的注册面。
        slot: NodeId,
        /// The typed implementation name that was kept.
        /// 被保留的类型化实现名。
        declared: String,
        /// The record's implementation name that was ignored.
        /// 被忽略的记录实现名。
        recorded: String,
    },
    /// The record names an implementation the external registry cannot resolve.
    /// 记录命名了一个外部注册机无法解析的实现。
    RecordSelectorUnresolved {
        /// The directory selector the record was stored under.
        /// 记录存储所用的目录选择器。
        selector: String,
        /// The implementation name the external registry cannot resolve.
        /// 外部注册机无法解析的实现名。
        graft: String,
    },
}

impl fmt::Display for RecordReport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IdentityDrifted {
                selector,
                target,
                target_path,
            } => write!(
                formatter,
                "graft record `{selector}`: target {target} no longer matches stored path `{target_path}`; applied at the face's current path"
            ),
            Self::UnkeptSlot {
                selector,
                target_path,
            } => write!(
                formatter,
                "graft record `{selector}` addresses `{target_path}`, which no declaration keeps alive; record skipped"
            ),
            Self::GranularityOverridden {
                slot,
                declared_full,
                recorded_full,
            } => write!(
                formatter,
                "graft record changes `full` at {slot} from {declared_full} to {recorded_full}"
            ),
            Self::DeclarationOverridden {
                slot,
                declared,
                recorded,
            } => write!(
                formatter,
                "graft record overrides string declaration `{declared}` at {slot} with `{recorded}`"
            ),
            Self::TypedDeclarationKept {
                slot,
                declared,
                recorded,
            } => write!(
                formatter,
                "graft record `{recorded}` ignored at {slot}: typed declaration `{declared}` is final"
            ),
            Self::RecordSelectorUnresolved { selector, graft } => write!(
                formatter,
                "graft record `{selector}` names `{graft}`, which the external registry does not resolve; the declaration is kept"
            ),
        }
    }
}
