//! The host's crate-shape declaration, read as **text** by the build (audit `M7`, P3.1).
//! 宿主的 crate 形状声明，由构建当**文本**读（审计 `M7`，P3.1）。
//!
//! The host writes `add_crates.rs` at its package root as ordinary Rust, so the compiler checks it
//! (a subtree that does not exist is an unresolved path, and the error points at that line). The
//! build cannot link the host, so it reads the same file and asks the kernel's one rule whether the
//! shape holds together — and then answers the question the declaration is *for*: which faces each
//! crate would own.
//! 宿主在包根把 `add_crates.rs` 写成普通 Rust，因此编译器会查它（不存在的子树就是解析不了的路径，而错误
//! 指到那一行）。构建无法链接宿主，于是它读同一个文件、用内核那唯一一条规则问"这份形状成立吗"，然后回答
//! 声明**真正为了**的那个问题：每个 crate 会拥有哪些面。
//!
//! The reader is **narrow and refuses**: it reads exactly the shape `Crate::named(…).at(&[…::SUBTREE])`
//! inside a `Shape { … }` literal, and anything else is a named error rather than a guess. A reader
//! that silently accepted a second spelling would decide a package's crate layout by accident.
//! 这个读取器**窄而会拒绝**：它只读 `Shape { … }` 字面量里 `Crate::named(…).at(&[…::SUBTREE])` 这一种形状，
//! 其它一律是点名错误而不是猜测。一个静默接受第二种拼写的读取器，会让一个包的 crate 布局被意外决定。

use std::fmt::Write as _;
use std::path::Path;

use nichlink_kernel::identity::NodeId;
use nichlink_kernel::lexicon;
use nichlink_kernel::registry_core::{DeclaredCrate, validate_shape};

use super::discovery_cache::write_if_changed;
use super::face_view::PruningRow;
use super::static_plan::source_module_path;

/// What one host declared, with the digest of the file it came from.
/// 一个宿主声明了什么，以及它来自哪个文件的摘要。
#[derive(Debug)]
pub(crate) struct ShapeDeclaration {
    /// The prefix every published crate name is built from.
    /// 每个发布包名所依据的前缀。
    pub(crate) package_prefix: String,
    /// One entry per declared crate: its name, and the `::`-separated module paths it claims.
    /// 每个已声明 crate 一项：它的名字，以及它认领的 `::` 分隔模块路径。
    pub(crate) crates: Vec<(String, Vec<String>)>,
    /// The identity derived from the declaration file's bytes, which is what the lock records.
    /// 从声明文件字节推导出的身份，也就是锁记录的东西。
    pub(crate) digest: String,
}

/// Read the shape `package_root/add_crates.rs` declares, or `None` when the host has none.
/// 读取 `package_root/add_crates.rs` 声明的形状；宿主没有这个文件时是 `None`。
///
/// No file means "this package is one crate", which is what every host was before the declaration
/// existed — so the absence is an answer, not a missing input.
/// 没有文件意为"这个包就是一个 crate"，也就是声明存在之前每个宿主的样子——因此"没有"是一个答案，而不是
/// 缺了输入。
pub(crate) fn read_shape_declaration(
    package_root: &Path,
) -> Result<Option<ShapeDeclaration>, String> {
    let path = package_root.join(lexicon::ADD_CRATES_FILE);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Ok(None);
    };
    let digest = NodeId::from_bytes(text.as_bytes()).to_string();
    let body = text
        .split("Shape {")
        .nth(1)
        .and_then(|rest| rest.split('}').next())
        .ok_or_else(|| unreadable(&path, "it has no `Shape { … }` literal"))?;
    let package_prefix = body
        .split("package_prefix")
        .nth(1)
        .and_then(|rest| rest.split('"').nth(1))
        .ok_or_else(|| {
            unreadable(
                &path,
                "it declares no `package_prefix: \"…\"`, which every published crate name is \
                 built from",
            )
        })?
        .to_owned();
    let mut crates = Vec::new();
    for chunk in body.split("Crate::named(").skip(1) {
        let name = chunk
            .split('"')
            .nth(1)
            .ok_or_else(|| unreadable(&path, "a `Crate::named(…)` carries no name string"))?
            .to_owned();
        let list = chunk
            .split(".at(&[")
            .nth(1)
            .and_then(|rest| rest.split(']').next())
            .ok_or_else(|| {
                unreadable(
                    &path,
                    &format!("`{name}` has no `.at(&[…])`, so it claims no subtree"),
                )
            })?;
        let mut subtrees = Vec::new();
        for entry in list
            .split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
        {
            // The declaration names a **marker**: `<path>::SUBTREE`. Anything else is a spelling
            // this reader does not know, and guessing what it meant is how a shape gets decided by
            // accident.
            // 声明点名的是一个**标记**：`<path>::SUBTREE`。别的拼写是本读取器不认识的，而猜它的意思正是
            // 形状被意外决定的方式。
            let module = entry
                .strip_suffix("::SUBTREE")
                .ok_or_else(|| {
                    unreadable(
                        &path,
                        &format!("`{name}` names `{entry}`, which is not a `…::SUBTREE` marker"),
                    )
                })?
                .trim()
                .trim_start_matches("crate::")
                .to_owned();
            subtrees.push(module);
        }
        crates.push((name, subtrees));
    }
    if crates.is_empty() {
        return Err(unreadable(
            &path,
            "it declares no `Crate::named(…)`, so it asks for no crate",
        ));
    }
    let modules: Vec<Vec<&str>> = crates
        .iter()
        .map(|(_, subtrees)| subtrees.iter().map(String::as_str).collect())
        .collect();
    let declared: Vec<DeclaredCrate<'_>> = crates
        .iter()
        .zip(&modules)
        .map(|((name, _), subtrees)| DeclaredCrate { name, subtrees })
        .collect();
    validate_shape(&package_prefix, &declared).map_err(|refusal| unreadable(&path, &refusal))?;
    Ok(Some(ShapeDeclaration {
        package_prefix,
        crates,
        digest,
    }))
}

/// Write the lock that records what the declaration asked for, and which faces each crate would own.
/// 写下锁：记录声明要求了什么，以及每个 crate 会拥有哪些面。
///
/// The lock is the shape's **history**: the declaration itself is what a person edits, and this file
/// is what a commit records — so "which commit was which shape" is a git question about one small
/// file rather than a re-derivation. Faces are resolved with the tree's own rule
/// (`source_module_path`), never by string surgery on file paths.
/// 锁是形状的**历史**：声明是人在编辑的东西，而这个文件是提交记录下来的东西——于是"哪个提交是哪种形状"是
/// 关于一个小文件的 git 问题，而不是一次重新推导。面的归属用树自己的规则（`source_module_path`）解析，
/// 绝不对文件路径做字符串手术。
pub(crate) fn write_shape_lock(
    out_dir: &Path,
    declaration: &ShapeDeclaration,
    rows: &[PruningRow],
) -> Result<(), String> {
    let mut owner: Vec<(String, Vec<String>)> = declaration
        .crates
        .iter()
        .map(|(name, _)| (name.clone(), Vec::new()))
        .collect();
    let mut unclaimed = 0usize;
    let mut seen: std::collections::BTreeSet<nichlink_kernel::identity::NodeId> =
        std::collections::BTreeSet::new();
    for row in rows {
        if !seen.insert(row.id) {
            continue;
        }
        let module = source_module_path(&row.source);
        let mut claimed = false;
        for (position, (_, subtrees)) in declaration.crates.iter().enumerate() {
            if subtrees
                .iter()
                .any(|subtree| module == *subtree || module.starts_with(&format!("{subtree}::")))
            {
                owner[position].1.push(row.source.clone());
                claimed = true;
                break;
            }
        }
        if !claimed {
            unclaimed += 1;
        }
    }
    let mut output = format!(
        "# add-crates\t{}\n# declaration\t{}\npackage_prefix\t{}\n",
        lexicon::ADD_CRATES_MARKER,
        declaration.digest,
        declaration.package_prefix
    );
    for (position, (name, subtrees)) in declaration.crates.iter().enumerate() {
        let mut faces = owner[position].1.clone();
        faces.sort();
        let face_set = NodeId::from_bytes(faces.join("\n").as_bytes()).to_string();
        writeln!(
            output,
            "crate\t{name}\tsubtrees={}\tfaces={}\tface_set={face_set}",
            subtrees.join(","),
            faces.len()
        )
        .unwrap();
    }
    writeln!(output, "host\tfaces={unclaimed}").unwrap();
    write_if_changed(&out_dir.join(lexicon::ADD_CRATES_LOCK_FILE), &output)
}

/// The pipeline's call: read the declaration, and publish the lock when the host declared one.
/// 管线的调用：读声明，宿主声明了形状时发布锁。
///
/// A host with no declaration is not an error: it is the one-crate package every host was before
/// this file existed, and the pipeline does nothing at all for it.
/// 没有声明的宿主不是错误：它就是这份文件存在之前每个宿主的样子——一个 crate 的包，而管线对它什么都不做。
pub(crate) fn check_shape_declaration(
    package_root: &Path,
    out_dir: &Path,
    rows: &[PruningRow],
) -> Result<(), String> {
    match read_shape_declaration(package_root)? {
        Some(declaration) => write_shape_lock(out_dir, &declaration, rows),
        None => Ok(()),
    }
}

/// The one refusal spelling for a declaration the build cannot read.
/// 构建读不了的声明，其唯一的拒绝拼法。
fn unreadable(path: &Path, why: &str) -> String {
    format!(
        "{}: {why}. The declaration is ordinary Rust, so the compiler checks the paths; this reader \
         only accepts `Shape {{ package_prefix: \"…\", crates: &[Crate::named(\"…\").at(&[\
         crate::…::SUBTREE])] }}`",
        path.display()
    )
}

#[cfg(test)]
#[path = "shape_decl_tests.rs"]
mod shape_decl_tests;
