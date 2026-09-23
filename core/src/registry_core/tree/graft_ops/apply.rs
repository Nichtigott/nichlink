//! Applying reconciled graft records over declarations: the overlay half.
//! 在声明之上应用已对账的嫁接记录：覆盖那一半。
//!
//! This page turns surviving records into one `GraftPlan` and delegates to the
//! same private overlay path an ordinary `overlay_static` uses, so hard failures
//! stay byte-identical. It owns precedence (record versus declaration form) and
//! the "untouched declaration is passed through as its typed cut" rule.
//! 本页把存活记录变成一份 `GraftPlan`，并委派给普通 `overlay_static` 使用的同一条
//! 私有覆盖路径，因此硬失败保持逐字节相同。它拥有优先级（记录对声明形式）与
//! “未受影响的声明按类型化切口原样传入”的规则。

// Split decision: application is separated from reconciliation because it is the
// only page allowed to touch the overlay machinery and the precedence policy;
// the identity rules it consumes are intentionally opaque here, so a report
// variant cannot change which cut is produced.
// 拆分决定：应用与对账分开，因为它是唯一允许触碰覆盖机制与优先级策略的页面；它消费
// 的身份规则在此刻意不透明，因此一个报告变体无法改变产出的切口。

use std::collections::{BTreeMap, BTreeSet};

use crate::registry_core::diagnostic::RegistryResult;
use crate::registry_core::identity::NodeId;
use crate::registry_core::plugin::graft::{GraftCut, GraftPlan};
use crate::registry_core::release::{CutTarget, StaticGraftCut};

use super::super::Registry;
use super::super::overlay::{GraftCutRef, Resolution};
use super::RecordedGraft;
use super::reports::RecordReport;

/// The effective tree a record set produced, plus the evidence for it.
/// 记录集产生的有效树，以及支持它的证据。
pub struct RecordedOverlay {
    /// The tree after every surviving record was applied over the declarations.
    /// 应用所有存活记录后得到的有效树。
    pub effective: Registry,
    /// The advisory evidence emitted while applying, in encounter order.
    /// 应用过程中按遇到顺序发出的提示性证据。
    pub reports: Vec<RecordReport>,
}

/// One record that won its slot and the cut it contributes.
/// 一条赢得槽位的记录，以及它贡献的切口。
struct WinningRecord {
    path: String,
    graft: String,
    full: bool,
}

/// Which backing store a planned cut borrows from, so declared cuts keep their
/// typed `Id` selectors while record cuts carry plain strings.
/// 计划切口借用哪个后备存储，使声明切口保留类型化 `Id` 选择器，而记录切口携带纯字符串。
#[derive(Clone, Copy)]
enum PlannedCut {
    Declared(usize),
    Owned(usize),
}

impl Registry {
    /// Apply graft records over the declarations that keep their slots alive.
    /// 在这些保持槽位活着的声明之上施加嫁接记录。
    ///
    /// Precedence policy / 优先级策略:
    ///
    /// The record is the newest, most specific artifact and the build is
    /// explicitly written never to apply it, so it wins over a **string-form**
    /// declaration (`StaticGraftCut::cut()` is `CutTarget::Path`) and reports
    /// [`RecordReport::DeclarationOverridden`]. A **typed-form** declaration
    /// (`CutTarget::Id`, emitted from `cut(...)`) is the host saying "this exact
    /// linked crate face"; the compiler resolved it and the binary linked it, so
    /// a text file must not defeat it: the declaration stays final and the
    /// record is reported with [`RecordReport::TypedDeclarationKept`]. A record
    /// whose selector the external registry cannot resolve falls back to the
    /// declaration and reports [`RecordReport::RecordSelectorUnresolved`], so a
    /// bad record can never break a graft the declaration could satisfy.
    /// 记录是最新、最具体的产物，而构建被明确写成永不应用它，因此它胜过**字符串形式**
    /// 声明（`StaticGraftCut::cut()` 为 `CutTarget::Path`），并报告
    /// [`RecordReport::DeclarationOverridden`]。**类型化形式**声明（`CutTarget::Id`，
    /// 由 `cut(...)` 发射）是宿主在说“就是这个已链接的 crate 注册面”；编译器解析了它、
    /// 二进制链接了它，因此文本文件不得击败它：声明保持最终，记录以
    /// [`RecordReport::TypedDeclarationKept`] 报告。外部注册机解析不出记录选择器时回退
    /// 到声明并报告 [`RecordReport::RecordSelectorUnresolved`]，坏记录因此永远不会破坏
    /// 声明本可满足的嫁接。
    ///
    /// Why the obvious approach is wrong / 显而易见的做法为何不对:
    ///
    /// Two obvious readings both fail. "Let the record always win" lets an
    /// unreviewed, machine-local `.nichlink/` file re-route shipped behavior and
    /// defeats a linked, compiler-resolved face. "Let the declaration always
    /// win" leaves the record's `graft` with no production reader at all — the
    /// feature is wired but unobservable. Splitting by declaration form keeps the
    /// dynamic-by-name trust the string form already grants
    /// (`resolution.rs::resolve_node`) while refusing to defeat the typed form.
    /// 两种直觉读法都不成立。“总让记录赢”会让未经审查、机器本地的 `.nichlink/` 文件
    /// 重新路由已发布行为，并击败已链接、编译器解析过的注册面；“总让声明赢”则让记录的
    /// `graft` 完全没有生产读者——功能接了线却不可观测。按声明形式区分，既保留了字符串
    /// 形式本就授予的“按名字动态解析”的信任（`resolution.rs::resolve_node`），又拒绝
    /// 击败类型化形式。
    ///
    /// Boundary / 边界: a record can never resurrect a pruned slot (the declared
    /// set is the arbiter), can never select a face the loaded external registry
    /// lacks, and is reported on every overlay. Hard failures from
    /// `overlay_cuts` (ambiguous selector, contract mismatch, duplicate cut) stay
    /// byte-identical to an ordinary overlay because this function delegates to
    /// the same private path with one `GraftPlan`.
    /// 边界：记录永远不能复活被剪掉的槽位（声明的集合才是仲裁者），永远不能选中已加载
    /// 外部注册机里没有的面，并且每次 overlay 都会报告。`overlay_cuts` 的硬失败
    /// （选择器歧义、合同不匹配、重复切口）与普通 overlay 逐字节相同，因为本函数用同一
    /// 份 `GraftPlan` 委派给同一条私有路径。
    ///
    /// Pinned by `record_tests::a_string_declaration_yields_to_the_record`,
    /// `record_tests::a_typed_declaration_stays_final`,
    /// `record_tests::an_unresolved_record_selector_falls_back_to_the_declaration`,
    /// `record_tests::granularity_is_overridden_as_one_atomic_record`, and
    /// `run_method/tests/graft_record.rs::a_record_on_disk_reaches_overlay`.
    /// 由 `record_tests::a_string_declaration_yields_to_the_record`、
    /// `record_tests::a_typed_declaration_stays_final`、
    /// `record_tests::an_unresolved_record_selector_falls_back_to_the_declaration`、
    /// `record_tests::granularity_is_overridden_as_one_atomic_record` 与
    /// `run_method/tests/graft_record.rs::a_record_on_disk_reaches_overlay` 钉住。
    pub fn overlay_recorded(
        &self,
        records: &[RecordedGraft],
        declared: &[StaticGraftCut],
        external: &Registry,
    ) -> RegistryResult<RecordedOverlay> {
        let mut reports = Vec::new();

        // What "kept alive" means is exactly the union of every declaration's
        // resolved slots. Resolving once also surfaces an unresolvable
        // declaration before any record work, so a stale declaration keeps its
        // historical hard failure.
        // “活着”的确切含义就是每条声明解析出的槽位之并集。只解析一次也能在任何记录
        // 工作之前暴露无法解析的声明，因此陈旧声明保持其历史上的硬失败。
        let mut declared_slots = BTreeSet::new();
        let mut declared_targets = Vec::with_capacity(declared.len());
        for cut in declared {
            let targets = self.resolve_cut_targets(GraftCutRef::static_cut(cut))?;
            declared_slots.extend(targets.iter().copied());
            declared_targets.push(targets);
        }

        let mut winners: BTreeMap<NodeId, Vec<WinningRecord>> = BTreeMap::new();
        for record in records {
            let resolved = self.resolve_record(record)?;
            reports.extend(resolved.reports().iter().cloned());
            let Some((slot, path, full)) = resolved.into_slot() else {
                continue;
            };
            if !declared_slots.contains(&slot) {
                reports.push(RecordReport::UnkeptSlot {
                    selector: record.selector.clone(),
                    target_path: record.document.target_path.clone(),
                });
                continue;
            }
            let declaration = declared
                .iter()
                .zip(&declared_targets)
                .find(|(_, targets)| targets.contains(&slot))
                .map(|(cut, _)| *cut)
                .expect("a declared slot belongs to a declaration");
            let declared_graft = declaration.graft().describe();
            let recorded_graft = record.document.graft.clone();

            // A selector that does not resolve uniquely is fell back on for the
            // same reason an absent one is: an unreviewed local record must not
            // turn a declaration the host already ships into a hard failure.
            // 无法唯一解析的选择器与缺失的选择器同等回退，理由相同：未经审查的本地记录
            // 不得把宿主已经发布的声明变成硬失败。
            if !matches!(external.resolve_node(&recorded_graft), Resolution::One(_)) {
                reports.push(RecordReport::RecordSelectorUnresolved {
                    selector: record.selector.clone(),
                    graft: recorded_graft,
                });
                continue;
            }

            match declaration.graft() {
                CutTarget::Id(_) => {
                    reports.push(RecordReport::TypedDeclarationKept {
                        slot,
                        declared: declared_graft,
                        recorded: recorded_graft,
                    });
                }
                CutTarget::Path(_) => {
                    reports.push(RecordReport::DeclarationOverridden {
                        slot,
                        declared: declared_graft,
                        recorded: recorded_graft.clone(),
                    });
                    if declaration.full() != full {
                        reports.push(RecordReport::GranularityOverridden {
                            slot,
                            declared_full: declaration.full(),
                            recorded_full: full,
                        });
                    }
                    winners.entry(slot).or_default().push(WinningRecord {
                        path,
                        graft: recorded_graft,
                        full,
                    });
                }
            }
        }

        // Build one plan. A declaration no record touches is passed through as
        // the static cut itself, so its typed selectors, ranges and error
        // identity stay exactly as an ordinary `overlay_static` produced them.
        // 构建一份计划。没有任何记录触及的声明按静态切口原样传入，因此其类型化选择器、
        // 区间与错误身份与普通 `overlay_static` 完全一致。
        let mut owned_cuts: Vec<GraftCut> = Vec::new();
        let mut planned: Vec<PlannedCut> = Vec::new();
        for (index, (cut, targets)) in declared.iter().zip(&declared_targets).enumerate() {
            let touched = targets.iter().any(|slot| winners.contains_key(slot));
            if !touched {
                planned.push(PlannedCut::Declared(index));
                continue;
            }
            // A range declaration that a record only partly covers is split per
            // node: the overridden node takes the record, every other node keeps
            // the declaration. The plan format has no range field, so a record
            // can never create one.
            // 只被记录部分覆盖的区间声明按节点拆分：被覆盖的节点取记录，其余节点保留声明。
            // 计划格式没有区间字段，因此记录永远无法创建区间。
            let declared_graft = cut.graft().describe();
            for slot in targets {
                if let Some(won) = winners.get(slot) {
                    for item in won {
                        owned_cuts.push(GraftCut {
                            cut: item.path.clone(),
                            graft: item.graft.clone(),
                            end: None,
                            subtree: item.full,
                        });
                        planned.push(PlannedCut::Owned(owned_cuts.len() - 1));
                    }
                } else {
                    let path = self
                        .path_for(*slot)
                        .expect("a resolved cut target has a logical path");
                    owned_cuts.push(GraftCut {
                        cut: path,
                        graft: declared_graft.clone(),
                        end: None,
                        subtree: cut.full(),
                    });
                    planned.push(PlannedCut::Owned(owned_cuts.len() - 1));
                }
            }
        }

        let plan = {
            let mut plan = GraftPlan::new(self.framework());
            plan.cuts = owned_cuts;
            plan
        };
        let cuts = planned.iter().map(|item| match *item {
            PlannedCut::Declared(index) => GraftCutRef::static_cut(&declared[index]),
            PlannedCut::Owned(index) => GraftCutRef::dynamic(&plan.cuts[index]),
        });
        let effective = self.overlay_cuts(self.framework(), cuts, external)?;
        Ok(RecordedOverlay { effective, reports })
    }
}
