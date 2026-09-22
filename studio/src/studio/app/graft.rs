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
//!   screen writes. It is consumed here and by nothing else.
//!
//! * 宿主入口的 `static_graft_plan!` 是构建读取的**声明**，用来保住槽位并填充发布态
//!   静态计划。
//! * `Registry::overlay` 是运行期产生有效树的**应用**；它不改动任何一棵树。
//! * `.nichlink/external-grafts/` 下的 `graft.plan` 是本界面写下的**记录**，只在这里
//!   被消费。

use super::support::{package_root, with_authoring_context};
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
        let created = with_authoring_context(|| {
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
                let declaration = plan.document.declaration();
                self.open_editor_file(path.clone(), 1);
                self.event = format!(
                    "External graft plan created at {}; declare the slot in the host entry: {declaration}",
                    path.display()
                );
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
        match with_authoring_context(|| {
            nichlink_run_method::rewrite_external_graft(selector, !full)
        }) {
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
        match with_authoring_context(|| nichlink_run_method::remove_external_graft(selector)) {
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
        .map(|info| nichlink_build_method::source_module_path(&info.source.file))
        .unwrap_or_default();
    state.declaration = match nichlink_build_method::declared_grafts(&package_root()) {
        Ok(declared) => declaration_for(&state.target_path, &module, &declared),
        Err(reason) => GraftDeclaration::Unknown { reason },
    };
}

/// The declaration, if any, that names this face's slot.
/// 命名这个注册面槽位的声明（若有）。
fn declaration_for(path: &str, module: &str, declared: &DeclaredGrafts) -> GraftDeclaration {
    for cut in &declared.cuts {
        if graft_names_face(cut, path, module) {
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

/// Whether one declared cut hands over exactly this face.
/// 一条声明的切口是否正好交出这个注册面。
///
/// A typed cut names its target with a Rust path, so it is matched through the
/// same module mapping the build uses; a string cut names the logical path, and
/// a range names both of its endpoints.
/// 类型化切口用 Rust 路径命名目标，因此通过与构建相同的模块映射来匹配；字符串切口
/// 命名逻辑路径；区间切口命名它的两个端点。
fn graft_names_face(cut: &DeclaredGraft, path: &str, module: &str) -> bool {
    let normalized = |value: &str| {
        value
            .strip_prefix("crate::")
            .or_else(|| value.strip_prefix("self::"))
            .unwrap_or(value)
            .to_owned()
    };
    match &cut.expressions {
        Some(expressions) => {
            let wanted = format!("{module}::NODE_ID");
            normalized(&expressions.cut) == wanted
                || expressions
                    .cut_end
                    .as_deref()
                    .is_some_and(|end| normalized(end) == wanted)
        }
        None => {
            cut.cut == path
                || cut
                    .cut
                    .split_once(" to ")
                    .is_some_and(|(start, end)| start.trim() == path || end.trim() == path)
        }
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
        None => format!(
            "cut \"{}\"{} graft \"{}\"",
            cut.cut,
            if cut.full { " full" } else { "" },
            cut.graft
        ),
    }
}

/// A selector the filesystem and the plan format both accept.
/// 文件系统与计划格式都能接受的选择器。
fn graft_selector_error(selector: &str) -> Option<String> {
    nichlink_run_method::validate_graft_selector(selector.trim()).err()
}

/// Where a plan by this selector lives, in the same layout the writer uses.
/// 该选择器的计划所在位置，与写入方使用同一套布局。
fn plan_path_for(selector: &str) -> PathBuf {
    package_root()
        .join(".nichlink/external-grafts")
        .join(selector.trim())
        .join("graft.plan")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn declared(cut: DeclaredGraft) -> DeclaredGrafts {
        DeclaredGrafts {
            entry: PathBuf::from("src/lib.rs"),
            cuts: vec![cut],
        }
    }

    fn string_cut(path: &str) -> DeclaredGraft {
        DeclaredGraft {
            cut: path.to_owned(),
            graft: "button_fast".to_owned(),
            full: false,
            cfg: None,
            expressions: None,
            line: 4,
        }
    }

    #[test]
    fn a_string_cut_names_the_logical_path() {
        let declared = declared(string_cut("root/control/button"));
        assert_eq!(
            declaration_for("root/control/button", "control::object::button", &declared),
            GraftDeclaration::Declared {
                expression: "cut \"root/control/button\" graft \"button_fast\"".to_owned(),
                line: 4,
                cfg: None,
            }
        );
        assert!(matches!(
            declaration_for("root/control/slider", "control::object::slider", &declared),
            GraftDeclaration::Absent { .. }
        ));
    }

    #[test]
    fn a_typed_cut_names_the_module_from_the_crate_root() {
        let declared = declared(DeclaredGraft {
            cut: "crate::control::object::button::NODE_ID".to_owned(),
            graft: "control_button_graft::button_fast::NODE_ID".to_owned(),
            full: true,
            cfg: None,
            expressions: Some(nichlink_build_method::DeclaredGraftExpressions {
                cut: "crate::control::object::button::NODE_ID".to_owned(),
                cut_end: None,
                graft: "control_button_graft::button_fast::NODE_ID".to_owned(),
            }),
            line: 7,
        });
        assert_eq!(
            declaration_for("root/control/button", "control::object::button", &declared),
            GraftDeclaration::Declared {
                expression:
                    "cut(crate::control::object::button::NODE_ID) full graft(control_button_graft::button_fast::NODE_ID)"
                        .to_owned(),
                line: 7,
                cfg: None,
            }
        );
        // The build strips `crate::` before comparing; so does this check.
        assert!(matches!(
            declaration_for("root/control/button", "other::module", &declared),
            GraftDeclaration::Absent { .. }
        ));
    }

    #[test]
    fn a_range_cut_names_both_endpoints() {
        let declared = declared(string_cut("root/a to root/c"));
        for path in ["root/a", "root/c"] {
            assert!(matches!(
                declaration_for(path, "a", &declared),
                GraftDeclaration::Declared { .. }
            ));
        }
        assert!(matches!(
            declaration_for("root/b", "b", &declared),
            GraftDeclaration::Absent { .. }
        ));
    }

    #[test]
    fn selectors_that_escape_the_plan_directory_are_refused() {
        assert!(graft_selector_error("button_fast").is_none());
        assert!(graft_selector_error(" ").is_some());
        assert!(graft_selector_error("a/b").is_some());
        assert!(graft_selector_error("a\\b").is_some());
    }
}
