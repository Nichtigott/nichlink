//! App construction, reload, and registry lifecycle.
//! App 构造、重载与注册表生命周期。

use super::support::{cargo_rustc_mir, package_root};
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
        }
    }

    fn new(registry: Registry, event: String) -> Self {
        let selected = registry
            .depth_first()
            .first()
            .map_or(registry.id(), |info| info.id);
        Self {
            registry,
            // A session starts with no evidence. `load` looks for this project's
            // trace artifact and replaces this with the rebuilt `CallTrace` only
            // when the artifact passes every identity check; otherwise the trace
            // stays disabled and the legend says so instead of claiming a
            // recording that was never loaded.
            // 会话以“没有证据”开始。`load` 查找本项目的 trace artifact，只有在它通过全部身份
            // 检查时才把这里替换为重建出的 `CallTrace`；否则追踪保持关闭，图例如实说明，而不是
            // 宣称一份从未装入的记录。
            runtime_trace: CallTrace::disabled(),
            trace_status: TraceStatus::Absent,
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
            hot: HotZones::default(),
            #[cfg(feature = "node-graph")]
            graph_flow: None,
            tree_offset: 0,
            split_percent: 45,
            graph_split_percent: 60,
            graph_dragging_divider: false,
            dragging_divider: false,
            last_source_stamp: source_stamp(),
            tree_cache: RefCell::new(Vec::new()),
            last_source_check: Instant::now(),
        }
    }

    /// Build an app from the on-disk registration snapshot.
    /// 从磁盘上的注册快照构建 App。
    ///
    /// A failed load keeps an empty root registry and reports the error in `event`.
    /// 加载失败时保留一个空根注册表，并把错误写入 `event`。
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
        app.install_trace();
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
                self.refresh_open_graft();
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

    /// Registration snapshot of the currently selected node, if it still exists.
    /// 当前选中节点的注册快照（若仍存在）。
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

    /// Reload the registration snapshot from disk.
    /// 从磁盘重新加载注册快照。
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
                self.refresh_open_graft();
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

    /// Ask rustc for a one-shot in-memory MIR snapshot.
    /// 直接请求 rustc 一次，将 MIR 快照留在内存中。
    /// Load a MIR snapshot on request and say what came of it, in one place so
    /// the key binding stays one line.
    /// 按请求载入 MIR 快照并说明结果，集中在一处，使按键绑定保持一行。
    pub(super) fn load_mir_snapshot_report(&mut self) {
        self.event = match self.load_mir_snapshot() {
            Ok(calls) => format!("MIR snapshot loaded in memory: {calls} candidate calls."),
            Err(error) => format!("MIR unavailable: {error}"),
        };
    }

    pub fn load_mir_snapshot(&mut self) -> Result<usize, String> {
        let manifest = host_manifest();
        let output = cargo_rustc_mir(&manifest, &["-Zunpretty=mir"])?;
        if !output.status.success() {
            let detail = String::from_utf8_lossy(&output.stderr);
            return Err(if detail.trim().is_empty() {
                format!("cargo rustc exited with {}", output.status)
            } else if detail.contains("option `Z` is only accepted") {
                "MIR inspection requires a nightly rustc; normal Studio builds stay on stable."
                    .to_owned()
            } else {
                detail.trim().to_owned()
            });
        }
        let graph = MirGraph::from_mir_text(&String::from_utf8_lossy(&output.stdout));
        let calls = graph.calls.len();
        self.mir_graph = Some(graph);
        Ok(calls)
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
