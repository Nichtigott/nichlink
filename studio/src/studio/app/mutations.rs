//! App mutations and editor handoff.
//! App 文件变更与编辑器交接。

use super::support::{package_root, select_project, with_authoring_context};
use super::*;
use nichlink_run_method::pascal_case;

impl App {
    pub(super) fn submit_new_project(&mut self, project: &NewProjectState) {
        let directory = project.values[0].trim();
        let package = project.values[1].trim();
        let kind = project.values[2].trim();
        if directory.is_empty() || package.is_empty() {
            self.event = "New project failed: directory and package are required".to_owned();
            return;
        }
        if !matches!(kind, "binary" | "library") {
            self.event = "New project failed: kind must be binary or library".to_owned();
            return;
        }
        if !package
            .chars()
            .all(|ch| ch == '_' || ch == '-' || ch.is_ascii_alphanumeric())
        {
            self.event =
                "New project failed: package must use letters, digits, '_' or '-'".to_owned();
            return;
        }
        let root = std::path::PathBuf::from(directory);
        let root = if root.is_absolute() {
            root
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .join(root)
        };
        let source = nichlink_build_method::scaffold::detected_source(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")),
            &std::env::current_exe().unwrap_or_default(),
        );
        let kind = if kind == "library" {
            nichlink_build_method::scaffold::ProjectKind::Library
        } else {
            nichlink_build_method::scaffold::ProjectKind::Binary
        };
        if let Err(error) =
            nichlink_build_method::scaffold::create_project(&root, package, kind, &source)
        {
            self.event = format!("New project failed: {error}");
            return;
        }
        // Keep the new project visible immediately. The next reload reads its
        // folder-backed faces; no manual environment setup or restart needed.
        // 立即切换到新项目；下一次 reload 会读取它的文件注册面，无需手动设置环境变量。
        select_project(root.clone(), root.join("Cargo.toml"), package);
        self.reload();
        self.event = format!(
            "Created {kind} project at {}; Studio switched to it",
            root.display()
        );
        self.overlay = None;
    }

    /// Take the pending editor handoff, leaving none queued behind it.
    /// 取走待处理的编辑器交接请求，取走后不再有排队项。
    ///
    /// Returns the file and the 1-based line to open; `None` when nothing is pending.
    /// 返回要打开的文件与从 1 开始的行号；没有待处理请求时为 `None`。
    pub fn take_editor_request(&mut self) -> Option<(PathBuf, u32)> {
        self.editor_request.take()
    }

    pub(super) fn submit_add(&mut self, add: &AddState) {
        let parent = match self.resolve_parent(&add.values[face_field::PARENT]) {
            Ok(parent) => parent,
            Err(error) => {
                self.event = format!("Add failed: {error}");
                return;
            }
        };
        let kind_fallback = pascal_case(&add.values[face_field::MODULE]);
        let kind = if add.values[face_field::KIND].trim().is_empty() {
            kind_fallback.as_str()
        } else {
            add.values[face_field::KIND].trim()
        };
        if add.values[face_field::MODULE].is_empty() {
            self.event = "Add failed: module name is required".to_owned();
            return;
        }
        let needs_registry = match add.values[face_field::NEEDS_REGISTRY].parse::<bool>() {
            Ok(value) => value,
            Err(_) => {
                self.event = "Add failed: needs registry must be true or false".to_owned();
                return;
            }
        };
        let face = nichlink_run_method::NewModuleFace {
            module: &add.values[face_field::MODULE],
            kind,
            preset: &add.values[face_field::PRESET],
            parts: &add.values[face_field::PARTS],
            name_zh: if add.values[face_field::NAME_ZH].trim().is_empty() {
                kind
            } else {
                &add.values[face_field::NAME_ZH]
            },
            name_en: if add.values[face_field::NAME_EN].trim().is_empty() {
                kind
            } else {
                &add.values[face_field::NAME_EN]
            },
            summary_zh: &add.values[face_field::SUMMARY_ZH],
            summary_en: &add.values[face_field::SUMMARY_EN],
            exports: &add.values[face_field::EXPORTS],
            stable_name: &add.values[face_field::STABLE_NAME],
            parent,
            needs_registry,
            getting_from_other_registry: &add.values[face_field::GETTING_FROM_OTHER_REGISTRY],
            registration_rule: &add.values[face_field::REGISTRY_RULE],
            admission: &add.values[face_field::ADMISSION],
            handle_traits: &add.values[face_field::HANDLE_TRAITS],
            handle_contracts: &add.values[face_field::HANDLE_CONTRACTS],
            part_traits: &add.values[face_field::PART_TRAITS],
            part_contracts: &add.values[face_field::PART_CONTRACTS],
            requires: &add.values[face_field::REQUIRES],
            provides: &add.values[face_field::PROVIDES],
            runtime_checks: &add.values[face_field::RUNTIME_CHECKS],
            flow: &add.values[face_field::FLOW],
            flow_provider: &add.values[face_field::FLOW_PROVIDER],
        };
        match with_authoring_context(|| {
            nichlink_run_method::add_module_from_face(&self.registry, &face)
        }) {
            Ok((change, info)) => {
                if let Err(error) = self.registry.register_snapshot_batch([info]) {
                    self.event = format!("Add failed:\n{error}");
                    return;
                }
                self.event = format!("{}; press r to reload", change.message);
                self.overlay = None;
            }
            Err(error) => self.event = format!("Add failed: {error}"),
        }
    }

    pub(super) fn submit_edit(&mut self, id: NodeId, edit: &AddState) {
        let needs_registry = match edit.values[face_field::NEEDS_REGISTRY].parse::<bool>() {
            Ok(value) => value,
            Err(_) => {
                self.event = "Edit failed: needs registry must be true or false".to_owned();
                return;
            }
        };
        let patch = nichlink_run_method::ModuleFacePatch {
            module: &edit.values[face_field::MODULE],
            kind: &edit.values[face_field::KIND],
            preset: &edit.values[face_field::PRESET],
            parts: &edit.values[face_field::PARTS],
            name_zh: &edit.values[face_field::NAME_ZH],
            name_en: &edit.values[face_field::NAME_EN],
            summary_zh: &edit.values[face_field::SUMMARY_ZH],
            summary_en: &edit.values[face_field::SUMMARY_EN],
            exports: &edit.values[face_field::EXPORTS],
            stable_name: &edit.values[face_field::STABLE_NAME],
            needs_registry,
            getting_from_other_registry: &edit.values[face_field::GETTING_FROM_OTHER_REGISTRY],
            registration_rule: &edit.values[face_field::REGISTRY_RULE],
            admission: &edit.values[face_field::ADMISSION],
            handle_traits: &edit.values[face_field::HANDLE_TRAITS],
            handle_contracts: &edit.values[face_field::HANDLE_CONTRACTS],
            part_traits: &edit.values[face_field::PART_TRAITS],
            part_contracts: &edit.values[face_field::PART_CONTRACTS],
            requires: &edit.values[face_field::REQUIRES],
            provides: &edit.values[face_field::PROVIDES],
            runtime_checks: &edit.values[face_field::RUNTIME_CHECKS],
            flow: &edit.values[face_field::FLOW],
            flow_provider: &edit.values[face_field::FLOW_PROVIDER],
        };
        match with_authoring_context(|| {
            nichlink_run_method::edit_module_face(&self.registry, id, &patch)
        }) {
            Ok(change) => {
                let message = change.message;
                let changed_source = change.source;
                // Keep the visible inspector in sync with the file we just
                // committed. The reload is deliberately after the atomic
                // write, so a failed parse keeps the previous healthy state.
                // 保存成功后立即刷新检视器；刷新失败时仍保留上一份健康快照。
                self.reload();
                if self.reload_error.is_none() {
                    if let Some(info) = self
                        .registry
                        .depth_first()
                        .into_iter()
                        .find(|info| source_path_for(&info.source.file) == changed_source)
                    {
                        self.selected = info.id;
                    }
                    self.event = format!("{message}; registration reloaded");
                }
                self.overlay = None;
            }
            Err(error) => self.event = format!("Edit failed: {error}"),
        }
    }

    pub(super) fn submit_plugin(&mut self, plugin: &PluginState) {
        let source = plugin.values[0].trim();
        let framework = plugin.values[1].trim();
        let package = plugin.values[2].trim();
        let version = plugin.values[3].trim();
        let crate_name = plugin.values[4].trim();
        let checksum = plugin.values[5].trim();
        let mode = plugin.values[6].trim();
        if !matches!(source, "official" | "user") {
            self.event = "Plugin failed: source must be official or user".to_owned();
            return;
        }
        if [framework, package, version, crate_name, checksum]
            .iter()
            .any(|value| value.is_empty())
        {
            self.event =
                "Plugin failed: framework, package, version, crate and checksum are required"
                    .to_owned();
            return;
        }
        if !matches!(mode, "extension" | "replacement") {
            self.event = "Plugin failed: mode must be extension or replacement".to_owned();
            return;
        }
        let package_root = package_root();
        let plugin_root = package_root.join(".nichlink/plugins");
        let lock_name = if source == "official" {
            "official.lock"
        } else {
            "user.lock"
        };
        let lock = plugin_root.join(lock_name);
        let record =
            format!("{source}|{framework}|{package}|{version}|{crate_name}|{checksum}|{mode}");
        let existing = std::fs::read_to_string(&lock).unwrap_or_default();
        if existing.lines().any(|line| line.trim() == record) {
            self.event = "Plugin already selected".to_owned();
            self.overlay = None;
            return;
        }
        let catalog = match PluginCatalog::parse(&existing) {
            Ok(catalog) => catalog,
            Err(error) => {
                self.event = format!("Plugin failed: invalid lock: {error}");
                return;
            }
        };
        let candidate = PluginRecord {
            source: if source == "official" {
                PluginSource::Official
            } else {
                PluginSource::User
            },
            framework: framework.to_owned(),
            package: package.to_owned(),
            version: version.to_owned(),
            crate_name: crate_name.to_owned(),
            checksum: checksum.to_owned(),
            mode: if mode == "replacement" {
                PluginMode::Replacement
            } else {
                PluginMode::Extension
            },
            signature: None,
            public_key_fingerprint: None,
            revocation_list: None,
        };
        if source == "official" && !catalog.contains(&candidate) {
            self.event =
                "Plugin failed: official package is not present in the trusted lock".to_owned();
            return;
        }
        if !crate_name
            .chars()
            .all(|character| character == '_' || character.is_ascii_alphanumeric())
        {
            self.event = "Plugin failed: crate must be a Rust identifier".to_owned();
            return;
        }
        let entry_name = if source == "official" {
            "official.rs"
        } else {
            "user.rs"
        };
        let entry = plugin_root.join(entry_name);
        let anchor = format!("\n#[allow(unused_imports)]\nuse {crate_name} as _;\n");
        let entry_text = std::fs::read_to_string(&entry).unwrap_or_default();
        if !entry_text.contains(&format!("use {crate_name} as _;"))
            && let Err(error) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&entry)
                .and_then(|mut file| std::io::Write::write_all(&mut file, anchor.as_bytes()))
        {
            self.event = format!("Plugin failed: cannot update {entry_name}: {error}");
            return;
        }
        let line = format!("{record}\n");
        if let Err(error) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&lock)
            .and_then(|mut file| std::io::Write::write_all(&mut file, line.as_bytes()))
        {
            self.event = format!("Plugin failed: cannot update {lock_name}: {error}");
            return;
        }
        self.event = format!("Plugin selected: {package} ({source}, {mode})");
        self.overlay = None;
    }

    fn resolve_parent(&self, value: &str) -> Result<NodeId, String> {
        let value = value.trim();
        if value.is_empty() || value.eq_ignore_ascii_case("root") {
            return Ok(self.registry.id());
        }
        if let Ok(id) = value.parse::<NodeId>() {
            return self
                .registry
                .registry(id)
                .map(|_| id)
                .ok_or_else(|| format!("`{id}` is not a Registry"));
        }
        let matches = self
            .registry
            .depth_first()
            .into_iter()
            .filter(|info| {
                info.needs_registry
                    && (info.registry_name.eq_ignore_ascii_case(value)
                        || info.kind.eq_ignore_ascii_case(value)
                        || self
                            .registry
                            .path_for(info.id)
                            .is_some_and(|path| path.eq_ignore_ascii_case(value)))
            })
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [info] => Ok(info.id),
            [] => Err(format!("parent `{value}` was not found")),
            _ => Err(format!(
                "parent `{value}` is ambiguous; use its node identity"
            )),
        }
    }
}
