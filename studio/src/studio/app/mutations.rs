//! App mutations and editor handoff.
//! App 文件变更与编辑器交接。

use super::support::{package_root, select_project, with_authoring_context};
use super::*;

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
        if root.exists()
            && std::fs::read_dir(&root)
                .map(|mut entries| entries.next().is_some())
                .unwrap_or(true)
        {
            self.event = format!("New project failed: {} is not empty", root.display());
            return;
        }
        let studio_manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace = studio_manifest
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));
        // A source checkout can use sibling path dependencies. A binary
        // installed by `cargo install` has no workspace siblings, so projects
        // created from it must point back to the published Git packages.
        let (core_dependency, build_dependency) = if workspace.join("core").is_dir()
            && workspace.join("build").is_dir()
        {
            let core = workspace
                .join("core")
                .display()
                .to_string()
                .replace('\\', "\\\\");
            let build = workspace
                .join("build")
                .display()
                .to_string()
                .replace('\\', "\\\\");
            (
                format!("nichlink-core = {{ package = \"nichlink-core\", path = \"{core}\" }}"),
                format!("nichlink-build = {{ path = \"{build}\" }}"),
            )
        } else {
            let repository = "https://github.com/Nichtigott/nichlink";
            (
                format!(
                    "nichlink-core = {{ package = \"nichlink-core\", git = \"{repository}\", version = \"0.1.0\" }}"
                ),
                format!("nichlink-build = {{ git = \"{repository}\", version = \"0.1.0\" }}"),
            )
        };
        let crate_source = if kind == "library" {
            "src/lib.rs"
        } else {
            "src/main.rs"
        };
        let prelude = format!(
            "pub use nichlink_core::{{application, external_object}};\n\npub mod registry_core {{\n    pub use nichlink_core::*;\n}}\n\ninclude!(concat!(env!(\"OUT_DIR\"), \"/generated_lib.rs\"));\n{}",
            if kind == "library" {
                ""
            } else {
                "\nfn main() { println!(\"registered faces: {}\", builtin_static_plan().len()); }"
            }
        );
        let cargo = format!(
            "[package]\nname = \"{package}\"\nversion = \"0.1.0\"\nedition = \"2024\"\nbuild = \"build.rs\"\n\n[dependencies]\n{core_dependency}\n\n[build-dependencies]\n{build_dependency}\n"
        );
        let files = [
            ("Cargo.toml", cargo),
            (
                "build.rs",
                "fn main() { nichlink_build::run(); }\n".to_owned(),
            ),
            (crate_source, prelude),
        ];
        if let Err(error) = write_project_files(&root, &files) {
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

    pub fn take_editor_request(&mut self) -> Option<(PathBuf, u32)> {
        self.editor_request.take()
    }

    pub(super) fn submit_add(&mut self, add: &AddState) {
        let parent = match self.resolve_parent(&add.values[0]) {
            Ok(parent) => parent,
            Err(error) => {
                self.event = format!("Add failed: {error}");
                return;
            }
        };
        let kind_fallback = add.values[1]
            .split('_')
            .filter(|part| !part.is_empty())
            .map(|part| {
                let mut chars = part.chars();
                chars
                    .next()
                    .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
                    .unwrap_or_default()
            })
            .collect::<String>();
        let kind = if add.values[8].trim().is_empty() {
            kind_fallback.as_str()
        } else {
            add.values[8].trim()
        };
        if add.values[1].is_empty() {
            self.event = "Add failed: module name is required".to_owned();
            return;
        }
        let needs_registry = match add.values[2].parse::<bool>() {
            Ok(value) => value,
            Err(_) => {
                self.event = "Add failed: needs registry must be true or false".to_owned();
                return;
            }
        };
        let face = nichlink::NewModuleFace {
            module: &add.values[1],
            kind,
            preset: &add.values[13],
            parts: &add.values[6],
            name_zh: if add.values[9].trim().is_empty() {
                kind
            } else {
                &add.values[9]
            },
            name_en: if add.values[10].trim().is_empty() {
                kind
            } else {
                &add.values[10]
            },
            summary_zh: &add.values[11],
            summary_en: &add.values[12],
            params: &add.values[14],
            exports: &add.values[7],
            handle: if add.values[15].trim().is_empty() {
                kind
            } else {
                &add.values[15]
            },
            stable_name: &add.values[16],
            parent,
            needs_registry,
            registry_name: &add.values[3],
            getting_from_other_registry: &add.values[17],
            registry_rule_path: &add.values[18],
            registration_rule: &add.values[4],
            admission: &add.values[5],
            handle_traits: &add.values[19],
            handle_contracts: &add.values[20],
            part_traits: &add.values[21],
            part_contracts: &add.values[29],
            requires: &add.values[22],
            provides: &add.values[23],
            expected_output: &add.values[24],
            actual_output: &add.values[25],
            runtime_checks: &add.values[26],
            flow: &add.values[27],
            flow_provider: &add.values[28],
        };
        match with_authoring_context(|| nichlink::add_module_from_face(&self.registry, &face)) {
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
        let needs_registry = match edit.values[2].parse::<bool>() {
            Ok(value) => value,
            Err(_) => {
                self.event = "Edit failed: needs registry must be true or false".to_owned();
                return;
            }
        };
        let patch = nichlink::ModuleFacePatch {
            module: &edit.values[1],
            kind: &edit.values[8],
            preset: &edit.values[13],
            parts: &edit.values[6],
            name_zh: &edit.values[9],
            name_en: &edit.values[10],
            summary_zh: &edit.values[11],
            summary_en: &edit.values[12],
            params: &edit.values[14],
            exports: &edit.values[7],
            handle: &edit.values[15],
            stable_name: &edit.values[16],
            needs_registry,
            registry_name: &edit.values[3],
            getting_from_other_registry: &edit.values[17],
            registry_rule_path: &edit.values[18],
            registration_rule: &edit.values[4],
            admission: &edit.values[5],
            handle_traits: &edit.values[19],
            handle_contracts: &edit.values[20],
            part_traits: &edit.values[21],
            part_contracts: &edit.values[29],
            requires: &edit.values[22],
            provides: &edit.values[23],
            expected_output: &edit.values[24],
            actual_output: &edit.values[25],
            runtime_checks: &edit.values[26],
            flow: &edit.values[27],
            flow_provider: &edit.values[28],
        };
        match with_authoring_context(|| nichlink::edit_module_face(&self.registry, id, &patch)) {
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

fn write_project_files(root: &std::path::Path, files: &[(&str, String)]) -> Result<(), String> {
    std::fs::create_dir_all(root)
        .map_err(|error| format!("cannot create {}: {error}", root.display()))?;
    for (relative, content) in files {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
        }
        if let Err(error) = std::fs::write(&path, content) {
            return Err(format!("cannot write {}: {error}", path.display()));
        }
    }
    Ok(())
}
