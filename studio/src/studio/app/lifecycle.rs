//! App construction, reload, and registry lifecycle.
//! App 构造、重载与注册表生命周期。

use super::support::package_root;
use super::*;

impl App {
    /// Switch between the four Studio workspaces and initialize their state.
    /// 切换四个 Studio 工作区并初始化对应状态。
    pub fn open_page(&mut self, page: StudioPage) {
        self.page = page;
        match page {
            StudioPage::Inspect => self.overlay = None,
            StudioPage::Search => {
                self.overlay = Some(Overlay::Search(SearchState::default()));
            }
            StudioPage::Data => {
                let (center, center_function) = self
                    .selected_info()
                    .map(|info| (Some(info.id), Some(info.source.function.to_owned())))
                    .unwrap_or((None, None));
                self.overlay = Some(Overlay::Search(SearchState {
                    graph_mode: true,
                    center,
                    center_function,
                    ..SearchState::default()
                }));
            }
            StudioPage::Compare => {
                self.overlay = Some(Overlay::Search(SearchState {
                    compare_query: Some(String::new()),
                    active_pane: 1,
                    ..SearchState::default()
                }));
            }
        }
    }

    fn new(registry: Registry, event: String) -> Self {
        let selected = registry
            .depth_first()
            .first()
            .map_or(registry.id(), |info| info.id);
        Self {
            registry,
            runtime_trace: sample_live_trace(),
            mir_graph: None,
            selected,
            page: StudioPage::Inspect,
            details_selected: 0,
            collapsed: BTreeSet::new(),
            focus: Focus::Tree,
            overlay: None,
            event,
            reload_error: None,
            should_quit: false,
            editor_request: None,
            tree_area: ratatui::layout::Rect::default(),
            details_area: ratatui::layout::Rect::default(),
            workspace_area: ratatui::layout::Rect::default(),
            overlay_area: ratatui::layout::Rect::default(),
            overlay_list_area: ratatui::layout::Rect::default(),
            overlay_compare_list_area: ratatui::layout::Rect::default(),
            delete_cancel_area: ratatui::layout::Rect::default(),
            delete_confirm_area: ratatui::layout::Rect::default(),
            action_cancel_area: ratatui::layout::Rect::default(),
            action_validate_area: ratatui::layout::Rect::default(),
            action_edit_area: ratatui::layout::Rect::default(),
            action_confirm_area: ratatui::layout::Rect::default(),
            action_exit_area: ratatui::layout::Rect::default(),
            tree_offset: 0,
            split_percent: 45,
            graph_split_percent: 44,
            graph_area: ratatui::layout::Rect::default(),
            graph_detail_area: ratatui::layout::Rect::default(),
            graph_provenance_area: ratatui::layout::Rect::default(),
            graph_callers_area: ratatui::layout::Rect::default(),
            graph_center_area: ratatui::layout::Rect::default(),
            graph_callees_area: ratatui::layout::Rect::default(),
            graph_tree_a_area: ratatui::layout::Rect::default(),
            graph_tree_b_area: ratatui::layout::Rect::default(),
            graph_data_a_area: ratatui::layout::Rect::default(),
            graph_data_b_area: ratatui::layout::Rect::default(),
            graph_a_input_area: ratatui::layout::Rect::default(),
            graph_a_center_area: ratatui::layout::Rect::default(),
            graph_a_output_area: ratatui::layout::Rect::default(),
            graph_b_input_area: ratatui::layout::Rect::default(),
            graph_b_center_area: ratatui::layout::Rect::default(),
            graph_b_output_area: ratatui::layout::Rect::default(),
            graph_dragging_divider: false,
            dragging_divider: false,
            last_source_stamp: source_stamp(),
            last_source_check: Instant::now(),
        }
    }

    pub fn load() -> Self {
        let mut app = match load_registry() {
            Ok(registry) => Self::new(
                registry,
                "Ready. Press / to search or a to add a registration face.".to_owned(),
            ),
            Err(error) => Self::new(
                load_registry().unwrap_or_else(|_| Registry::root()),
                format!("Registration startup failed:\n{error}"),
            ),
        };
        if let Ok(query) = std::env::var("NICH_LINK_INITIAL_QUERY") {
            app.overlay = Some(Overlay::Search(SearchState {
                query,
                ..SearchState::default()
            }));
        }
        app
    }

    /// Refresh a file-backed registration snapshot after an external save.
    /// 外部编辑器保存后刷新文件型注册快照。
    ///
    /// This checks a compact mtime/size stamp at most twice per second. It
    /// does not rebuild Rust code or touch the live trace, so idle Studio stays
    /// cheap and a failed edit keeps the last valid snapshot visible.
    /// 每秒最多检查两次紧凑的修改时间/大小戳；不会重编译 Rust，也不会清空
    /// 实时追踪。编辑失败时继续显示上一份有效快照。
    pub fn poll_hot_reload(&mut self) {
        if self.last_source_check.elapsed() < Duration::from_millis(500) {
            return;
        }
        self.last_source_check = Instant::now();
        let stamp = source_stamp();
        if stamp == self.last_source_stamp {
            return;
        }
        self.last_source_stamp = stamp;
        let before = self.selected;
        match load_registry() {
            Ok(registry) => {
                self.registry = registry;
                self.reload_error = None;
                if before != self.registry.id() && self.registry.find(before).is_some() {
                    self.selected = before;
                } else if self.registry.find(self.selected).is_none() {
                    self.selected = self.registry.id();
                }
                self.details_selected = self
                    .details_selected
                    .min(self.detail_field_count().saturating_sub(1));
                self.event = "Hot reload: registration snapshot refreshed.".to_owned();
            }
            Err(error) => {
                self.reload_error = Some(ReloadError {
                    phase: "hot_reload",
                    message: error.clone(),
                });
                self.event = "Hot reload failed; keeping the last healthy snapshot.".to_owned();
            }
        }
    }

    pub fn selected_info(&self) -> Option<&RegistrationSnapshot> {
        self.registry.find(self.selected)
    }

    /// Number of rows shown by the main face inspector.
    /// 主注册面检视器显示的字段行数。
    pub fn detail_field_count(&self) -> usize {
        if self.selected_info().is_some() {
            14
        } else {
            2
        }
    }

    /// Ask rustc for a one-shot in-memory MIR snapshot.
    /// 直接请求 rustc 一次，将 MIR 快照留在内存中。
    pub(super) fn reload(&mut self) {
        match load_registry() {
            Ok(registry) => {
                self.registry = registry;
                self.reload_error = None;
                if self.selected != self.registry.id()
                    && self.registry.find(self.selected).is_none()
                {
                    self.selected = self.registry.id();
                }
                self.event = "Registration snapshot reloaded from disk.".to_owned();
            }
            Err(error) => {
                self.reload_error = Some(ReloadError {
                    phase: "reload",
                    message: error.clone(),
                });
                self.event = "Reload failed; keeping the last healthy snapshot.".to_owned();
            }
        }
    }

    pub(super) fn build_all(&mut self) {
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        let project_root = package_root();
        self.event = match Command::new(cargo)
            .args(["build", "--quiet", "--release", "--all-targets"])
            .current_dir(project_root)
            .output()
        {
            Ok(output) if output.status.success() => "Final build passed.".to_owned(),
            Ok(output) => format!(
                "Final build failed:\n{}",
                String::from_utf8_lossy(&output.stderr)
            ),
            Err(error) => format!("Cannot invoke Cargo: {error}"),
        };
    }
}
