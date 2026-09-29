//! The external graft plans on disk, and whether the host entry keeps them.
//! 磁盘上的外部 graft 计划，以及宿主入口是否保住它们。
//!
//! A plan under `.nichlink/external-grafts/<selector>/graft.plan` is an authoring record
//! the build never opens. When the host entry's `static_graft_plan!` does not name the
//! slot a plan targets, the release prunes that slot and the record can never take
//! effect — the build warns, and a `cargo` log is where that warning goes to die. The CLI
//! has answered this since `nichlink grafts`; this is the same answer for an agent, and
//! the rule behind both is `crate::build_time::graft_plan_rows`, so they cannot
//! disagree. A declaration is about the **slot** a plan targets, not the implementation
//! the plan selects — that is the build's own question — so each row shows the plan's
//! target and graft *and* the declaration's own cut and graft, and a difference between
//! them is visible rather than silently accepted or silently rejected. Read-only: it opens
//! files and writes nothing.
//! `.nichlink/external-grafts/<selector>/graft.plan` 下的计划是构建从不打开的创作记录。宿主入口的
//! `static_graft_plan!` 没有点名计划所针对的槽位时，发布态会剪掉那个槽位，这条记录便永远无法生效
//! ——构建会警告，而 `cargo` 日志正是那条警告湮没的地方。CLI 从 `nichlink grafts` 起就在回答这个问题；
//! 这里是给代理的同一个答案，而两者背后的规则是 `crate::build_time::graft_plan_rows`，因此它们
//! 不可能给出不同答案。声明针对的是计划所瞄准的**槽位**，而不是计划选择的那个实现——那正是构建自己的
//! 问题——因此每行既给出计划的目标与 graft，也给出声明自己的切口与 graft，两者不同时是**看得见**的，
//! 而不是被默默接受或默默拒绝。只读：只打开文件，不写任何东西。

use std::path::Path;

use crate::build_time::{declared_grafts, face_views, graft_plan_rows};
use serde_json::Value;

use crate::mcp::protocol::DEFAULT_LIMIT;
use crate::mcp::registry::namespace;

/// The most plan rows one reply carries before it says it truncated.
/// 一条回复在声明被截断之前最多携带的计划条目数。
const MAX_ROWS: usize = 400;

/// Report every external graft plan and whether the host entry declares its slot.
/// 报告每条外部 graft 计划，以及宿主入口是否声明了它的槽位。
pub(crate) fn grafts(root: &Path, arguments: &Value) -> Result<String, String> {
    let namespace = namespace(root)?;
    let faces = face_views(root, &namespace)?;
    let limit = arguments
        .get("limit")
        .and_then(Value::as_u64)
        .map_or(DEFAULT_LIMIT, |value| {
            value.clamp(1, MAX_ROWS as u64) as usize
        });
    // The entry and the plans are read independently: an unreadable entry leaves the
    // declaration state unknown rather than turning every plan into "not declared",
    // which is a different — and wrong — answer.
    // 入口与计划各自独立读取：入口读不了只会让声明状态未知，而不会把每条计划都变成"未被声明"——
    // 那是另一个（而且错误的）答案。
    let declared = declared_grafts(root);
    let entry_line = match &declared {
        Ok(declared) => format!("host entry {}\n", declared.entry.display()),
        Err(error) => format!("host entry unreadable ({error})\n"),
    };
    let rows = graft_plan_rows(root, &faces, declared.as_ref().ok())?;
    let mut output = format!("namespace {namespace}\n{entry_line}");
    if let Err(error) = &declared {
        let _ = error;
        output.push_str(
            "declaration state unknown: no plan below can be told whether the release keeps it\n",
        );
    }
    output.push_str(&format!("plans {}\n", rows.len()));
    if rows.is_empty() {
        output.push_str("no external graft plans under .nichlink/external-grafts/\n");
        return Ok(output);
    }
    // The headline count is over *every* row, not over the rows the bound happened to
    // print: this number is what a maintainer reads before a release prunes the slot, and
    // counting inside the truncating loop silently rewrote it to whatever `limit` allowed.
    // 头条计数是**全部**行，而不是上限恰好打印出来的那些：这个数字是维护者在发布剪掉槽位之前读的
    // 东西，而在截断循环里计数会把它静默改写成 `limit` 允许的那部分。
    let unkept = rows
        .iter()
        .filter(|row| row.error.is_none() && row.declared == Some(false))
        .count();
    for (index, row) in rows.iter().enumerate() {
        if index >= limit {
            output.push_str(&format!("  … +{} more\n", rows.len() - limit));
            break;
        }
        match &row.error {
            Some(error) => {
                output.push_str(&format!("  {}: unreadable ({error})\n", row.selector));
            }
            None => {
                let verdict = match row.declared {
                    Some(true) => match &row.declared_by {
                        Some(cut) => format!(
                            "declared at entry line {} as cut `{}` graft `{}`",
                            cut.line,
                            cut.cut_label(),
                            cut.graft
                        ),
                        None => "declared".to_owned(),
                    },
                    Some(false) => "NOT declared by the host entry".to_owned(),
                    None => "declaration unknown".to_owned(),
                };
                output.push_str(&format!(
                    "  {}: target={} graft={} full={} [{verdict}]\n",
                    row.selector,
                    row.target_path.as_deref().unwrap_or("<unknown>"),
                    row.graft.as_deref().unwrap_or("<unknown>"),
                    row.full
                        .map(|full| full.to_string())
                        .unwrap_or_else(|| "<unknown>".to_owned()),
                ));
            }
        }
    }
    if unkept > 0 {
        output.push_str(&format!(
            "unkept plans {unkept}: the release prunes these slots, so the records can never take \
             effect. Add a `static_graft_plan!` declaration naming each one to the host entry.\n"
        ));
    }
    Ok(output)
}

#[cfg(test)]
#[path = "grafts_tests.rs"]
mod grafts_tests;
