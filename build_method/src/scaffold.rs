//! Project scaffolding for new NichLink host crates.
//! 新 NichLink 宿主 crate 的项目脚手架。

use std::fs;
use std::path::{Path, PathBuf};

use nichlink::registry_core::declaration::FACE_FIELD_ORDER;

const NICHLINK_REPOSITORY: &str = "https://github.com/Nichtigott/nichlink";

/// The shape each face field is written in, as a snippet template.
/// 每个注册面字段的书写形状，用 snippet 模板表示。
///
/// Every field has a fixed form — `parent: crate::…::NODE_ID,`, `exports:
/// ["…"],`, `handle_contracts: [crate::…],` — so completing a field name should
/// hand the author the whole line and leave the cursor where the value goes.
/// `$1`/`$2` are tab stops; the trailing comma belongs to the shape because a
/// declaration is a comma-separated list. The snippet layer is the only place
/// this can happen: rust-analyzer inserts a bare field name, and its value
/// completion (which does work for `crate::` paths) only runs once the colon is
/// there.
/// 每个字段都有固定写法——`parent: crate::…::NODE_ID,`、`exports: ["…"],`、
/// `handle_contracts: [crate::…],`——因此补全字段名时应当把整行交给作者，并把光标留在值位。
/// `$1`/`$2` 是跳转位；结尾逗号属于形状，因为声明本身是逗号分隔的列表。这件事只能由 snippet
/// 层完成：rust-analyzer 只插裸字段名，而它的值补全（对 `crate::` 路径确实可用）要等冒号
/// 写出来之后才会触发。
const FIELD_SHAPES: &[(&str, &str)] = &[
    ("source", "source: \"$1\","),
    ("kind", "kind: $1,"),
    ("preset", "preset: $1,"),
    ("parts", "parts: $1,"),
    ("name", "name: { zh: \"$1\", en: \"$2\" },"),
    ("summary", "summary: { zh: \"$1\", en: \"$2\" },"),
    ("params", "params: \"$1\","),
    ("exports", "exports: [\"$1\"],"),
    ("handle", "handle: $1,"),
    ("stable_name", "stable_name: \"$1\","),
    ("needs_registry", "needs_registry: $1,"),
    ("registry_name", "registry_name: $1,"),
    ("parent", "parent: $1,"),
    (
        "getting_from_other_registry",
        "getting_from_other_registry: $1,",
    ),
    ("registry_rule_path", "registry_rule_path: \"$1\","),
    ("registry_rule", "registry_rule: $1,"),
    ("admission", "admission: $1,"),
    ("handle_traits", "handle_traits: [\"$1\"],"),
    ("handle_contracts", "handle_contracts: [$1],"),
    ("part_traits", "part_traits: [\"$1\"],"),
    ("part_contracts", "part_contracts: [$1],"),
    ("requires", "requires: [\"$1\" => $2],"),
    ("provides", "provides: [\"$1\"],"),
    ("expected_output", "expected_output: \"$1\","),
    ("actual_output", "actual_output: \"$1\","),
    ("flow", "flow: $1,"),
    ("flow_provider", "flow_provider: $1,"),
    ("plugin", "plugin: $1,"),
    ("runtime_checks", "runtime_checks: [$1],"),
];

/// The template for one field, with a permissive fallback so a field added to
/// the kernel vocabulary still produces a usable snippet before its shape is
/// spelled out here (the test below keeps that from going unnoticed).
/// 某个字段的模板；若内核词表新增了字段而这里还没来得及写形状，则回退到一个可用形状
/// （下面的测试会让这种遗漏无法悄悄存在）。
fn field_shape(field: &str) -> String {
    FIELD_SHAPES
        .iter()
        .find(|(name, _)| *name == field)
        .map_or_else(|| format!("{field}: $1,"), |(_, shape)| (*shape).to_owned())
}

/// Escape a template for a JSON string in the VS Code file.
/// 把模板转义成 VS Code 文件里的 JSON 字符串。
fn json_escape(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Turn a template into LuaSnip nodes: literal text plus `i(n)` tab stops.
/// 把模板转成 LuaSnip 节点：字面文本加 `i(n)` 跳转位。
fn luasnip_body(shape: &str) -> String {
    let mut nodes = Vec::new();
    let mut literal = String::new();
    let mut chars = shape.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '$' && chars.peek().is_some_and(char::is_ascii_digit) {
            let mut digits = String::new();
            while chars.peek().is_some_and(char::is_ascii_digit) {
                digits.push(chars.next().expect("peeked digit"));
            }
            if !literal.is_empty() {
                nodes.push(format!("t(\"{}\")", lua_escape(&literal)));
                literal.clear();
            }
            nodes.push(format!("i({digits})"));
        } else {
            literal.push(character);
        }
    }
    if !literal.is_empty() {
        nodes.push(format!("t(\"{}\")", lua_escape(&literal)));
    }
    nodes.join(", ")
}

/// Escape literal text for a Lua string.
/// 把字面文本转义成 Lua 字符串。
fn lua_escape(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Which editor the snippet file is written for.
/// snippet 文件写给哪个编辑器。
///
/// The file is the last resort for the `: ` after a face field name: every
/// editor's field completion inserts the bare name (rust-analyzer does this for
/// plain structs too), and a language-server snippet does not fire inside a
/// macro call's token tree, so the colon only comes from the editor's own
/// snippet layer — which speaks a different format per editor.
/// 这份文件是注册面字段名后那个 `: ` 的最后办法：所有编辑器的字段补全都只插裸名字
/// （rust-analyzer 对普通结构体也一样），而语言服务器的 snippet 在宏调用的 token 树里
/// 不会触发，因此冒号只能来自编辑器自己的 snippet 层——而每个编辑器的格式不同。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Editor {
    /// VS Code (and forks that read project-scoped `.vscode` files).
    /// VS Code（以及读取项目级 `.vscode` 文件的衍生编辑器）。
    Vscode,
    /// Neovim with LuaSnip, as NvChad and most distributions configure it.
    /// 使用 LuaSnip 的 Neovim（NvChad 与多数发行版的默认装配）。
    Nvim,
    /// Neovim with blink.cmp's default (native `vim.snippet`) provider, which is
    /// what a Nix-packaged Neovim such as nvf ships. It reads VS Code-format
    /// JSON from `<config>/snippets/<filetype>/`.
    /// 使用 blink.cmp 默认（原生 `vim.snippet`）snippet 源的 Neovim，也就是 nvf 这类
    /// Nix 打包 Neovim 的默认装配；它读 `<config>/snippets/<filetype>/` 下的 VS Code
    /// 格式 JSON。
    Blink,
}

impl Editor {
    pub const ALL: [Editor; 3] = [Editor::Vscode, Editor::Nvim, Editor::Blink];

    /// The CLI spelling of an editor name.
    /// 编辑器名在命令行里的写法。
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "vscode" | "code" => Some(Editor::Vscode),
            "nvim" | "neovim" => Some(Editor::Nvim),
            "blink" | "blink.cmp" => Some(Editor::Blink),
            _ => None,
        }
    }

    /// The CLI spelling of this editor.
    /// 该编辑器在命令行里的写法。
    pub fn name(self) -> &'static str {
        match self {
            Editor::Vscode => "vscode",
            Editor::Nvim => "nvim",
            Editor::Blink => "blink",
        }
    }

    /// The file name inside the editor's snippet directory.
    /// 编辑器 snippet 目录里的文件名。
    pub fn file_name(self) -> &'static str {
        match self {
            Editor::Vscode => "nichlink-face.code-snippets",
            Editor::Nvim => "nichlink-face.lua",
            Editor::Blink => "nichlink-face.json",
        }
    }
}

/// Where VS Code reads project-scoped snippets from, relative to the package.
/// VS Code 从项目里读取项目级 snippet 的相对路径。
pub const SNIPPET_FILE: &str = ".vscode/nichlink-face.code-snippets";

/// Where Neovim's LuaSnip loader finds Rust snippets, relative to the nvim
/// config directory.
/// Neovim 的 LuaSnip 加载器寻找 Rust snippet 的相对路径（相对 nvim 配置目录）。
///
/// LuaSnip derives the filetype from the directory directly below `luasnippets`,
/// so `luasnippets/rust/` is Rust-only and leaves a hand-written
/// `luasnippets/rust.lua` untouched. NvChad already calls
/// `luasnip.loaders.from_lua.load()`, which scans the runtimepath for exactly
/// this directory, so no config change is needed to pick the file up.
/// LuaSnip 用 `luasnippets` 下一层目录名作为 filetype，因此 `luasnippets/rust/`
/// 只作用于 Rust，也不会碰到手写的 `luasnippets/rust.lua`。NvChad 启动时已经调用
/// `luasnip.loaders.from_lua.load()`，它正是扫描 runtimepath 里的这个目录，因此无需改配置
/// 就能生效。
pub const NVIM_SNIPPET_FILE: &str = "luasnippets/rust/nichlink-face.lua";

/// Where blink.cmp's default snippet provider looks for Rust snippets, relative
/// to the Neovim config directory.
/// blink.cmp 默认 snippet 源寻找 Rust snippet 的相对路径（相对 Neovim 配置目录）。
///
/// Its registry takes the directory name under `snippets/` as the filetype and
/// parses VS Code-format JSON, so the same file we write for VS Code works here.
/// 它把 `snippets/` 下的目录名当作 filetype 并解析 VS Code 格式的 JSON，因此给 VS Code
/// 写的那份文件在这里同样可用。
pub const BLINK_SNIPPET_FILE: &str = "snippets/rust/nichlink-face.json";

/// One snippet per face field: typing the field name offers an item that inserts
/// `name: ` and leaves the cursor after the colon.
/// 每个注册面字段一条 snippet：键入字段名时会出现一条插入 `name: ` 并把光标留在冒号后的候选。
///
/// The names come from the kernel's `FACE_FIELD_ORDER`, so a new field reaches
/// every project without any of these files drifting.
/// 字段名来自内核的 `FACE_FIELD_ORDER`，因此新增字段无需改这些文件就能到达每个项目。
pub fn editor_snippets(editor: Editor) -> String {
    match editor {
        Editor::Vscode => vscode_snippets(),
        Editor::Nvim => nvim_snippets(),
        Editor::Blink => vscode_snippets(),
    }
}

fn vscode_snippets() -> String {
    let mut output = String::from("{\n");
    for (index, field) in FACE_FIELD_ORDER.iter().enumerate() {
        if index > 0 {
            output.push_str(",\n");
        }
        output.push_str(&format!(
            "  \"{field}: \": {{\n    \"prefix\": [\"{field}: \"],\n    \"body\": [\"{body}\"],\n    \"scope\": \"rust\",\n    \"description\": \"插入 `{field}: ` 的定式并把光标停在值位 / insert the `{field}: ` shape and stop at its value\"\n  }}",
            body = json_escape(&field_shape(field))
        ));
    }
    output.push_str("\n}\n");
    output
}

fn nvim_snippets() -> String {
    let mut output = String::from(
        "-- Generated by `nichlink snippets --editor nvim`; edit the face fields, not this file.\n\
         -- 由 `nichlink snippets --editor nvim` 生成；要改字段请改注册面词表，不要改这个文件。\n\
         local ls = require(\"luasnip\")\n\
         local s = ls.snippet\n\
         local t = ls.text_node\n\
         local i = ls.insert_node\n\
         \n\
         return {\n",
    );
    for field in FACE_FIELD_ORDER {
        // The trigger *is* `field: ` on purpose: cmp_luasnip labels the item with
        // the trigger, so a bare field name would be indistinguishable from
        // rust-analyzer's own field item and would insert no colon. The body is
        // the field's whole shape, so one pick writes `field: <value>,` and drops
        // the cursor at the value — where rust-analyzer's own value completion
        // (`crate::…`, `crate::…::NODE_ID`) takes over.
        // trigger 故意就是 `field: `：cmp_luasnip 用 trigger 当候选标签，只写字段名会和
        // rust-analyzer 自带的字段项长得一模一样、也不会插冒号。body 是字段的完整形状，
        // 因此选中一次就写出 `field: <值>,` 并把光标放到值位——那里由 rust-analyzer 自己的
        // 值补全（`crate::…`、`crate::…::NODE_ID`）接手。
        output.push_str(&format!(
            "  s({{ trig = \"{field}: \", dscr = \"write the `{field}: ` shape and stop at its value / 写出 `{field}: ` 的定式并把光标停在值位\", priority = 2000 }}, {{ {} }}),\n",
            luasnip_body(&field_shape(field))
        ));
    }
    output.push_str("}\n");
    output
}

/// Write one editor's snippets, leaving the file alone when it already has the
/// current content.
/// 写入某个编辑器的 snippet；内容已经是最新时不改动文件。
pub fn write_editor_snippets(root: &Path, editor: Editor) -> Result<bool, String> {
    let relative = match editor {
        Editor::Vscode => SNIPPET_FILE,
        Editor::Nvim => NVIM_SNIPPET_FILE,
        Editor::Blink => BLINK_SNIPPET_FILE,
    };
    write_snippets_file(&root.join(relative), &editor_snippets(editor))
}

/// Write a snippet file, leaving it alone when it already has the current
/// content.
/// 写入 snippet 文件；内容已经是最新时不改动它。
pub fn write_snippets_file(path: &Path, content: &str) -> Result<bool, String> {
    if fs::read_to_string(path).ok().as_deref() == Some(content) {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    fs::write(path, content)
        .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    Ok(true)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectKind {
    Binary,
    Library,
}

impl ProjectKind {
    fn entry_file(self) -> &'static str {
        match self {
            ProjectKind::Binary => "src/main.rs",
            ProjectKind::Library => "src/lib.rs",
        }
    }
}

impl std::fmt::Display for ProjectKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            ProjectKind::Binary => "binary",
            ProjectKind::Library => "library",
        })
    }
}

/// Where a generated manifest sources the NichLink crates from.
/// 生成清单中 NichLink crate 的依赖来源。
#[derive(Clone, Debug)]
pub enum DependencySource {
    /// Path dependencies into a local NichLink checkout.
    /// 指向本地 NichLink checkout 的 path 依赖。
    Local { workspace: PathBuf },
    /// Git dependencies; Cargo can resolve the same version from crates.io
    /// once a matching release is published.
    /// Git 依赖；发布匹配版本后 Cargo 可将同一版本解析到 crates.io。
    Git { url: String },
}

/// Detect the dependency source for a running tool binary: path dependencies
/// when the binary lives inside a NichLink checkout's own target directory;
/// portable Git dependencies otherwise.
/// 根据运行中的工具二进制位置检测依赖来源：位于 NichLink checkout 自身
/// target 目录内时使用 path 依赖，否则使用可移植的 Git 依赖。
pub fn detected_source(tool_manifest_dir: &Path, current_exe: &Path) -> DependencySource {
    let workspace = tool_manifest_dir.parent().unwrap_or_else(|| Path::new("."));
    let runs_from_workspace = current_exe.starts_with(workspace.join("target"))
        && workspace.join("core").is_dir()
        && workspace.join("build_method").is_dir();
    if runs_from_workspace {
        DependencySource::Local {
            workspace: workspace.to_path_buf(),
        }
    } else {
        DependencySource::Git {
            url: NICHLINK_REPOSITORY.to_owned(),
        }
    }
}

pub fn dependency_specs(source: &DependencySource) -> (String, String) {
    match source {
        DependencySource::Local { workspace } => {
            let runtime = toml_path(&workspace.join("run_method"));
            let build = toml_path(&workspace.join("build_method"));
            (
                format!(
                    "nichlink-run-method = {{ package = \"nichlink-run-method\", path = \"{runtime}\" }}"
                ),
                format!("nichlink-build-method = {{ path = \"{build}\" }}"),
            )
        }
        DependencySource::Git { url } => (
            format!(
                "nichlink-run-method = {{ package = \"nichlink-run-method\", git = \"{url}\", branch = \"main\", version = \"0.1.0\" }}"
            ),
            format!(
                "nichlink-build-method = {{ git = \"{url}\", branch = \"main\", version = \"0.1.0\" }}"
            ),
        ),
    }
}

/// Render the manifest, build script, and source entry for a new project.
/// 渲染新项目的 manifest、构建脚本和源码入口。
pub fn project_files(
    package: &str,
    kind: ProjectKind,
    source: &DependencySource,
) -> Vec<(&'static str, String)> {
    let (runtime_dependency, build_dependency) = dependency_specs(source);
    let prelude = format!(
        "nichlink_run_method::host!();\n{}",
        if kind == ProjectKind::Library {
            ""
        } else {
            "\nfn main() { println!(\"registered faces: {}\", builtin_static_plan().len()); }"
        }
    );
    let cargo = format!(
        "[package]\nname = \"{package}\"\nversion = \"0.1.0\"\nedition = \"2024\"\nbuild = \"build.rs\"\n\n[dependencies]\n{runtime_dependency}\n\n[build-dependencies]\n{build_dependency}\n"
    );
    vec![
        ("Cargo.toml", cargo),
        (
            "build.rs",
            "fn main() { nichlink_build_method::run(); }\n".to_owned(),
        ),
        (kind.entry_file(), prelude),
    ]
}

/// Validate the package name and target directory, then write the project.
/// 校验包名与目标目录后写入项目文件。
pub fn create_project(
    root: &Path,
    package: &str,
    kind: ProjectKind,
    source: &DependencySource,
) -> Result<(), String> {
    if package.is_empty() {
        return Err("package name is required".to_owned());
    }
    if !package
        .chars()
        .all(|ch| ch == '_' || ch == '-' || ch.is_ascii_alphanumeric())
    {
        return Err("package must use letters, digits, '_' or '-'".to_owned());
    }
    if root.exists()
        && fs::read_dir(root)
            .map(|mut entries| entries.next().is_some())
            .unwrap_or(true)
    {
        return Err(format!("{} is not empty", root.display()));
    }
    write_project_files(root, &project_files(package, kind, source))?;
    // A new project gets the face-field snippets too, so `kind: ` is one pick
    // away from the first field the author writes.
    // 新项目同时得到注册面字段 snippet，因此作者写下的第一个字段就能一键得到 `kind: `。
    write_editor_snippets(root, Editor::Vscode)?;
    Ok(())
}

fn write_project_files(root: &Path, files: &[(&str, String)]) -> Result<(), String> {
    fs::create_dir_all(root)
        .map_err(|error| format!("cannot create {}: {error}", root.display()))?;
    for (relative, content) in files {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
        }
        if let Err(error) = fs::write(&path, content) {
            return Err(format!("cannot write {}: {error}", path.display()));
        }
    }
    Ok(())
}

fn toml_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "\\\\")
}

#[cfg(test)]
mod tests {
    use super::{
        DependencySource, Editor, FACE_FIELD_ORDER, FIELD_SHAPES, NVIM_SNIPPET_FILE, ProjectKind,
        SNIPPET_FILE, create_project, editor_snippets, field_shape, json_escape, luasnip_body,
        write_editor_snippets,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_directory(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "nichlink-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ))
    }

    /// Every declared field gets exactly one snippet, keyed and prefixed by its
    /// own name, whose body is that field's shape.
    /// 每个声明的字段恰好得到一条 snippet，键与前缀都是字段名，body 是该字段的定式形状。
    #[test]
    fn the_editor_snippets_cover_every_declared_field() {
        let snippets = editor_snippets(Editor::Vscode);
        assert_eq!(
            snippets.matches("\"prefix\"").count(),
            FACE_FIELD_ORDER.len(),
            "one snippet per field"
        );
        for field in FACE_FIELD_ORDER {
            assert!(snippets.contains(&format!("\"{field}: \": {{")), "{field}");
            assert!(
                snippets.contains(&format!("\"prefix\": [\"{field}: \"]")),
                "{field}"
            );
            assert!(
                snippets.contains(&format!(
                    "\"body\": [\"{}\"],",
                    json_escape(&field_shape(field))
                )),
                "{field}"
            );
        }
    }

    /// The shapes table covers the kernel vocabulary, and every shape is a
    /// complete declaration line: at least one tab stop and the trailing comma
    /// that a field list needs.
    /// 形状表覆盖内核词表，且每个形状都是完整的一行声明：至少一个跳转位，以及字段列表
    /// 需要的结尾逗号。
    #[test]
    fn every_declared_field_has_a_complete_shape() {
        for field in FACE_FIELD_ORDER {
            let shape = FIELD_SHAPES
                .iter()
                .find(|(name, _)| name == field)
                .map(|(_, shape)| *shape)
                .unwrap_or_else(|| panic!("no shape for `{field}`"));
            assert!(shape.contains('$'), "{field}: {shape}");
            assert!(shape.ends_with(','), "{field}: {shape}");
            assert!(shape.starts_with(field), "{field}: {shape}");
        }
        assert_eq!(FIELD_SHAPES.len(), FACE_FIELD_ORDER.len());
    }

    /// The Neovim file carries the same shapes in LuaSnip's Lua format, with the
    /// colon in the trigger so the menu shows what it will write.
    /// Neovim 文件用 LuaSnip 的 Lua 格式承载同样的形状；trigger 带冒号，菜单里因此看得见
    /// 它将写出什么。
    #[test]
    fn the_luasnip_snippets_cover_every_declared_field() {
        let snippets = editor_snippets(Editor::Nvim);
        assert_eq!(
            snippets.matches("trig = ").count(),
            FACE_FIELD_ORDER.len(),
            "one snippet per field"
        );
        for field in FACE_FIELD_ORDER {
            assert!(
                snippets.contains(&format!("trig = \"{field}: \"")),
                "{field}"
            );
            assert!(
                snippets.contains(&format!("{{ {} }}", luasnip_body(&field_shape(field)))),
                "{field}"
            );
        }
        assert!(
            snippets.contains("i(1)"),
            "the cursor lands at the value position"
        );
    }

    /// The Neovim file lives in the rust-only directory LuaSnip already scans,
    /// and the CLI spellings map to the two supported editors.
    /// Neovim 文件位于 LuaSnip 本就会扫描的 rust 专属目录，命令行写法映射到两种受支持编辑器。
    #[test]
    fn the_nvim_file_is_rust_only_and_the_names_round_trip() {
        assert!(NVIM_SNIPPET_FILE.starts_with("luasnippets/rust/"));
        assert!(NVIM_SNIPPET_FILE.ends_with(Editor::Nvim.file_name()));
        assert_eq!(Editor::parse("vscode").map(Editor::name), Some("vscode"));
        assert_eq!(Editor::parse("code").map(Editor::name), Some("vscode"));
        assert_eq!(Editor::parse("nvim").map(Editor::name), Some("nvim"));
        assert_eq!(Editor::parse("neovim").map(Editor::name), Some("nvim"));
        assert_eq!(Editor::parse("emacs"), None);
    }

    /// Injecting twice leaves the second call with nothing to write.
    /// 注入两次时第二次无事可做。
    #[test]
    fn injecting_the_editor_snippets_is_idempotent() {
        let root = temporary_directory("snippets");
        assert!(write_editor_snippets(&root, Editor::Vscode).expect("first injection"));
        let path = root.join(SNIPPET_FILE);
        let first = fs::read_to_string(&path).expect("snippet file");
        assert_eq!(first, editor_snippets(Editor::Vscode));
        assert!(!write_editor_snippets(&root, Editor::Vscode).expect("second injection"));
        assert_eq!(fs::read_to_string(&path).expect("snippet file"), first);
        fs::remove_dir_all(root).expect("cleanup");
    }

    /// A scaffolded project already carries the file, so the author never runs
    /// the command by hand.
    /// 脚手架生成的项目自带该文件，作者无需手动执行命令。
    #[test]
    fn a_new_project_carries_the_editor_snippets() {
        let root = temporary_directory("project-snippets").join("app");
        create_project(
            &root,
            "app",
            ProjectKind::Binary,
            &DependencySource::Git {
                url: "https://example.invalid/nichlink".to_owned(),
            },
        )
        .expect("project");
        let snippets = fs::read_to_string(root.join(SNIPPET_FILE)).expect("snippet file");
        assert_eq!(snippets, editor_snippets(Editor::Vscode));
        fs::remove_dir_all(root.parent().expect("project parent")).expect("cleanup");
    }
}
