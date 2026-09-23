//! Cross-check external graft plans against the host entry's declaration.
//! 交叉校验外部 graft 计划与宿主入口的声明。
//!
//! A plan file under `.nichlink/external-grafts/` is an authoring record: the
//! build reads `static_graft_plan!` declarations and never opens a plan. When
//! the two disagree, the release-time plan prunes the slot the author meant to
//! hand over, so the record can never take effect. This check refuses that build:
//! shipping a binary whose graft silently does not happen is exactly what must
//! not leave the build, and the declaration list is right here to prove it.
//! `.nichlink/external-grafts/` 下的计划文件是创作记录：构建读的是
//! `static_graft_plan!` 声明，从不打开计划。两者不一致时，发布态计划会剪掉作者想
//! 交出的槽位，于是这条记录永远无法生效。这项检查拒绝这样的构建：发布一个嫁接静默
//! 不发生的二进制，正是绝不能离开构建的东西，而声明清单就在这里可以证明它。
//!
//! The check stays precise rather than conservative: it refuses only when no
//! declaration could name the plan's target slot at all, and a declaration that
//! carries a feature gate counts even when this build evaluates the gate to false,
//! because the gate is the author's business. That distinction is why the caller
//! passes every declaration in the entry, not just the ones this build enables.
//! 这项检查保持精确而不是保守：只有当没有任何声明可能命名计划的目标槽位时才拒绝，而且
//! 带特性门控的声明即使在本次构建里求值为假也算数，因为门控是作者的事。正是这一区别
//! 要求调用方传入入口里的每一条声明，而不只是本次构建启用的那些。

use std::fs;
use std::path::Path;

use nichlink::lexicon;
use nichlink::registry_core::plugin::graft_document::GraftPlanDocument;
use nichlink::{BuildDiagnostic, BuildDiagnostics};

use super::Node;
use super::registry_identity::NodeId;
use super::static_plan::source_module_path;
use super::{DeclaredGraft, node_id, relative_display};

/// One plan file the build found, reduced to what the check needs.
/// 构建找到的一个计划文件，只保留检查所需的内容。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PlannedSlot {
    pub(crate) selector: String,
    pub(crate) target: NodeId,
    pub(crate) target_path: String,
    /// The implementation the plan names, so the diagnostic can hand over the
    /// exact `static_graft_plan!` clause to paste.
    /// 计划命名的实现，使诊断能交出可直接粘贴的那条 `static_graft_plan!` 子句。
    pub(crate) graft: String,
    pub(crate) full: bool,
    /// The plan file itself, so the diagnostic points at the artifact rather than
    /// at a source file that contains no mistake.
    /// 计划文件本身，使诊断指向那件产物，而不是指向一个不含错误的源码文件。
    pub(crate) plan_file: String,
}

/// Every readable plan under the package's external graft directory.
/// 包的 external graft 目录下每个可读计划。
///
/// A directory whose plan is missing or unreadable is skipped here: this check is
/// about the declaration side, and a plan that does not parse is refused on the
/// apply path (`run_method::apply_recorded_grafts`) and shown as broken by the
/// authoring surfaces. `nichlink grafts` lists it too.
/// 计划缺失或读不懂的目录在这里被跳过：这项检查管的是声明那一侧，而解析不了的计划会在
/// 应用路径上被拒绝（`run_method::apply_recorded_grafts`），创作界面也会把它显示为坏
/// 计划。`nichlink grafts` 同样会列出它。
pub(crate) fn planned_slots(root: &Path) -> Vec<PlannedSlot> {
    let directory = root
        .join(lexicon::NICHLINK_DIR)
        .join(lexicon::EXTERNAL_GRAFT_DIR);
    let Ok(entries) = fs::read_dir(&directory) else {
        return Vec::new();
    };
    let mut slots = Vec::new();
    for entry in entries.flatten() {
        if !entry.path().is_dir() {
            continue;
        }
        let plan_file = entry.path().join(lexicon::GRAFT_PLAN_FILE);
        let Ok(text) = fs::read_to_string(&plan_file) else {
            continue;
        };
        let Ok(document) = GraftPlanDocument::parse(&text) else {
            continue;
        };
        // `/` on every platform through the kernel's portable-path rule: the
        // layout is documented with `/` and Windows reported `\` without it.
        // 经内核的可移植路径规则在任何平台都用 `/`：布局按 `/` 记录，不加则会报 `\`。
        let relative_plan = plan_file.strip_prefix(root).unwrap_or(&plan_file);
        slots.push(PlannedSlot {
            selector: entry.file_name().to_string_lossy().into_owned(),
            target: document.target,
            target_path: document.target_path.clone(),
            graft: document.graft.clone(),
            full: document.full,
            plan_file: nichlink::declaration::portable_path(&relative_plan.to_string_lossy()),
        });
    }
    slots.sort_by(|left, right| left.selector.cmp(&right.selector));
    slots
}

/// The source module of the face holding `id`, if the current tree has one.
/// 当前树中持有 `id` 的那个注册面的源码模块（若有）。
pub(crate) fn slot_module(src: &Path, nodes: &[Node], id: NodeId) -> Option<String> {
    fn walk(src: &Path, nodes: &[Node], id: NodeId) -> Option<String> {
        for node in nodes {
            if node_id(src, node) == Some(id) {
                return node
                    .file
                    .as_ref()
                    .map(|file| source_module_path(&relative_display(src, file)));
            }
            if let Some(module) = walk(src, &node.children, id) {
                return Some(module);
            }
        }
        None
    }
    walk(src, nodes, id)
}

/// One build error per plan whose target slot no declaration can name.
/// 每条"没有任何声明可能命名其目标槽位"的计划对应一条构建错误。
///
/// An error rather than a warning: the record can never take effect, and the build
/// is the last place that can say so before a binary ships without the graft its
/// author wrote. `declared` must be the entry's **full** declaration list — a
/// gated-off declaration counts, so this refuses only the genuinely undeclared
/// plan.
/// 这是错误而不是警告：这条记录永远无法生效，而构建是在"二进制带着作者写了却没生效的
/// 嫁接发布出去"之前最后一个能说话的地方。`declared` 必须是入口的**完整**声明清单——
/// 门控关掉的声明也算数，因此这里只拒绝真正没有声明的计划。
pub(crate) fn undeclared_plan_errors(
    slots: &[PlannedSlot],
    declared: &[DeclaredGraft],
    module_of: impl Fn(NodeId) -> Option<String>,
) -> BuildDiagnostics {
    let mut errors = BuildDiagnostics::default();
    for slot in slots {
        let module = module_of(slot.target);
        if declared
            .iter()
            .any(|cut| cut.names_face(&slot.target_path, module.as_deref()))
        {
            continue;
        }
        // The clause, not just the complaint: the author's next action is to paste
        // it into the entry, and the plan already holds every field it needs.
        // 给的是子句而不只是抱怨：作者的下一步就是把它粘进入口，而计划里已经握有它需要的
        // 每个字段。
        let clause = GraftPlanDocument::new(
            slot.target,
            slot.target_path.clone(),
            slot.graft.clone(),
            slot.full,
        )
        .declaration();
        errors.push(
            BuildDiagnostic::new(
                "static-plan",
                format!(
                    "external graft plan `{}` targets `{}`, which no declaration in the host entry names; the release-time plan keeps no such slot alive, so the record could never take effect. Declare it in static_graft_plan!: {clause}",
                    slot.selector, slot.target_path
                ),
            )
            .at(slot.plan_file.clone(), 0),
        );
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DeclaredGraftExpressions;
    use std::path::PathBuf;

    fn string_cut(cut: &str, graft: &str) -> DeclaredGraft {
        DeclaredGraft {
            cut: cut.to_owned(),
            cut_end: None,
            graft: graft.to_owned(),
            full: true,
            cfg: None,
            expressions: None,
            line: 1,
        }
    }

    fn typed_cut(expression: &str, graft: &str) -> DeclaredGraft {
        DeclaredGraft {
            cut: expression.to_owned(),
            cut_end: None,
            graft: graft.to_owned(),
            full: false,
            cfg: None,
            expressions: Some(DeclaredGraftExpressions {
                cut: expression.to_owned(),
                cut_end: None,
                graft: graft.to_owned(),
            }),
            line: 1,
        }
    }

    fn slot(selector: &str, target_path: &str) -> PlannedSlot {
        PlannedSlot {
            selector: selector.to_owned(),
            target: NodeId::from_namespaced_path("host", "control/control.rs", "Control"),
            target_path: target_path.to_owned(),
            graft: "canvas_fast".to_owned(),
            full: true,
            plan_file: format!(".nichlink/external-grafts/{selector}/graft.plan"),
        }
    }

    /// The one error's message, or a panic when the count is not one.
    /// 唯一一条错误的消息；条数不为一时 panic。
    fn only_message(errors: &BuildDiagnostics) -> String {
        let items = errors.iter().collect::<Vec<_>>();
        assert_eq!(items.len(), 1, "{items:?}");
        items[0].message.clone()
    }

    /// A plan for a slot nobody declares is the failure this check exists for:
    /// the release-time plan prunes it, the record can never take effect, and the
    /// build says so with the clause that would fix it.
    /// 没有任何声明交出的计划正是这项检查存在的理由：发布态计划会剪掉它，记录永远无法
    /// 生效，而构建会连同能修好它的那条子句一起说明。
    #[test]
    fn an_undeclared_plan_target_is_an_error_with_the_clause() {
        let errors = undeclared_plan_errors(
            &[slot("canvas_graft", "root/canvas")],
            &[string_cut("root/panel", "panel_fast")],
            |_| None,
        );
        let message = only_message(&errors);
        assert!(message.contains("canvas_graft"), "{message}");
        assert!(message.contains("root/canvas"), "{message}");
        assert!(message.contains("could never take effect"), "{message}");
        // The actionable half: the exact clause, `full` included.
        assert!(
            message.contains(r#"cut "root/canvas" full graft "canvas_fast","#),
            "{message}"
        );
    }

    /// A string cut names the logical path, and a typed cut names the base
    /// face's module. Both are declarations; neither is an error.
    /// 字符串切口命名逻辑路径，类型化切口命名原注册面的模块。两者都是声明，都不报错。
    #[test]
    fn string_and_typed_declarations_both_count() {
        let string_errors = undeclared_plan_errors(
            &[slot("canvas_graft", "root/canvas")],
            &[string_cut("root/canvas", "canvas_fast")],
            |_| None,
        );
        assert!(string_errors.is_empty(), "{string_errors:?}");

        let typed_errors = undeclared_plan_errors(
            &[slot("canvas_graft", "control")],
            &[typed_cut("crate::control::NODE_ID", "canvas_fast")],
            |_| Some("control".to_owned()),
        );
        assert!(typed_errors.is_empty(), "{typed_errors:?}");
    }

    /// A declaration gated off in this build still counts. The gate is the
    /// author's business, so a record for a slot the current feature set compiles
    /// out is a legitimate configuration rather than an authoring mistake; making
    /// the checker read only the enabled declarations is what turned it into a
    /// false error.
    /// 本次构建里被门控关掉的声明仍然算数。门控是作者的事，因此"当前特性组合把该槽位
    /// 编译掉"的记录是合法配置而不是作者失误；让校验器只读启用的声明，正是把它变成误报
    /// 的原因。
    #[test]
    fn a_gated_declaration_still_counts() {
        let mut gated = string_cut("root/canvas", "canvas_fast");
        gated.cfg = Some("feature = \"extra\"".to_owned());
        let errors =
            undeclared_plan_errors(&[slot("canvas_graft", "root/canvas")], &[gated], |_| None);
        assert!(errors.is_empty(), "{errors:?}");
    }

    /// An unresolved target cannot prove a typed cut names it, so only a string
    /// cut can clear the plan.
    /// 无法解析的目标不能证明某条类型化切口命名了它，因此只有字符串切口能解除这条错误。
    #[test]
    fn an_unresolved_target_only_matches_a_string_cut() {
        let errors = undeclared_plan_errors(
            &[slot("canvas_graft", "control")],
            &[typed_cut("crate::control::NODE_ID", "canvas_fast")],
            |_| None,
        );
        assert_eq!(errors.iter().count(), 1, "{errors:?}");
    }

    /// `names_face` reads the far endpoint from `cut_end`, so a range matches
    /// both of its endpoints and a single path that literally contains `" to "`
    /// matches only itself.
    /// `names_face` 从 `cut_end` 读取远端端点，因此区间匹配它的两个端点，而字面含有
    /// `" to "` 的单条路径只匹配它自己。
    #[test]
    fn a_string_range_names_both_endpoints_as_data() {
        let mut range = string_cut("root/a", "canvas_fast");
        range.cut_end = Some("root/c".to_owned());
        assert!(range.names_face("root/a", None));
        assert!(range.names_face("root/c", None));
        assert!(!range.names_face("root/b", None));

        // No split of the path text happens, so no endpoint is invented.
        // 路径文本不会被拆分，因此不会凭空造出端点。
        let literal = string_cut("root/a to b", "canvas_fast");
        assert!(literal.names_face("root/a to b", None));
        assert!(!literal.names_face("root/a", None));
        assert!(!literal.names_face("b", None));
    }

    /// The whole path is read from disk the way the authoring surface writes
    /// it, and a plan directory the tree does not have is simply no plan.
    /// 整条路径按创作界面写下的样子从磁盘读取；树里没有的计划目录就是没有计划。
    #[test]
    fn planned_slots_reads_the_authoring_layout() {
        let root = temporary_directory("graft-plan-slots");
        assert!(planned_slots(&root).is_empty());

        let plan = root
            .join(lexicon::NICHLINK_DIR)
            .join(lexicon::EXTERNAL_GRAFT_DIR)
            .join("canvas_graft");
        fs::create_dir_all(&plan).expect("plan directory");
        let document = GraftPlanDocument::new(
            NodeId::from_namespaced_path("host", "control/control.rs", "Control"),
            "root/canvas",
            "canvas_fast",
            true,
        );
        fs::write(plan.join(lexicon::GRAFT_PLAN_FILE), document.render()).expect("plan file");

        let slots = planned_slots(&root);
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].selector, "canvas_graft");
        assert_eq!(slots[0].target_path, "root/canvas");
        assert_eq!(slots[0].target, document.target);
        // The clause fields and the artifact path the diagnostic points at.
        // 子句所需字段，以及诊断指向的那件产物路径。
        assert_eq!(slots[0].graft, "canvas_fast");
        assert!(slots[0].full);
        assert_eq!(
            slots[0].plan_file,
            ".nichlink/external-grafts/canvas_graft/graft.plan"
        );

        fs::remove_dir_all(&root).expect("temporary fixture cleanup");
    }

    /// A host that declares the plan and also carries the file is the normal
    /// case, and it must stay quiet end to end: entry parsed, slot resolved,
    /// typed declaration matched through the face's module.
    /// 既声明了计划、又带着计划文件的宿主是正常情形，端到端必须保持沉默：入口解析、
    /// 槽位解析、类型化声明经注册面模块匹配都通得过。
    #[test]
    fn a_declared_plan_is_quiet_end_to_end() {
        let root = temporary_directory("graft-plan-declared");
        let src = root.join("src");
        let face = src.join("control");
        fs::create_dir_all(&face).expect("face directory");
        fs::write(
            face.join("control.rs"),
            "crate::root_object! {\n    kind: Control,\n}\n",
        )
        .expect("face source");
        fs::write(
            src.join("lib.rs"),
            "nichlink_run_method::host!();\nnichlink_run_method::static_graft_plan!(FRAMEWORK, cut(crate::control::NODE_ID) graft(canvas_fast));\n",
        )
        .expect("host entry");

        let nodes = crate::discover_root(&src);
        let entry = crate::entry::resolve_host_entry(&src, &nodes, None);
        let declared = crate::host_graft_entries(&entry)
            .declared
            .iter()
            .map(crate::declared_graft_view)
            .collect::<Vec<_>>();
        assert_eq!(declared.len(), 1, "{declared:?}");
        assert!(declared[0].expressions.is_some(), "{declared:?}");

        let plan = root
            .join(lexicon::NICHLINK_DIR)
            .join(lexicon::EXTERNAL_GRAFT_DIR)
            .join("canvas_graft");
        fs::create_dir_all(&plan).expect("plan directory");
        let target = first_face_id(&src, &nodes).expect("face identity");
        let document = GraftPlanDocument::new(target, "control", "canvas_fast", true);
        fs::write(plan.join(lexicon::GRAFT_PLAN_FILE), document.render()).expect("plan file");

        let errors = undeclared_plan_errors(&planned_slots(&root), &declared, |id| {
            slot_module(&src, &nodes, id)
        });
        assert!(errors.is_empty(), "{errors:?}");

        fs::remove_dir_all(&root).expect("temporary fixture cleanup");
    }

    /// A plan for the same slot after the declaration is removed errors again:
    /// silence must come from the declaration, not from the file existing.
    /// 同一条计划在声明被删掉后必须重新报错：沉默只能来自声明，不能来自文件存在。
    #[test]
    fn removing_the_declaration_brings_the_error_back() {
        let root = temporary_directory("graft-plan-undeclared");
        let plan = root
            .join(lexicon::NICHLINK_DIR)
            .join(lexicon::EXTERNAL_GRAFT_DIR)
            .join("canvas_graft");
        fs::create_dir_all(&plan).expect("plan directory");
        let document = GraftPlanDocument::new(
            NodeId::from_namespaced_path("host", "control/control.rs", "Control"),
            "control",
            "canvas_fast",
            true,
        );
        fs::write(plan.join(lexicon::GRAFT_PLAN_FILE), document.render()).expect("plan file");

        let errors = undeclared_plan_errors(
            &planned_slots(&root),
            &[string_cut("root/panel", "panel_fast")],
            |_| None,
        );
        assert_eq!(errors.iter().count(), 1, "{errors:?}");

        fs::remove_dir_all(&root).expect("temporary fixture cleanup");
    }

    fn first_face_id(src: &Path, nodes: &[Node]) -> Option<NodeId> {
        for node in nodes {
            if let Some(id) = crate::node_id(src, node) {
                return Some(id);
            }
            if let Some(id) = first_face_id(src, &node.children) {
                return Some(id);
            }
        }
        None
    }

    /// Keeps two tests in the same process apart even when the clock resolution
    /// collapses their timestamps into one nanosecond; the workspace suite runs
    /// them in parallel and a collision silently mixes two fixtures.
    /// 即使时钟分辨率把两个测试的时间戳压进同一纳秒，也把同一进程内的两者分开；
    /// workspace 套件并行运行它们，撞名会静默把两套夹具混在一起。
    static TEMP_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    fn temporary_directory(label: &str) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let sequence = TEMP_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "nichlink-{label}-{}-{stamp}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("temporary directory");
        root
    }
}
