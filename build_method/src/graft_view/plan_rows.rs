//! The external graft plans on disk, joined with the declarations that keep them.
//! 磁盘上的外部 graft 计划，与保住它们的那些声明对照。
//!
//! A plan under `.nichlink/external-grafts/<selector>/graft.plan` is an authoring
//! record the build never opens; the build only *warns* when the host entry's
//! `static_graft_plan!` does not name the slot the plan targets, and the release then
//! prunes that slot so the record can never take effect. That warning is easy to miss
//! in a long `cargo` log, so one question — "does the host entry declare what this plan
//! addresses" — has two surfaces asking it: the CLI's `grafts` verb and the MCP
//! bridge's `nichlink.grafts`. The rule lives here so they cannot answer differently.
//! `.nichlink/external-grafts/<selector>/graft.plan` 下的计划是构建从不打开的创作记录；只有
//! 宿主入口的 `static_graft_plan!` 没有点名该计划所针对的槽位时构建才**警告**，而发布随后会剪掉
//! 那个槽位，使这条记录永远无法生效。在冗长的 `cargo` 日志里这条警告容易被漏掉，因此"宿主入口是否
//! 声明了这条计划所针对的东西"这一个问题有两个执行面在问：CLI 的 `grafts` 与 MCP 桥的
//! `nichlink.grafts`。规则住在这里，两者就不可能给出不同答案。

use std::path::Path;

use nichlink::identity::NodeId;
use nichlink::lexicon;
use nichlink::plugin::graft_document::GraftPlanDocument;

use crate::face_view::FaceView;
use crate::graft_view::{DeclaredGraft, DeclaredGrafts};

/// One plan directory's answer.
/// 一个计划目录给出的答案。
///
/// A directory whose plan is missing or unparseable becomes a row carrying the reason
/// instead of being silently skipped: the caller is asking exactly whether the file is
/// usable. `declared` is `None` when the host entry itself could not be read, which is
/// a different answer from "the entry was read and does not name this slot".
/// 计划缺失或解析不了的目录会成为携带原因的条目，而不是被静默跳过：调用方问的正是"这个文件能不能
/// 用"。宿主入口本身读不了时 `declared` 为 `None`，这与"入口读到了、但没有点名这个槽位"是不同答案。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraftPlanRow {
    /// The plan directory's name, which is also the implementation selector.
    /// 计划目录的名字，同时也是实现选择器。
    pub selector: String,
    /// Why the plan could not be read, when it could not be.
    /// 计划读不了时的原因（若读不了）。
    pub error: Option<String>,
    /// The identity the plan targets.
    /// 计划针对的身份。
    pub target: Option<NodeId>,
    /// The logical path the plan targets.
    /// 计划针对的逻辑路径。
    pub target_path: Option<String>,
    /// The replacement the plan selects.
    /// 计划选择的替换件。
    pub graft: Option<String>,
    /// Whether the replacement covers the target's whole subtree.
    /// 替换是否覆盖目标的整棵子树。
    pub full: Option<bool>,
    /// Whether the host entry declares this slot; `None` when the entry is unreadable.
    /// 宿主入口是否声明了该槽位；入口读不了时为 `None`。
    pub declared: Option<bool>,
    /// The declaration that keeps this plan, when one does.
    /// 保住这条计划的那条声明（若有）。
    pub declared_by: Option<DeclaredGraft>,
}

/// Read every plan under `<package_root>/.nichlink/external-grafts/*/graft.plan`, sorted
/// by selector, and join each with the declaration that keeps it.
/// 读取 `<package_root>/.nichlink/external-grafts/*/graft.plan` 下的每个计划、按 selector
/// 排序，并把每一条与保住它的声明对照起来。
///
/// `faces` is what maps a plan's stored identity back to a module, and a typed cut can
/// only prove itself through that mapping; a target the current tree does not have stays
/// unresolved, so only a string cut can then match. A missing plans directory is the
/// ordinary "no plans" answer; a directory that exists and cannot be read is not, and
/// saying "no plans" there would answer a question nobody could answer.
/// `faces` 把计划里存的身份映射回模块，而类型化切口只能通过那次映射证明自己；当前树没有的目标保持
/// 未解析，此时只有字符串切口可能匹配。计划目录不存在是普通的"没有计划"；存在却读不了的则不是，在那里
/// 回一句"没有计划"等于回答了一个谁也答不出的问题。
pub fn graft_plan_rows(
    package_root: &Path,
    faces: &[FaceView],
    declared: Option<&DeclaredGrafts>,
) -> Result<Vec<GraftPlanRow>, String> {
    let directory = package_root
        .join(lexicon::NICHLINK_DIR)
        .join(lexicon::EXTERNAL_GRAFT_DIR);
    let entries = match std::fs::read_dir(&directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(format!("cannot read {}: {error}", directory.display()));
        }
    };
    let mut rows = Vec::new();
    for entry in entries.flatten() {
        if !entry.path().is_dir() {
            continue;
        }
        let selector = entry.file_name().to_string_lossy().into_owned();
        let plan = entry.path().join(lexicon::GRAFT_PLAN_FILE);
        let text = match std::fs::read_to_string(&plan) {
            Ok(text) => text,
            Err(error) => {
                rows.push(GraftPlanRow::unreadable(
                    selector,
                    format!("cannot read {}: {error}", plan.display()),
                ));
                continue;
            }
        };
        let document = match GraftPlanDocument::parse(&text) {
            Ok(document) => document,
            Err(error) => {
                rows.push(GraftPlanRow::unreadable(selector, error.to_string()));
                continue;
            }
        };
        let module = faces
            .iter()
            .find(|face| face.id == document.target)
            .map(|face| face.module.as_str());
        let matched = declared.and_then(|declared| {
            declared
                .cuts
                .iter()
                .find(|cut| cut.names_face(&document.target_path, module))
        });
        let declared_state = match (declared, matched) {
            (None, _) => None,
            (Some(_), Some(_)) => Some(true),
            (Some(_), None) => Some(false),
        };
        rows.push(GraftPlanRow {
            selector,
            error: None,
            target: Some(document.target),
            target_path: Some(document.target_path.clone()),
            graft: Some(document.graft.clone()),
            full: Some(document.full),
            declared: declared_state,
            declared_by: matched.cloned(),
        });
    }
    rows.sort_by(|left, right| left.selector.cmp(&right.selector));
    Ok(rows)
}

impl GraftPlanRow {
    /// A row for a plan directory whose plan could not be read.
    /// 计划读不了时的那条记录。
    fn unreadable(selector: String, error: String) -> Self {
        Self {
            selector,
            error: Some(error),
            target: None,
            target_path: None,
            graft: None,
            full: None,
            declared: None,
            declared_by: None,
        }
    }
}
