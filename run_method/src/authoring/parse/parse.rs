//! Historical-path shim: `nichlink_run_method::authoring::parse`.
//! 历史路径 shim：`nichlink_run_method::authoring::parse`。
//!
//! Parsing and rendering helpers for authored registration faces. The pure
//! source-text transformations live in the kernel `authoring` module; this shim
//! keeps the historical path and re-implements the two filesystem-bound entry
//! points as thin wrappers. The word `shim` used to appear only on the third line,
//! so a reader who grepped this file's name against
//! `authoring/manifest/parse.rs` had to read the body to learn which one is
//! the implementation (audit `NAM-10`).
//! 注册面创作文件的解析与渲染辅助函数。纯源码文本变换位于 kernel 的 `authoring` 模块；
//! 本 shim 保留历史路径，并把两个绑定文件系统的入口重新实现为薄包装。`shim` 这个词过去只出现在
//! 第三行，因此把本文件名与 `authoring/manifest/parse.rs` 对照着 grep 的读者必须先读正文
//! 才知道哪一个是实现（审计 `NAM-10`）。

use std::fs;
use std::path::{Path, PathBuf};

pub use nichlink_kernel::authoring::parse::*;

use super::context::{normalized_path, source_root};

/// Read the parent's declared `kind` from its attached source file.
/// 从父注册面的附属源文件读取它声明的 `kind`。
pub(super) fn kind_from_source_path(source: &str) -> Option<String> {
    let path = source_root().join(source);
    let text = fs::read_to_string(path).ok()?;
    nichlink_kernel::authoring::parse::kind_from_source_text(&text)
}

/// The `source` a face file records, relative to the package's `src/`.
/// 注册面文件记录的 `source`，相对包的 `src/`。
///
/// A file under this package's `src/` is named by its path below it. A file
/// outside it has no honest package-relative form, so the only fold available is
/// the rule the toolchain already applies to a compiler-recorded path: everything
/// after the last `src/` component. A **list of directory names** used to live
/// here instead — this repository's own fixture names — and for any other host it
/// cut the path at whichever directory happened to share a name
/// (`/data/control/myrepo/src/foo/foo.rs` became `myrepo/src/foo/foo.rs`), which
/// then became the identity input and the manifest's `source` column.
/// 位于本包 `src/` 下的文件按它之下的路径命名。`src/` 之外的文件没有诚实
/// 的包内相对形式，因此唯一可用的折叠就是工具链已经对编译器记录的路径使用的那条规则：最后一个
/// `src/` 段之后的一切。这里过去放的是一份**目录名清单**——本仓自己的夹具名——对任何其它宿主，
/// 它会在任何一个恰好同名的目录处把路径截断（`/data/control/myrepo/src/foo/foo.rs` 变成
/// `myrepo/src/foo/foo.rs`），而那个值随后成了身份输入与清单的 `source` 列。
pub(super) fn source_path_from_file(path: &Path) -> String {
    if let Ok(relative) = path.strip_prefix(source_root()) {
        return normalized_path(relative);
    }
    after_last_src(path).unwrap_or_else(|| normalized_path(path))
}

/// Everything after the last `src/` component, when the path has one.
/// 路径中存在 `src/` 段时，最后一个 `src/` 之后的一切。
fn after_last_src(path: &Path) -> Option<String> {
    let components: Vec<_> = path.components().collect();
    let at = components
        .iter()
        .rposition(|component| component.as_os_str() == "src")?;
    let tail: PathBuf = components[at + 1..].iter().collect();
    if tail.as_os_str().is_empty() {
        return None;
    }
    Some(normalized_path(&tail))
}

#[cfg(test)]
#[path = "parse_tests.rs"]
mod parse_tests;

/// The rule a face's own registry-rule source declares, read strictly.
/// 某个注册面自己的注册规则源所声明的规则，按严格方式读取。
///
/// A face with no rule file has no structural requirement, so a missing file is still
/// `Ok("ANY")` — absence is not a malformed rule. A file that exists is read through
/// [`nichlink_kernel::authoring::parse::try_rule_syntax_from_text`], which locates the
/// `REGISTRATION_RULE` const's initializer through the AST: a clause inside a comment is
/// not part of the rule, and a rule the reader cannot recognize is refused with the face
/// file, the rule file, and the reason, instead of silently degrading to `ANY` or to a
/// rule nobody declared (audit `KRN-K-10`; the tolerant/`String` entry is kept only as
/// published API and this path no longer uses it).
/// 没有规则文件的注册面没有结构要求，因此文件缺失仍是 `Ok("ANY")`——缺席不是畸形规则。存在的文件经
/// [`nichlink_kernel::authoring::parse::try_rule_syntax_from_text`] 读取，它经 AST 定位
/// `REGISTRATION_RULE` 常量的初始器：注释里的子句不属于规则，而读取器认不出的规则会被拒绝，并带上
/// 注册面文件、规则文件与理由，而不是静默降级成 `ANY` 或某条没人声明的规则（审计 `KRN-K-10`；
/// 宽容的 `String` 入口只作为已发布 API 保留，本路径不再使用）。
pub(super) fn rule_syntax_for_source(path: &Path) -> Result<String, String> {
    let Some(parent) = path.parent() else {
        return Ok("ANY".to_owned());
    };
    let canonical = parent.join("registry_rule/registry_rule.rs");
    let legacy = parent.join("registry/rules/rules.rs");
    for candidate in [&canonical, &legacy] {
        let Ok(text) = fs::read_to_string(candidate) else {
            continue;
        };
        return nichlink_kernel::authoring::parse::try_rule_syntax_from_text(&text).map_err(
            |error| {
                format!(
                    "face `{}`: registration rule source `{}` was refused: {error}",
                    normalized_path(path),
                    normalized_path(candidate)
                )
            },
        );
    }
    Ok("ANY".to_owned())
}
