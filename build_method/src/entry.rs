//! Host entry resolution shared by the build step and the authoring query.
//! 构建步骤与创作查询共用的宿主入口解析。
//!
//! The entry is the file that calls `host!()`: it owns the generated module
//! tree and the declared graft plan. The scope policy and the graft view both
//! resolve it through [`resolve_host_entry`], so they can never disagree about
//! which file is the host.
//! 入口是调用 `host!()` 的文件：它拥有生成的模块树与声明的 graft 计划。作用域策略
//! 与嫁接视图都经由 [`resolve_host_entry`] 解析入口，因此两者对“哪个文件是宿主”
//! 绝不会产生分歧。
//!
//! `NICH_LINK_ENTRY` is read exactly once per build, in
//! [`host_entry_from_environment`], and the resolved value is threaded to both
//! readers. The variable used to reach pruning only: the graft view resolved the
//! entry on its own and never saw the variable, so with the variable set the
//! release pruned one file's slots while the generated cut table described
//! another file's — the runtime then held a table the release had not kept, or
//! silently lost a slot declared only in the configured file.
//! `NICH_LINK_ENTRY` 每次构建只读一次（在 [`host_entry_from_environment`] 中），
//! 解析结果再传给两个读取者。该变量过去只作用于剪枝：嫁接视图自行解析入口、从不看
//! 这个变量，于是在设置变量时，发布态剪掉的是一个文件的槽位，而生成的切口表描述的
//! 是另一个文件——运行期于是拿着发布态并未保留的表，或静默丢掉只在被指定文件里声明
//! 的槽位。

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use nichlink::lexicon;

use super::diagnostics::{BuildDiagnostic, BuildDiagnostics};
use super::node::{Node, relative_display};
use super::registry_syntax::application_entries;

/// Pick the ordinary Cargo entry when no `application!` declaration exists.
/// 没有 `application!` 声明时，按 Cargo 约定选择默认入口。
pub(crate) fn default_entry_source(src: &Path) -> PathBuf {
    let candidates = [src.join("main.rs"), src.join("lib.rs")];
    // Cargo prefers `main.rs`, but a lib+bin host calls `host!()` in whichever
    // file owns the generated tree — often `lib.rs`, with `main.rs` left as a
    // stub. An `application!` declaration is not the only thing that can live
    // there: the declared graft plan does too, and picking the stub silently
    // dropped it. The entry is the file that calls `host!()`; Cargo's order only
    // breaks the tie.
    // Cargo 偏好 `main.rs`，但库+二进制宿主会在拥有生成树的那个文件里调用
    // `host!()`——常见情形是 `lib.rs`，而 `main.rs` 只是个空壳。那里不只可能放
    // `application!` 声明，声明的 graft 计划也在其中，选中空壳会把它静默丢掉。
    // 入口应当是调用 `host!()` 的文件；Cargo 的顺序只用来打破平局。
    if let Some(entry) = candidates.iter().find(|path| calls_host(path)) {
        return entry.clone();
    }
    let main = candidates[0].clone();
    if main.is_file() {
        return main;
    }
    let lib = candidates[1].clone();
    if lib.is_file() {
        return lib;
    }
    // Keep the existing missing-entry diagnostic and conservative fallback.
    // 保留原有 missing-entry 诊断，并继续使用保守的全树回退。
    main
}

/// Whether a source file declares this crate as a host.
/// 源文件是否把本 crate 声明为宿主。
///
/// The decision is lexical, not line-based: the kernel's source scanner masks
/// comments and string literals, so the `host!()` a doc comment documents never
/// counts, while the real invocation at the crate root does. `host!()` is a
/// macro invocation and the kernel's `body_calls` scanner deliberately skips
/// macro names, so the bang is folded to a space before the scan: that keeps the
/// scanner's masking — the part this needs — and turns the invocation into the
/// call shape it matches.
/// 判断是词法层面的，而不是逐行匹配：内核源码扫描器会屏蔽注释与字符串字面量，
/// 因此文档注释里提到的 `host!()` 不算数，而 crate 根上的真实调用算数。
/// `host!()` 是宏调用，内核的 `body_calls` 扫描器刻意跳过宏名，因此扫描前把感叹号
/// 折成空格：这样既保留了这里需要的屏蔽语义，又把该调用变成扫描器能匹配的调用形状。
fn calls_host(path: &Path) -> bool {
    fs::read_to_string(path).is_ok_and(|source| host_macro_call(&source))
}

/// Whether `source` contains a real `host!()` invocation outside comments and
/// string literals.
/// `source` 是否在注释与字符串字面量之外包含真实的 `host!()` 调用。
fn host_macro_call(source: &str) -> bool {
    // Only the bang right after `host` is rewritten; comments and string
    // literals are still masked by the kernel scanner itself.
    // 只改写紧跟在 `host` 之后的感叹号；注释与字符串字面量仍由内核扫描器自己屏蔽。
    let probe = source.replace("host!", "host ");
    nichlink::source::body_calls(&probe, "host")
}

/// The file declaring `application!(entry = …)`, or `None` when none does.
/// 声明 `application!(entry = …)` 的文件；没有声明时返回 `None`。
///
/// Every refusal is a diagnostic rather than a panic. The build still fails — the
/// diagnostic is rendered into the generated tree like the rest — but it fails
/// with a document a reader and `check --json` can both consume, and the pipeline
/// survives long enough to report every other problem in the same run.
/// 每一次拒绝都是诊断而不是 panic。构建仍然失败——诊断会像其余诊断一样被渲染进生成树
/// ——但它带着读者与 `check --json` 都能消费的文档失败，而且管线能活到把同一次运行里的
/// 其他问题一并报完。
pub(crate) fn application_entry_source(
    src: &Path,
    nodes: &[Node],
    errors: &mut BuildDiagnostics,
) -> Option<PathBuf> {
    fn visit(
        src: &Path,
        nodes: &[Node],
        errors: &mut BuildDiagnostics,
        declarations: &mut Vec<(PathBuf, String, usize, usize)>,
    ) {
        for node in nodes {
            if let Some(file) = &node.file {
                match fs::read_to_string(file) {
                    Ok(source) => match application_entries(&source) {
                        Ok(entries) => {
                            for (path, location) in entries {
                                declarations.push((
                                    file.clone(),
                                    path,
                                    location.line,
                                    location.column,
                                ));
                            }
                        }
                        Err(error) => errors.push(
                            BuildDiagnostic::new(
                                "entry",
                                format!("invalid `application!` declaration: {}", error.message),
                            )
                            .at(
                                relative_display(src, file),
                                error.location.as_ref().map_or(0, |location| location.line),
                            ),
                        ),
                    },
                    Err(error) => errors.push(
                        BuildDiagnostic::new(
                            "entry",
                            format!(
                                "failed to read `{}` while looking for an `application!` entry: {error}",
                                file.display()
                            ),
                        )
                        .at(relative_display(src, file), 0),
                    ),
                }
            }
            visit(src, &node.children, errors, declarations);
        }
    }
    let mut declarations = Vec::new();
    visit(src, nodes, errors, &mut declarations);
    match declarations.as_slice() {
        [] => None,
        [(file, path, line, _)] => {
            if !path.starts_with("crate::") {
                errors.push(rejected_entry(
                    src,
                    file,
                    *line,
                    path,
                    "must start with `crate::`",
                ));
                return None;
            }
            if !entry_path_exists(src, path) {
                errors.push(rejected_entry(
                    src,
                    file,
                    *line,
                    path,
                    "does not resolve to a source module under the package root",
                ));
                return None;
            }
            Some(file.clone())
        }
        many => {
            let details = many
                .iter()
                .map(|(file, path, line, column)| {
                    format!("{path} at {}:{line}:{column}", file.display())
                })
                .collect::<Vec<_>>()
                .join(", ");
            let (file, _, line, _) = &many[0];
            errors.push(
                BuildDiagnostic::new(
                    "entry",
                    format!("multiple `application!` entry declarations found: {details}"),
                )
                .at(relative_display(src, file), *line),
            );
            None
        }
    }
}

/// One `application!` entry the build refused, with the file that declared it.
/// 构建拒绝的一条 `application!` 入口，以及声明它的文件。
fn rejected_entry(src: &Path, file: &Path, line: usize, path: &str, why: &str) -> BuildDiagnostic {
    BuildDiagnostic::new("entry", format!("`application!` entry `{path}` {why}"))
        .at(relative_display(src, file), line)
}

/// The build's host entry, together with the rule that decided it.
/// 构建的宿主入口，以及决定它的那条规则。
///
/// The variant is not decoration: it carries whether an unreadable entry is a
/// build error. An entry the host named — by environment variable or by
/// `application!(entry = …)` — must be readable, because the build is about to
/// derive pruning and the runtime cut table from it. Only Cargo's convention may
/// legitimately name a file that does not exist (a package with neither
/// `src/main.rs` nor `src/lib.rs`), and that case stays "no entry", not an error.
/// 变体不是装饰：它携带“入口读不出来算不算构建错误”。宿主自己指定的入口——环境变量
/// 或 `application!(entry = …)`——必须可读，因为构建即将据此推导剪枝与运行期切口表。
/// 只有 Cargo 约定可以合法地指向一个不存在的文件（既无 `src/main.rs` 也无
/// `src/lib.rs` 的包），那种情况仍然是“没有入口”，而不是错误。
#[derive(Clone, Debug)]
pub(crate) enum HostEntry {
    /// Named by `NICH_LINK_ENTRY`; a relative path resolves against the package root.
    /// 由 `NICH_LINK_ENTRY` 指定；相对路径相对包根解析。
    Configured(PathBuf),
    /// Named by the file declaring `application!(entry = …)`.
    /// 由声明 `application!(entry = …)` 的文件指定。
    Declared(PathBuf),
    /// Cargo's `main.rs`/`lib.rs` convention, which may name no file at all.
    /// Cargo 的 `main.rs`/`lib.rs` 约定，可能指不到任何文件。
    Convention(PathBuf),
}

impl HostEntry {
    /// The file the build reads for the generated plan and the cut table.
    /// 构建为生成计划与切口表读取的文件。
    pub(crate) fn path(&self) -> &Path {
        match self {
            Self::Configured(path) | Self::Declared(path) | Self::Convention(path) => path,
        }
    }

    /// Whether an entry that cannot be read here is a build error.
    /// 此处读不出来的入口是否算构建错误。
    pub(crate) fn is_required(&self) -> bool {
        !matches!(self, Self::Convention(_))
    }
}

/// Resolve the host entry from the environment, once per build.
/// 从环境解析宿主入口，每次构建一次。
///
/// The environment is read here and nowhere else in the build, so the two
/// readers downstream cannot observe different values for it.
/// 环境只在这里读取，构建中别无他处读取它，因此下游两个读取者不可能看到不同的值。
pub(crate) fn host_entry_from_environment(
    src: &Path,
    nodes: &[Node],
    errors: &mut BuildDiagnostics,
) -> HostEntry {
    resolve_host_entry_reporting(
        src,
        nodes,
        env::var_os(lexicon::ENTRY_ENV).map(PathBuf::from),
        errors,
    )
}

/// Resolve a host entry with the findings thrown away.
/// 解析宿主入口，并丢弃发现结果。
///
/// Test-only: production code goes through the reporting form, and keeping the
/// plain wrapper is what lets the existing tests stay one line shorter than the
/// code they pin.
/// 仅测试使用：生产代码走带报告的形式，而保留这个纯包装正是让既有测试比它们钉住的代码
/// 少一行的原因。
#[cfg(test)]
pub(crate) fn resolve_host_entry(
    src: &Path,
    nodes: &[Node],
    configured: Option<PathBuf>,
) -> HostEntry {
    resolve_host_entry_reporting(src, nodes, configured, &mut BuildDiagnostics::default())
}

/// Resolve the host entry from an explicit `NICH_LINK_ENTRY` value.
/// 从明确的 `NICH_LINK_ENTRY` 取值解析宿主入口。
///
/// Taking the value as a parameter instead of reading the environment keeps the
/// resolution testable: a test can hand it the same value both readers receive
/// without mutating the process environment, which `#[test]` threads share.
/// 把取值作为参数传入而不是就地读环境，使解析可被测试：测试可以把两个读取者收到的
/// 同一个值直接传进来，而不必改动进程环境（`#[test]` 线程是共享它的）。
///
/// Why a configured entry that is not a file fails the build.
/// The obvious implementation — ignore the variable here, or fall back to
/// `main.rs` — is exactly the bug this function exists to remove: pruning would
/// follow the variable (reading nothing, hence keeping the whole tree) while the
/// cut table kept describing `main.rs`, so the runtime held a table the release
/// had not kept, or silently lost every slot declared only in the configured
/// file. A `main.rs` fallback would reproduce that split; an empty table would be
/// worse than today, because a typo in the variable would turn a working build
/// into a runtime full of unknown targets. The boundary: only a *set* variable
/// that names no file fails the build. An unset variable keeps Cargo's convention,
/// including the legitimate "neither `main.rs` nor `lib.rs` exists" package,
/// which stays a non-error. The behaviour is pinned by
/// `a_configured_entry_that_is_not_a_file_is_a_diagnostic` and
/// `a_configured_entry_drives_the_scope_and_the_cut_table`.
/// 为什么被指定却指不到文件的入口要让构建失败。显而易见的做法——这里干脆
/// 不看变量，或回退到 `main.rs`——正是本函数要消除的那个 bug：剪枝会跟随变量（读不到
/// 内容，于是保留整棵树），而切口表仍在描述 `main.rs`，运行期于是拿着发布态并未保留
/// 的表，或静默丢掉只在被指定文件里声明的每一个槽位。回退到 `main.rs` 会重现这种
/// 分裂；返回空表则比今天更糟，因为变量里一个错字就会把本来能跑的构建变成运行期满是
/// 未知目标。边界是：只有变量**被设置**且路径不可用时才是错误。变量未设置时仍按
/// Cargo 约定，包括合法的“既无 `main.rs` 也无 `lib.rs`”的包，那仍然不是错误。该行为
/// 由 `a_configured_entry_that_is_not_a_file_is_a_diagnostic` 与
/// `a_configured_entry_drives_the_scope_and_the_cut_table` 钉住。
/// Resolve the host entry and report what it had to refuse.
/// 解析宿主入口，并报告它不得不拒绝的东西。
pub(crate) fn resolve_host_entry_reporting(
    src: &Path,
    nodes: &[Node],
    configured: Option<PathBuf>,
    errors: &mut BuildDiagnostics,
) -> HostEntry {
    if let Some(path) = configured {
        let path = if path.is_absolute() {
            path
        } else {
            // The package root, not `src`: a configured `src/main.rs` and a
            // configured `main.rs` must both be able to name the same file.
            // 基准是包根而不是 `src`：`src/main.rs` 与 `main.rs` 两种写法都必须能
            // 指到同一个文件。
            src.parent().unwrap_or(src).join(path)
        };
        if !path.is_file() {
            errors.push(
                BuildDiagnostic::new(
                    "entry",
                    format!(
                        "{} names `{}`, which is not a file",
                        lexicon::ENTRY_ENV,
                        path.display()
                    ),
                )
                .at(relative_display(src, &path), 0),
            );
            // The diagnostic above already fails the build, so which entry the
            // pipeline carries matters only for the diagnostics still to come:
            // the convention entry keeps them flowing instead of aborting here.
            // 上面的诊断已经让构建失败，因此管线带着哪个入口只影响后面还要产出的诊断：
            // 约定入口让它们继续流出来，而不是在这里中止。
            return HostEntry::Convention(default_entry_source(src));
        }
        return HostEntry::Configured(path);
    }
    if let Some(file) = application_entry_source(src, nodes, errors) {
        return HostEntry::Declared(file);
    }
    HostEntry::Convention(default_entry_source(src))
}

/// Resolve the host entry the build step reads graft declarations from.
/// 解析构建步骤读取 graft 声明的宿主入口。
///
/// Resolution order matches the build's `resolve_host_entry`: `NICH_LINK_ENTRY`,
/// then a file declaring `application!(entry = …)`, then the file that calls
/// `host!()`, then Cargo's `main.rs`/`lib.rs`. Two differences are deliberate: an
/// authoring surface walks the package's own Rust sources when looking for the
/// declaration (the build walks the discovered registration folders), and a
/// source tree it cannot represent is an `Err`, never a panic, because an
/// authoring surface has to stay alive to say so.
/// 解析顺序与构建的 `resolve_host_entry` 一致：`NICH_LINK_ENTRY`、声明
/// `application!(entry = …)` 的文件、调用 `host!()` 的文件、最后按 Cargo 的
/// `main.rs`/`lib.rs` 约定。两处差异是有意的：创作界面找声明时遍历包自己的 Rust
/// 源码（构建遍历已发现的注册目录），且无法表示的源码树返回 `Err` 而不是 panic——
/// 创作界面必须活着把问题说出来。
pub fn host_entry_source(root: &Path) -> Result<PathBuf, String> {
    let src = root.join("src");
    if !src.is_dir() {
        return Err(format!("no source tree at {}", src.display()));
    }
    if let Some(configured) = env::var_os(lexicon::ENTRY_ENV) {
        let path = PathBuf::from(configured);
        let path = if path.is_absolute() {
            path
        } else {
            root.join(path)
        };
        if !path.is_file() {
            return Err(format!(
                "{} names `{}`, which is not a file",
                lexicon::ENTRY_ENV,
                path.display()
            ));
        }
        return Ok(path);
    }
    if let Some(entry) = declaring_application_entry(&src)? {
        return Ok(entry);
    }
    let entry = default_entry_source(&src);
    if !entry.is_file() {
        return Err(format!(
            "no host entry at {}; expected a source file that calls `host!()`",
            entry.display()
        ));
    }
    Ok(entry)
}

/// The file declaring `application!(entry = …)`, if any.
/// 声明 `application!(entry = …)` 的文件（若有）。
///
/// The build walks the discovered registration folders, which is what a host
/// normally has. The authoring query walks the package's own Rust sources
/// instead, so a declaration in `src/lib.rs` or `src/main.rs` is seen too; a
/// tree the build would reject is reported rather than panicked on.
/// 构建遍历已发现的注册目录（宿主通常如此）；创作查询改为遍历包自己的 Rust 源码，
/// 因此写在 `src/lib.rs` 或 `src/main.rs` 里的声明也能看见；构建会拒绝的树在这里
/// 只被上报，不会 panic。
fn declaring_application_entry(src: &Path) -> Result<Option<PathBuf>, String> {
    let mut files = Vec::new();
    collect_rust_sources(src, &mut files)?;
    files.sort();
    let mut declarations = Vec::new();
    for file in files {
        let source = fs::read_to_string(&file)
            .map_err(|error| format!("cannot read {}: {error}", file.display()))?;
        for (path, location) in application_entries(&source).map_err(|error| {
            format!(
                "invalid application! declaration in {}: {error}",
                file.display()
            )
        })? {
            if !path.starts_with("crate::") {
                return Err(format!(
                    "application! entry `{path}` in {} must start with `crate::`",
                    file.display()
                ));
            }
            if !entry_path_exists(src, &path) {
                return Err(format!(
                    "application! entry `{path}` in {} does not resolve to a source module under `{}`",
                    file.display(),
                    src.display()
                ));
            }
            declarations.push((file.clone(), path, location.line, location.column));
        }
    }
    match declarations.as_slice() {
        [] => Ok(None),
        [(file, _, _, _)] => Ok(Some(file.clone())),
        many => {
            let details = many
                .iter()
                .map(|(file, path, line, column)| {
                    format!("{path} at {}:{line}:{column}", file.display())
                })
                .collect::<Vec<_>>()
                .join(", ");
            Err(format!(
                "multiple application! entry declarations found: {details}"
            ))
        }
    }
}

/// Every `.rs` file under `directory`, following the crate's own layout.
/// `directory` 下的每个 `.rs` 文件，遵循 crate 自己的布局。
/// The filesystem facts the kernel's source walk asks this surface for.
/// 内核源码遍历向本执行面索取的文件系统事实。
struct StdSourceTree;

impl nichlink::source::SourceTree for StdSourceTree {
    fn is_directory(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn entries(&self, path: &Path) -> Result<Vec<PathBuf>, String> {
        fs::read_dir(path)
            .map_err(|error| format!("cannot scan {}: {error}", path.display()))?
            .map(|entry| {
                entry
                    .map(|entry| entry.path())
                    .map_err(|error| format!("cannot scan {}: {error}", path.display()))
            })
            .collect()
    }

    fn read_text(&self, path: &Path) -> Result<String, String> {
        fs::read_to_string(path).map_err(|error| format!("cannot read {}: {error}", path.display()))
    }
}

fn collect_rust_sources(directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    nichlink::source::collect_rust_sources(
        &StdSourceTree,
        directory,
        nichlink::source::SourceWalk::EVERYTHING,
        |_, _| nichlink::source::Keep::Yes,
        files,
    )
}

fn entry_path_exists(src: &Path, path: &str) -> bool {
    let mut segments = path
        .strip_prefix("crate::")
        .unwrap_or_default()
        .split("::")
        .filter(|segment| !segment.is_empty());
    let Some(first) = segments.next() else {
        return false;
    };
    let module = src.join(first);
    module.with_extension("rs").is_file()
        || module.join("mod.rs").is_file()
        || src.join("bin").join(first).with_extension("rs").is_file()
        || (first == "main" && src.join("lib.rs").is_file())
}

pub(crate) fn path_mentions_module(path: &str, module: &str) -> bool {
    let path = path
        .strip_prefix("crate::")
        .or_else(|| path.strip_prefix("nichlink::"))
        .unwrap_or(path);
    path == module || path.starts_with(&format!("{module}::"))
}

#[cfg(test)]
#[path = "entry_tests.rs"]
mod entry_tests;
