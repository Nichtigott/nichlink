//! NichLink command-line interface.
//! NichLink 命令行界面。

use std::path::{Path, PathBuf};

use nichlink_build_method::scaffold::{self, DependencySource, ProjectKind};

const USAGE: &str = "\
nichlink — NichLink command-line interface

USAGE:
    nichlink new <name> [--lib] [--path <workspace> | --git <url>]
    nichlink check [path]
    nichlink build [path] [cargo options]
    nichlink snippets [path] [--editor vscode|nvim|blink|auto] [--stdout]
    nichlink studio
    nichlink mcp

COMMANDS:
    new       Create a NichLink host project in ./<name>
    check     Run the registration discovery and validation pass without compiling
    build     Validate the registration tree, then run cargo build
    snippets  Inject the face-field editor snippets (VS Code project file, or
              the LuaSnip file Neovim loads)
    studio    Launch the Studio TUI for the current project
    mcp       Run the read-only MCP stdio bridge

OPTIONS:
    --lib             Create a library project instead of a binary
    --path <dir>      Source nichlink-core/build from a local checkout
    --git <url>       Source nichlink-core/build from a Git repository
    --editor <name>   Editor to write snippets for: vscode (default), nvim
                      (LuaSnip), blink (blink.cmp) or auto (every editor
                      installed on this machine, in its user-level location;
                      fuzzy-matching engines need to be named explicitly)
    --stdout          Print the snippets instead of writing them (any editor)
";

pub fn main() -> Result<(), String> {
    run(std::env::args())
}

/// Dispatch one command from an argv-style iterator (the program name is
/// consumed and ignored). Shared by the `nichlink` and `cargo-nichlink`
/// binaries.
/// 从 argv 风格的迭代器分发一条命令（程序名会被消耗忽略）。`nichlink` 与
/// `cargo-nichlink` 两个二进制共用。
pub fn run(argv: impl IntoIterator<Item = String>) -> Result<(), String> {
    let mut args = argv.into_iter().skip(1);
    match args.next().as_deref() {
        None | Some("--help") | Some("-h") | Some("help") => {
            print!("{USAGE}");
            Ok(())
        }
        Some("new") => new(&mut args),
        Some("check") => check(&mut args),
        Some("build") => build(&mut args),
        Some("snippets") => snippets(&mut args),
        Some("studio") => nichlink_studio::launch().map_err(|error| error.to_string()),
        Some("mcp") => {
            nichlink_mcp::run();
            Ok(())
        }
        Some(other) => Err(format!("unknown command '{other}' (see --help)")),
    }
}

fn new(args: &mut impl Iterator<Item = String>) -> Result<(), String> {
    let mut name: Option<String> = None;
    let mut lib = false;
    let mut path: Option<PathBuf> = None;
    let mut git: Option<String> = None;
    let mut args = args.by_ref().peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--lib" => lib = true,
            "--path" => {
                let value = args.next().ok_or("--path requires a directory")?;
                path =
                    Some(std::fs::canonicalize(&value).unwrap_or_else(|_| PathBuf::from(&value)));
            }
            "--git" => git = Some(args.next().ok_or("--git requires a URL")?),
            _ if name.is_none() => name = Some(arg),
            _ => return Err(format!("unexpected argument '{arg}'")),
        }
    }
    let name = name.ok_or("new requires a package name")?;
    let source = match (path, git) {
        (Some(workspace), _) => DependencySource::Local { workspace },
        (None, Some(url)) => DependencySource::Git { url },
        (None, None) => scaffold::detected_source(
            Path::new(env!("CARGO_MANIFEST_DIR")),
            &std::env::current_exe()
                .map_err(|error| format!("cannot locate current executable: {error}"))?,
        ),
    };
    let root = std::env::current_dir()
        .map_err(|error| format!("cannot read current directory: {error}"))?
        .join(&name);
    let kind = if lib {
        ProjectKind::Library
    } else {
        ProjectKind::Binary
    };
    scaffold::create_project(&root, &name, kind, &source)?;
    println!(
        "created {} project at {}",
        if lib { "library" } else { "binary" },
        root.display()
    );
    Ok(())
}

fn check(args: &mut impl Iterator<Item = String>) -> Result<(), String> {
    let directory = args.next().unwrap_or_else(|| ".".to_owned());
    if args.next().is_some() {
        return Err("check accepts at most one path".to_owned());
    }
    let package = registration_check(&directory)?;
    println!("nichlink check: ok ({package})");
    Ok(())
}

/// Validate the registration tree first; only then invoke `cargo build`
/// with the remaining arguments passed through verbatim. The path, when
/// given, must come before any cargo option.
/// 先校验注册树；通过后再调用 `cargo build`，其余参数原样透传。
/// 路径如给出，必须位于任何 cargo 选项之前。
fn build(args: &mut impl Iterator<Item = String>) -> Result<(), String> {
    let all: Vec<String> = args.by_ref().collect();
    let (directory, cargo_args) = split_build_args(&all);
    let directory = directory.unwrap_or_else(|| ".".to_owned());
    let package = registration_check(&directory)?;
    println!("nichlink build: registration ok ({package})");
    let manifest = std::fs::canonicalize(&directory)
        .map_err(|error| format!("cannot resolve {directory}: {error}"))?;
    let status = std::process::Command::new("cargo")
        .arg("build")
        .args(&cargo_args)
        .current_dir(&manifest)
        .status()
        .map_err(|error| format!("cannot start cargo build: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("cargo build failed ({status})"))
    }
}

/// Inject the face-field editor snippets into a project or an editor config.
/// 把注册面字段的编辑器 snippet 注入项目或编辑器配置。
///
/// An editor's field completion inserts the bare name, and a language-server
/// snippet does not fire inside a macro call's token tree, so the `: ` after a
/// field name has to come from the editor's own snippet layer — and that layer
/// speaks a different format per editor. VS Code reads a project-scoped file
/// next to the sources; Neovim's LuaSnip loader scans its config for
/// `luasnippets/<filetype>/`, so nothing has to be wired up there either.
/// 编辑器的字段补全插入的是裸名字，而语言服务器的 snippet 在宏调用的 token 树里不会
/// 触发，因此字段名后的 `: ` 只能由编辑器自己的 snippet 层提供——而每个编辑器的格式不同。
/// VS Code 读源码旁的项目级文件；Neovim 的 LuaSnip 加载器会扫描配置目录里的
/// `luasnippets/<filetype>/`，因此那边同样无需额外接线。
fn snippets(args: &mut impl Iterator<Item = String>) -> Result<(), String> {
    let mut directory: Option<String> = None;
    let mut editor = scaffold::Editor::Vscode;
    let mut auto = false;
    let mut stdout = false;
    let args = args.by_ref();
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--stdout" => stdout = true,
            "--editor" => {
                let name = args.next().ok_or("--editor requires a name")?;
                if name == "auto" {
                    auto = true;
                    continue;
                }
                let accepted = scaffold::Editor::ALL
                    .iter()
                    .map(|editor| editor.name())
                    .collect::<Vec<_>>()
                    .join(", ");
                editor = scaffold::Editor::parse(&name).ok_or_else(|| {
                    format!("unknown editor '{name}' (accepted: {accepted}, auto)")
                })?;
            }
            _ if arg.starts_with('-') => return Err(format!("unexpected argument '{arg}'")),
            _ if directory.is_none() => directory = Some(arg),
            _ => return Err("snippets accepts at most one path".to_owned()),
        }
    }
    if auto {
        if stdout {
            return Err(
                "--stdout prints one editor's file; pick one with --editor <name>".to_owned(),
            );
        }
        if directory.is_some() {
            return Err(
                "auto writes each editor's user-level location; use --editor <name> for a \
                 project file"
                    .to_owned(),
            );
        }
        return install_everywhere();
    }
    if stdout {
        print!("{}", scaffold::editor_snippets(editor));
        return Ok(());
    }
    let path = match editor {
        scaffold::Editor::Vscode => {
            let directory = directory.unwrap_or_else(|| ".".to_owned());
            std::fs::canonicalize(&directory)
                .map_err(|error| format!("cannot resolve {directory}: {error}"))?
                .join(scaffold::SNIPPET_FILE)
        }
        scaffold::Editor::Nvim | scaffold::Editor::Blink => {
            if directory.is_some() {
                return Err(
                    "nvim snippets live in the editor config; use --stdout to write them \
                     somewhere else"
                        .to_owned(),
                );
            }
            let file = match editor {
                scaffold::Editor::Blink => scaffold::BLINK_SNIPPET_FILE,
                _ => scaffold::NVIM_SNIPPET_FILE,
            };
            nvim_config_dir().join(file)
        }
    };
    let written = scaffold::write_snippets_file(&path, &scaffold::editor_snippets(editor))?;
    println!(
        "nichlink snippets: {} {}",
        if written { "wrote" } else { "kept" },
        path.display()
    );
    Ok(())
}

/// Install the snippets for every editor this machine has, each in that editor's
/// **user-level** location, so one run covers every project from then on.
/// 给本机装有的每个编辑器安装 snippet，都写进各编辑器的**用户级**位置，因此跑一次就覆盖
/// 之后的所有项目。
///
/// The files are small, generated from one vocabulary and written only when their
/// content changes, so this is safe to run again (a post-checkout hook, a shell
/// alias, or by hand after `FACE_FIELD_ORDER` gains a field).
/// 这些文件很小、由同一份词表生成、内容不变时不写盘，因此可以反复运行（检出钩子、shell
/// 别名，或在 `FACE_FIELD_ORDER` 新增字段后手动跑一次）。
fn install_everywhere() -> Result<(), String> {
    let config_home = config_home();
    let targets = snippet_targets(&config_home, false);
    if targets.is_empty() && !has_nvim_config(&config_home) {
        return Err(
            "no VS Code, VSCodium, Cursor or Neovim configuration found; write the file \
             yourself with --editor <name> --stdout"
                .to_owned(),
        );
    }
    for (editor, path) in &targets {
        let written = scaffold::write_snippets_file(path, &scaffold::editor_snippets(*editor))?;
        println!(
            "nichlink snippets: {} {} ({})",
            if written { "wrote" } else { "kept" },
            path.display(),
            editor.name()
        );
    }
    if has_nvim_config(&config_home) {
        println!(
            "nichlink snippets: skipped Neovim — its snippet engines match fuzzily, so these \
             triggers would also appear at value positions; run `--editor blink` or \
             `--editor nvim` if you want them anyway"
        );
    }
    Ok(())
}

/// Every editor snippet location this machine has.
/// 本机具有的每个编辑器 snippet 位置。
///
/// Only editors that match a snippet **prefix by prefix** are installed by
/// default. An engine that matches fuzzily — blink.cmp, LuaSnip through
/// nvim-cmp — also offers a field trigger at a **value** position, so
/// `crc` there matches `handle_contracts: ` and the field shapes pollute the
/// value chain (measured; see `docs/audit-2026-09-21.md`). Those engines are
/// skipped unless the caller asks for them by name, because the sentence a field
/// shape really needs — `crate::…::NODE_ID`, the face's own marker type — is
/// value completion, which rust-analyzer already provides.
/// 默认只安装**按前缀**匹配 snippet 的编辑器。模糊匹配的引擎（blink.cmp、经 nvim-cmp 的
/// LuaSnip）在**值位**也会命中字段 trigger，于是值位敲 `crc` 会命中 `handle_contracts: `、
/// 字段定式污染值补全链（已实测，见 `docs/audit-2026-09-21.md`）。这类引擎只有在调用方显式
/// 指名时才安装——因为字段定式真正需要的语句（`crate::…::NODE_ID`、本面自己的标记类型）
/// 属于值补全，rust-analyzer 本来就提供。
///
/// VS Code and its forks merge every `*.code-snippets` in their user snippets
/// directory, so one file there is enough and covers all projects.
/// VS Code 及其衍生编辑器会合并用户 snippet 目录里所有 `*.code-snippets`，因此那边一个
/// 文件就够，而且覆盖所有项目。
fn snippet_targets(config_home: &Path, fuzzy_engines: bool) -> Vec<(scaffold::Editor, PathBuf)> {
    let mut targets = Vec::new();
    if fuzzy_engines {
        let nvim = config_home.join("nvim");
        if nvim.is_dir() {
            targets.push((
                scaffold::Editor::Blink,
                nvim.join(scaffold::BLINK_SNIPPET_FILE),
            ));
            targets.push((
                scaffold::Editor::Nvim,
                nvim.join(scaffold::NVIM_SNIPPET_FILE),
            ));
        }
    }
    for fork in ["Code", "VSCodium", "Cursor"] {
        let root = config_home.join(fork);
        if root.is_dir() {
            targets.push((
                scaffold::Editor::Vscode,
                root.join("User")
                    .join("snippets")
                    .join("nichlink-face.code-snippets"),
            ));
        }
    }
    targets
}

/// Whether a Neovim configuration exists, so the auto installer can say why it
/// skipped it.
/// 是否存在 Neovim 配置，好让自动安装说明它为什么跳过。
fn has_nvim_config(config_home: &Path) -> bool {
    config_home.join("nvim").is_dir()
}

/// The user configuration directory: `$XDG_CONFIG_HOME`, or `~/.config`.
/// 用户配置目录：`$XDG_CONFIG_HOME`，否则 `~/.config`。
fn config_home() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"))
}

/// The Neovim configuration directory: `$XDG_CONFIG_HOME` (or `~/.config`)
/// followed by `$NVIM_APPNAME` (or `nvim`), which is how Neovim itself resolves
/// it.
/// Neovim 配置目录：`$XDG_CONFIG_HOME`（或 `~/.config`）加上 `$NVIM_APPNAME`
/// （或 `nvim`），与 Neovim 自身的解析方式一致。
fn nvim_config_dir() -> PathBuf {
    let app_name = std::env::var("NVIM_APPNAME").unwrap_or_else(|_| "nvim".to_owned());
    nvim_config_dir_in(&config_home(), &app_name)
}

/// Resolve the Neovim config directory from explicit values, so the rule can be
/// pinned by a test.
/// 用显式值解析 Neovim 配置目录，使这条规则可以被测试钉住。
fn nvim_config_dir_in(config_home: &Path, app_name: &str) -> PathBuf {
    config_home.join(if app_name.is_empty() {
        "nvim"
    } else {
        app_name
    })
}

/// Split build args into an optional leading path and the remaining cargo
/// options. The path must come first; anything after it is passed verbatim.
/// 将 build 参数拆成可选的首个路径和其余 cargo 选项。路径必须在前，
/// 之后的所有内容原样透传。
fn split_build_args(args: &[String]) -> (Option<String>, Vec<String>) {
    match args.first() {
        Some(first) if !first.starts_with('-') => (Some(first.clone()), args[1..].to_vec()),
        _ => (None, args.to_vec()),
    }
}

/// Run registration discovery and validation for the host project at
/// `directory`, returning the package name on success.
/// 为 `directory` 处的宿主项目运行注册发现与校验，成功时返回包名。
fn registration_check(directory: &str) -> Result<String, String> {
    let manifest = std::fs::canonicalize(directory)
        .map_err(|error| format!("cannot resolve {directory}: {error}"))?;
    if !manifest.join("Cargo.toml").is_file() {
        return Err(format!("{} has no Cargo.toml", manifest.display()));
    }
    let package = package_name(&manifest)?;
    let out_dir = manifest.join("target/nichlink/out");
    nichlink_build_method::run_for(&manifest, &out_dir, &package)?;
    Ok(package)
}

/// Read the package name from `[package] name = "..."`. Falls back to the
/// manifest directory name for workspace-inherited names.
/// 从 `[package] name = "..."` 读取包名；workspace 继承名回退到目录名。
fn package_name(manifest: &Path) -> Result<String, String> {
    let content = std::fs::read_to_string(manifest.join("Cargo.toml"))
        .map_err(|error| format!("cannot read manifest: {error}"))?;
    for line in content.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("name = \"")
            && let Some(name) = rest.split('"').next().filter(|name| !name.is_empty())
        {
            return Ok(name.to_owned());
        }
    }
    manifest
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| "cannot determine package name".to_owned())
}

#[cfg(test)]
mod tests {
    use super::{has_nvim_config, nvim_config_dir_in, run, snippet_targets, split_build_args};
    use std::path::{Path, PathBuf};

    #[test]
    fn help_and_missing_command_succeed() {
        assert!(run(["nichlink".to_owned(), "--help".to_owned()]).is_ok());
        assert!(run(["nichlink".to_owned()]).is_ok());
    }

    #[test]
    fn unknown_command_is_an_error() {
        let result = run(["nichlink".to_owned(), "bogus".to_owned()]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown command 'bogus'"));
    }

    #[test]
    fn new_requires_a_package_name() {
        let result = run(["nichlink".to_owned(), "new".to_owned()]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("package name"));
    }

    #[test]
    fn build_args_split_leading_path_from_cargo_options() {
        let owned = |items: &[&str]| {
            items
                .iter()
                .map(|item| item.to_string())
                .collect::<Vec<_>>()
        };
        let (path, rest) = split_build_args(&owned(&["app", "--release"]));
        assert_eq!(path.as_deref(), Some("app"));
        assert_eq!(rest, owned(&["--release"]));
        let (path, rest) = split_build_args(&owned(&["--release"]));
        assert_eq!(path, None);
        assert_eq!(rest, owned(&["--release"]));
        let (path, rest) = split_build_args(&[]);
        assert_eq!(path, None);
        assert!(rest.is_empty());
    }

    /// The Neovim config directory follows Neovim's own rule, and the nvim
    /// snippets land in the rust-only directory LuaSnip scans.
    /// Neovim 配置目录遵循 Neovim 自身的规则，nvim snippet 落在 LuaSnip 扫描的 rust
    /// 专属目录里。
    #[test]
    fn nvim_snippet_path_follows_the_editor_config() {
        assert_eq!(
            nvim_config_dir_in(Path::new("/home/x/.config"), "nvim"),
            PathBuf::from("/home/x/.config/nvim")
        );
        assert_eq!(
            nvim_config_dir_in(Path::new("/home/x/.config"), "nvchad"),
            PathBuf::from("/home/x/.config/nvchad")
        );
        assert_eq!(
            nvim_config_dir_in(Path::new("/home/x/.config"), ""),
            PathBuf::from("/home/x/.config/nvim")
        );
        let target = nvim_config_dir_in(Path::new("/tmp/cfg"), "nvim")
            .join(nichlink_build_method::scaffold::NVIM_SNIPPET_FILE);
        assert_eq!(
            target,
            PathBuf::from("/tmp/cfg/nvim/luasnippets/rust/nichlink-face.lua")
        );
        let blink = nvim_config_dir_in(Path::new("/tmp/cfg"), "nvim")
            .join(nichlink_build_method::scaffold::BLINK_SNIPPET_FILE);
        assert_eq!(
            blink,
            PathBuf::from("/tmp/cfg/nvim/snippets/rust/nichlink-face.json")
        );
    }

    /// The auto installer finds each editor's user-level snippet location, and
    /// writes both Neovim files because it cannot know which engine is loaded.
    /// 自动安装会找到每个编辑器的用户级 snippet 位置；Neovim 两个文件都写，因为无法得知
    /// 它加载的是哪个引擎。
    #[test]
    fn auto_targets_follow_the_installed_editors() {
        let root = std::env::temp_dir().join(format!(
            "nichlink-auto-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        for dir in ["nvim", "Code", "VSCodium"] {
            std::fs::create_dir_all(root.join(dir)).expect("editor config");
        }
        let names = |fuzzy: bool| {
            snippet_targets(&root, fuzzy)
                .iter()
                .map(|(editor, path)| {
                    format!(
                        "{} {}",
                        editor.name(),
                        path.strip_prefix(&root)
                            .expect("under root")
                            .display()
                            .to_string()
                            .replace('\\', "/")
                    )
                })
                .collect::<Vec<_>>()
        };
        // By default only the engines that match a snippet prefix as a prefix.
        assert_eq!(
            names(false),
            [
                "vscode Code/User/snippets/nichlink-face.code-snippets",
                "vscode VSCodium/User/snippets/nichlink-face.code-snippets",
            ]
        );
        assert!(has_nvim_config(&root));
        // Fuzzy-matching engines are installed only when asked for by name.
        assert_eq!(
            names(true),
            [
                "blink nvim/snippets/rust/nichlink-face.json",
                "nvim nvim/luasnippets/rust/nichlink-face.lua",
                "vscode Code/User/snippets/nichlink-face.code-snippets",
                "vscode VSCodium/User/snippets/nichlink-face.code-snippets",
            ]
        );
        // No editor at all is reported rather than guessed.
        assert!(snippet_targets(&root.join("missing"), true).is_empty());
        assert!(!has_nvim_config(&root.join("missing")));
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    /// An unknown editor, or a path next to a config-dir editor, is refused
    /// before anything is written.
    /// 未知编辑器，或给"写配置目录"的编辑器附带路径，都在写盘之前拒绝。
    #[test]
    fn snippets_refuses_an_unknown_editor_and_a_stray_path() {
        let unknown = run([
            "nichlink".to_owned(),
            "snippets".to_owned(),
            "--editor".to_owned(),
            "emacs".to_owned(),
        ])
        .expect_err("unknown editor");
        assert!(unknown.contains("unknown editor 'emacs'"), "{unknown}");
        assert!(unknown.contains("vscode, nvim, blink"), "{unknown}");

        let stray = run([
            "nichlink".to_owned(),
            "snippets".to_owned(),
            "--editor".to_owned(),
            "nvim".to_owned(),
            "/tmp/somewhere".to_owned(),
        ])
        .expect_err("stray path");
        assert!(stray.contains("editor config"), "{stray}");
    }

    /// The command injects a parseable editor file covering the whole kernel
    /// vocabulary, and refuses arguments it does not understand.
    /// 该命令注入一个可解析、覆盖整个内核词表的编辑器文件，并拒绝看不懂的参数。
    #[test]
    fn snippets_injects_the_editor_file() {
        let root = std::env::temp_dir().join(format!(
            "nichlink-cli-snippets-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).expect("temporary directory");
        run([
            "nichlink".to_owned(),
            "snippets".to_owned(),
            root.display().to_string(),
        ])
        .expect("snippets command");

        let path = root.join(nichlink_build_method::scaffold::SNIPPET_FILE);
        let text = std::fs::read_to_string(&path).expect("editor file");
        let parsed: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
        let snippets = parsed.as_object().expect("an object of snippets");
        assert_eq!(
            snippets.len(),
            nichlink::registry_core::declaration::FACE_FIELD_ORDER.len()
        );
        let kind = snippets.get("kind: ").expect("the kind snippet");
        assert_eq!(kind["prefix"][0], "kind: ");
        assert_eq!(kind["body"][0], "kind: $1,");
        assert_eq!(kind["scope"], "rust");

        assert!(
            run([
                "nichlink".to_owned(),
                "snippets".to_owned(),
                "--bogus".to_owned()
            ])
            .is_err()
        );
        assert!(
            run([
                "nichlink".to_owned(),
                "snippets".to_owned(),
                "a".to_owned(),
                "b".to_owned()
            ])
            .is_err()
        );
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
