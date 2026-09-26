//! External graft authoring: compose a plan, read the host entry, manage plans.
//! 外部 graft 创作：撰写计划、读取宿主入口、管理已有计划。
//!
//! Three layers meet here and the screen keeps them apart:
//! 三个层次在此相遇，界面把它们分清楚：
//!
//! * `static_graft_plan!` in the host entry is the **declaration** the build
//!   reads to keep a slot alive and to fill the release-time static plan.
//! * `Registry::overlay` is the **application** that produces the effective
//!   tree at runtime; it never edits either registry.
//! * `graft.plan` under `.nichlink/external-grafts/` is the **record** this
//!   screen writes. The screen consumes it here, and the runtime overlay path
//!   (`run_method::apply_recorded_grafts` → `Registry::overlay_recorded`)
//!   consumes it too.
//!
//! * 宿主入口的 `static_graft_plan!` 是构建读取的**声明**，用来保住槽位并填充发布态
//!   静态计划。
//! * `Registry::overlay` 是运行期产生有效树的**应用**；它不改动任何一棵树。
//! * `.nichlink/external-grafts/` 下的 `graft.plan` 是本界面写下的**记录**：本界面在
//!   这里消费它，运行期覆盖路径（`run_method::apply_recorded_grafts` →
//!   `Registry::overlay_recorded`）也会消费它。

use super::support::{package_root, with_authoring_context};
use super::writers::with_selected_project;
use super::*;
use nichlink_build_method::{DeclaredGraft, DeclaredGrafts};

impl App {
    /// Open the graft screen for the selected face.
    /// 为当前选中的注册面打开 graft 界面。
    pub(super) fn open_graft(&mut self) {
        let Some(info) = self.selected_info() else {
            return;
        };
        let target = info.id;
        let target_path = self
            .registry
            .path_for(target)
            .unwrap_or_else(|| "root".to_owned());
        let selector = format!("{}_graft", info.registry_name);
        let flow_declared = info.flow.is_declared();
        let inherited_children = self
            .registry
            .registry(target)
            .map_or(0, |child| child.len());
        let mut state = GraftState {
            target,
            target_path,
            selector,
            full: false,
            field: 0,
            pane: 0,
            editing: false,
            plans: Vec::new(),
            plan_selected: 0,
            pending_delete: None,
            declaration: GraftDeclaration::Unknown {
                reason: "host entry not read yet".to_owned(),
            },
            flow_declared,
            inherited_children,
        };
        graft_facts(&self.registry, &mut state);
        self.overlay = Some(Overlay::Graft(state));
    }

    /// Re-read the plans on disk and the entry's declarations.
    /// 重新读取磁盘上的计划与入口的声明。
    pub(super) fn refresh_graft(&mut self) {
        let Some(Overlay::Graft(state)) = self.overlay.as_mut() else {
            return;
        };
        graft_facts(&self.registry, state);
    }

    /// Refresh the graft screen only when it is the open overlay.
    /// 仅当 graft 界面正打开时刷新它。
    pub(super) fn refresh_open_graft(&mut self) {
        if matches!(self.overlay, Some(Overlay::Graft(_))) {
            self.refresh_graft();
        }
    }

    /// Write the composed plan, or open the one that is already there.
    /// 写入撰写的计划，或打开已经存在的那一条。
    pub(super) fn submit_graft(&mut self, state: &GraftState) {
        if let Some(error) = graft_selector_error(&state.selector) {
            self.event = format!("Graft failed: {error}");
            return;
        }
        let selector = state.selector.trim();
        if let Some(existing) = state.plans.iter().find(|plan| plan.selector == selector) {
            if existing.is_valid()
                && existing.target_path == state.target_path
                && existing.full == state.full
            {
                self.open_graft_plan(selector);
                self.event = format!(
                    "External graft `{selector}` already declares `{}`; opened {}",
                    state.target_path,
                    plan_path_for(selector).display()
                );
                return;
            }
            self.event = format!(
                "Graft failed: `{selector}` already exists at {}; open it (o), delete it (d), or choose another selector",
                plan_path_for(selector).display()
            );
            return;
        }
        let created = with_selected_project(|| {
            nichlink_run_method::create_external_graft(
                &self.registry,
                state.target,
                selector,
                state.full,
            )
        });
        match created {
            Ok(plan) => {
                let path = plan.plan_path();
                // The paste-ready line comes from the same kernel constructor and
                // renderer the written record goes through. Hand-formatting a copy
                // here is how the banner and the record start to disagree.
                // 可粘贴的那一行由写出的记录所用的同一个内核构造器与渲染器生成。在这里
                // 手写一份副本，正是横幅与记录开始不一致的原因。
                let declaration = plan.document.declaration();
                self.open_editor_file(path.clone(), 1);
                self.event = match &state.declaration {
                    // Writing the record first and declaring the slot afterwards is
                    // a legitimate order, so the plan is still written. What the
                    // banner must not do is let that order hide the consequence:
                    // the release prunes a slot no declaration names, and the
                    // runtime then skips this record as `UnkeptSlot` — no graft is
                    // applied, and nothing fails loudly. So it is a warning, it
                    // says which file is missing the declaration, and it carries
                    // the exact clause to add.
                    // 先写记录、后声明槽位是合法的顺序，因此计划照旧写入。横幅不能做的
                    // 是让这个顺序掩盖后果：发布态会剪掉没有声明命名的槽位，运行期随后
                    // 把这条记录当作 `UnkeptSlot` 跳过——不会应用任何嫁接，也不会有任何
                    // 响亮的失败。因此它是一条警告，说明缺少声明的是哪个文件，并带上要补
                    // 的那条子句。
                    GraftDeclaration::Absent { entry } => format!(
                        "Warning: External graft plan created at {}; the host entry {} declares no such slot, so the release prunes `{}` and the runtime skips this record (UnkeptSlot) instead of applying it. Add to static_graft_plan!:\n{declaration}",
                        path.display(),
                        entry.display(),
                        state.target_path,
                    ),
                    GraftDeclaration::Unknown { reason } => format!(
                        "Warning: External graft plan created at {}; the host entry could not be read ({reason}), so the release may prune `{}` and the runtime would skip this record (UnkeptSlot). Add to static_graft_plan!:\n{declaration}",
                        path.display(),
                        state.target_path,
                    ),
                    GraftDeclaration::Declared { line, .. } => format!(
                        "External graft plan created at {}; the host entry already declares this slot at line {line}:\n{declaration}",
                        path.display(),
                    ),
                };
            }
            Err(error) => self.event = format!("Graft failed: {error}"),
        }
    }

    /// Open one plan file in the configured editor.
    /// 用配置的编辑器打开一个计划文件。
    pub(super) fn open_graft_plan(&mut self, selector: &str) {
        match with_authoring_context(|| nichlink_run_method::read_external_graft(selector)) {
            Ok(plan) => {
                let path = plan.plan_path();
                self.open_editor_file(path.clone(), 1);
                self.event = format!("Opened external graft plan {}", path.display());
            }
            Err(error) => self.event = format!("Graft failed: {error}"),
        }
    }

    /// Switch one plan between node and subtree replacement.
    /// 在"只替换节点"与"替换整棵子树"之间切换一条计划。
    pub(super) fn toggle_graft_plan(&mut self, selector: &str, full: bool) {
        match with_selected_project(|| nichlink_run_method::rewrite_external_graft(selector, !full))
        {
            Ok(plan) => {
                self.event = format!(
                    "External graft `{selector}` now {} `{}`",
                    if plan.full() {
                        "replaces"
                    } else {
                        "keeps the children of"
                    },
                    plan.target_path()
                );
            }
            Err(error) => self.event = format!("Graft failed: {error}"),
        }
    }

    /// Move one plan to the recoverable trash.
    /// 把一条计划移到可恢复的回收目录。
    pub(super) fn delete_graft_plan(&mut self, selector: &str) {
        match with_selected_project(|| nichlink_run_method::remove_external_graft(selector)) {
            Ok(trash) => {
                self.event = format!(
                    "External graft `{selector}` moved to {}; press r to reload",
                    trash.display()
                );
            }
            Err(error) => self.event = format!("Graft failed: {error}"),
        }
    }

    /// The plan an open/delete/toggle key should act on.
    /// 打开/删除/切换键应当作用的那条计划。
    ///
    /// The list pane acts on its highlighted row; the compose pane acts on the
    /// selector being typed, but only when a plan by that name already exists.
    /// 列表面板作用于高亮行；撰写面板作用于正在输入的选择器，但仅当同名计划已存在。
    pub(super) fn selected_graft_selector(&self, state: &GraftState) -> Option<String> {
        if state.pane == 1 {
            return state
                .plans
                .get(state.plan_selected)
                .map(|plan| plan.selector.clone());
        }
        let selector = state.selector.trim();
        state
            .plans
            .iter()
            .any(|plan| plan.selector == selector)
            .then(|| selector.to_owned())
    }
}

/// Refresh the plans and the entry declaration shown by one graft screen.
/// 刷新一个 graft 界面显示的计划与入口声明。
fn graft_facts(registry: &Registry, state: &mut GraftState) {
    state.plans = with_authoring_context(nichlink_run_method::list_external_grafts)
        .unwrap_or_default()
        .into_iter()
        .map(|entry| match entry.document {
            Ok(document) => GraftPlanRow {
                selector: entry.selector,
                target_path: document.target_path,
                full: document.full,
                error: None,
            },
            Err(error) => GraftPlanRow {
                selector: entry.selector,
                target_path: String::new(),
                full: false,
                error: Some(error),
            },
        })
        .collect();
    state.plan_selected = state.plan_selected.min(state.plans.len().saturating_sub(1));
    if state.plans.is_empty() {
        state.pane = 0;
    }

    let module = registry
        .find(state.target)
        .map(|info| nichlink_build_method::source_module_path(&info.source.file));
    state.declaration = match nichlink_build_method::declared_grafts(&package_root()) {
        Ok(declared) => declaration_for(&state.target_path, module.as_deref(), &declared),
        Err(reason) => GraftDeclaration::Unknown { reason },
    };
}

/// The declaration, if any, that names this face's slot.
/// 命名这个注册面槽位的声明（若有）。
///
/// The one cut/face matching rule lives in `DeclaredGraft::names_face`, the
/// same rule the build uses; its `module` is the face's source module, or
/// `None` when the target cannot be resolved in the current tree.
/// 唯一的切口/注册面匹配规则位于 `DeclaredGraft::names_face`，与构建使用的是同一条
/// 规则；其 `module` 是该注册面的源码模块，目标在当前树里解析不出来时为 `None`。
fn declaration_for(
    path: &str,
    module: Option<&str>,
    declared: &DeclaredGrafts,
) -> GraftDeclaration {
    for cut in &declared.cuts {
        if cut.names_face(path, module) {
            return GraftDeclaration::Declared {
                expression: describe_graft(cut),
                line: cut.line,
                cfg: cut.cfg.clone(),
            };
        }
    }
    GraftDeclaration::Absent {
        entry: declared.entry.clone(),
    }
}

/// Render one declaration the way an author reads it.
/// 按作者阅读的方式渲染一条声明。
fn describe_graft(cut: &DeclaredGraft) -> String {
    match &cut.expressions {
        Some(expressions) => {
            let range = expressions
                .cut_end
                .as_deref()
                .map_or(String::new(), |end| format!(" to {end}"));
            format!(
                "cut({}{range}){} graft({})",
                expressions.cut,
                if cut.full { " full" } else { "" },
                expressions.graft
            )
        }
        None => match &cut.cut_end {
            // A range is rendered in the bracketed two-literal form the macro
            // grammar accepts, not as one quoted path: `cut "a to b"` is a
            // single path that literally contains the range word, so echoing the
            // endpoints as text would hand the author a clause that means
            // something else.
            // 区间渲染成宏语法接受的“两个带方括号字面量”形式，而不是一条带引号的路径：
            // `cut "a to b"` 是字面含有区间词的单条路径，把端点当文本回显会给作者一句
            // 含义不同的子句。
            Some(end) => format!(
                "cut [\"{}\" to \"{end}\"]{} graft \"{}\"",
                cut.cut,
                if cut.full { " full" } else { "" },
                cut.graft
            ),
            None => format!(
                "cut \"{}\"{} graft \"{}\"",
                cut.cut,
                if cut.full { " full" } else { "" },
                cut.graft
            ),
        },
    }
}

/// A selector the filesystem and the plan format both accept.
/// 文件系统与计划格式都能接受的选择器。
fn graft_selector_error(selector: &str) -> Option<String> {
    nichlink_run_method::validate_graft_selector(selector.trim())
        .err()
        .map(|error| error.to_string())
}

/// Where a plan by this selector lives, in the same layout the writer uses.
/// 该选择器的计划所在位置，与写入方使用同一套布局。
fn plan_path_for(selector: &str) -> PathBuf {
    nichlink_run_method::graft_record_root(&package_root())
        .join(selector.trim())
        .join(nichlink_run_method::lexicon::GRAFT_PLAN_FILE)
}

#[cfg(test)]
#[path = "graft_tests.rs"]
mod graft_tests;
