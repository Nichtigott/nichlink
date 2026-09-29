//! Build audit manifests.
//! 构建审计清单。
//!
//! The function column is a *declaration* list: it comes from the kernel's lexical
//! scanner (`nichlink_kernel::source::function_symbols`), which masks comments, strings and
//! macro text before it looks for `fn`. A symbol is therefore `module::name` — the
//! scanner reports declarations without ownership, so a method carries no `impl`
//! type prefix. This file used to hold a second, line-based scanner that matched
//! `fn ` anywhere on a line and guessed the owner from the previous `impl ` line,
//! which nested `impl` blocks and closures polluted; that column asked the same
//! question the kernel already answers, and removing it is what closes the
//! duplication. No reader inside this tree consumes the file — `grep -rn
//! function_manifest` finds only the writers — so the change is visible to whoever
//! starts trusting it, which is exactly why its format is stated here.
//! 函数列的语义是**声明**清单：它来自内核词法扫描器
//! （`nichlink_kernel::source::function_symbols`），该扫描器在寻找 `fn` 之前会屏蔽注释、字符串与宏文本。
//! 因此符号是 `module::name`——扫描器报告的是不带归属的声明，方法因此不带 `impl` 类型前缀。
//! 本文件过去带第二套按行的扫描器，它在行的任意位置匹配 `fn `，并用上一个 `impl ` 行猜归属；
//! 嵌套 impl 与闭包都会污染它。那一列问的正是内核已回答的同一个问题，删掉它正是收掉这份重复。
//! 本树内没有任何读者使用这份文件——`grep -rn function_manifest` 只找到写入侧——因此对将来开始
//! 信任它的人而言这次变化是可见的，这也正是这里写明格式的原因。
//!
//! The pruning column is documented where it is produced: see [`tracked_symbol`] and
//! the three item shapes in [`PruningItem`]. Its rows carry four more columns since
//! 2026-09-29 — `path`, `kind`, `registry_name`, `parent` — holding what the
//! **declaration** spelled (`-` where it named none), which is why a reader can now
//! match a face by logical path without reading the sources.
//! 修剪列的约定写在产出它的地方：见 [`tracked_symbol`] 与 [`PruningItem`] 里的三种条目形状。
//! 自 2026-09-29 起它的行多带四列——`path`、`kind`、`registry_name`、`parent`——内容是**声明**拼出的
//! 词（未声明处为 `-`），因此读者现在可以不读源码就按逻辑路径匹配一个面。
//! 修剪列的约定写在产出它的地方：见 [`tracked_symbol`] 与 [`PruningItem`] 里的三种条目形状。

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use super::Node;
use super::registry_identity::NodeId;
use super::registry_syntax::{FaceSyntax, GraftSyntax};
use super::static_plan::source_module_path;
use super::{SourceScope, collect_faces, parsed_face, relative_display, write_if_changed};

pub(crate) fn write_pruning_manifest(
    src: &Path,
    nodes: &[Node],
    out_dir: &Path,
) -> Result<(), String> {
    let mut rows = Vec::new();
    visit_pruning_symbols(src, nodes, &mut rows);
    write_face_rows(out_dir.join("pruning_manifest.tsv"), rows)
}

/// The face facts the pruning manifest publishes beside each symbol.
/// 剪枝清单在每个符号旁发布的面事实。
///
/// These are the words the **declaration** used, not resolved identities: `parent` is the Rust
/// path the face spelled, so a reader that needs the parent's `NodeId` still resolves it. The
/// three that are already typed (`kind`, `registry_name`, `path`) are published because every
/// table-driven answer used to re-read the sources to get them.
/// 这些是**声明**用的那些词，不是解析后的身份：`parent` 是该面拼出的 Rust 路径，因此需要父级
/// `NodeId` 的读者仍然要解析它。已经带类型的那三个（`kind`、`registry_name`、`path`）之所以发布，
/// 是因为每一份查表面的答案过去都要为此重读源码。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct FaceColumns {
    path: String,
    kind: String,
    registry_name: String,
    parent: String,
}

impl FaceColumns {
    /// The four facts as this face spells them, with `-` where it names none.
    /// 本面拼出的四项事实，未声明的写作 `-`。
    fn declared(face: &FaceSyntax, kind: &str) -> Self {
        let spelled = |value: Option<String>| value.unwrap_or_else(|| "-".to_owned());
        Self {
            path: spelled(face.string("path")),
            kind: kind.to_owned(),
            registry_name: spelled(face.string("registry_name")),
            parent: spelled(face.path("parent")),
        }
    }
}

pub(crate) fn write_function_manifest(
    src: &Path,
    nodes: &[Node],
    out_dir: &Path,
) -> Result<(), String> {
    let mut rows = Vec::new();
    visit_function_symbols(src, nodes, &mut rows);
    write_rows(out_dir.join("function_manifest.tsv"), rows)
}

pub(crate) fn write_source_scope_manifest(
    src: &Path,
    nodes: &[Node],
    scope: &SourceScope,
    out_dir: &Path,
) -> Result<(), String> {
    let Some(selected) = &scope.roots else {
        write_if_changed(
            &out_dir.join("source_scope.tsv"),
            &format!(
                "# mode\t{}\n# result\tall\n# selected\tall\n# reason\t{}\n",
                if scope.reason == "scope-all" {
                    "explicit"
                } else {
                    "auto"
                },
                scope.reason
            ),
        )?;
        return Ok(());
    };
    let mut output = format!(
        "# mode\t{}\n# selected\t{}\n# node\tsource\tmodule\n",
        if scope.reason == "scope-all" {
            "explicit"
        } else {
            "auto"
        },
        selected.len()
    );
    for face in collect_faces(src, nodes) {
        if selected.contains(&face.id) {
            writeln!(
                output,
                "{}\t{}\t{}",
                face.id,
                relative_display(src, &face.source),
                face.module
            )
            .unwrap();
        }
    }
    write_if_changed(&out_dir.join("source_scope.tsv"), &output)
}

/// Persist host graft selectors as data-only build metadata.
/// 将宿主 graft 选择器持久化为只含数据的构建元信息。
pub(crate) fn write_graft_manifest(out_dir: &Path, grafts: &[GraftSyntax]) -> Result<(), String> {
    let mut output = String::from("# cut\tgraft\tfull\tline\tcolumn\n");
    for graft in grafts {
        // The manifest is human-facing audit text, so a range is rendered back
        // to `"start to end"` here through the one rule that owns it.
        // 清单是给人看的审计文本，因此区间在这里经那条唯一的规则渲染回 `"start to end"`。
        let cut = super::graft_view::graft_cut_label(&graft.cut, graft.cut_end.as_deref());
        writeln!(
            output,
            "{cut}\t{}\t{}\t{}\t{}",
            graft.graft, graft.full, graft.location.line, graft.location.column
        )
        .unwrap();
    }
    write_if_changed(&out_dir.join("graft_plan.tsv"), &output)
}

/// Write the face manifest: identity, symbol, and the four facts the declaration spelled.
/// 写面清单：身份、符号，以及声明拼出的四项事实。
fn write_face_rows(
    path: impl AsRef<Path>,
    mut rows: Vec<(NodeId, String, String, FaceColumns)>,
) -> Result<(), String> {
    rows.sort();
    rows.dedup();
    let mut output = String::from("# node\tsource\tsymbol\tpath\tkind\tregistry_name\tparent\n");
    for (id, source, symbol, columns) in rows {
        writeln!(
            output,
            "{id}\t{source}\t{symbol}\t{}\t{}\t{}\t{}",
            columns.path, columns.kind, columns.registry_name, columns.parent
        )
        .unwrap();
    }
    write_if_changed(path.as_ref(), &output)
}

fn write_rows(
    path: impl AsRef<Path>,
    mut rows: Vec<(NodeId, String, String)>,
) -> Result<(), String> {
    rows.sort();
    rows.dedup();
    let mut output = String::from("# node\tsource\tsymbol\n");
    for (id, source, symbol) in rows {
        writeln!(output, "{id}\t{source}\t{symbol}").unwrap();
    }
    write_if_changed(path.as_ref(), &output)
}

/// List the functions one source file declares, through the kernel's lexical
/// scanner.
/// 通过内核的词法扫描器列出一个源码文件声明的函数。
///
/// The line scanner this replaced matched `fn ` anywhere on a line, so a
/// commented-out function or a doc example was a declaration, and it guessed an
/// `impl` owner from "the previous `impl ` line has no closing brace", which
/// nested `impl` blocks and closures polluted. The kernel scanner masks what is
/// not Rust before it looks, and reports no ownership — so the symbol is
/// `module::name` and nothing invented is published.
/// 它替换掉的按行扫描器在行的任意位置匹配 `fn `，因此被注释掉的函数或文档示例都成了声明；它还用
/// "上一个 `impl ` 行没有右花括号"猜 impl 归属，嵌套 impl 与闭包都会污染它。内核扫描器在寻找之前
/// 先屏蔽非 Rust 文本，且不报告归属——因此符号是 `module::name`，不发布任何编造出来的东西。
fn visit_function_symbols(src: &Path, nodes: &[Node], rows: &mut Vec<(NodeId, String, String)>) {
    for node in nodes {
        if let Some(file) = &node.file {
            let relative = relative_display(src, file);
            if let Ok(source) = fs::read_to_string(file)
                && !nichlink_kernel::lexicon::is_registration_path(&relative)
                && let Some(face) = parsed_face(&source, &relative)
            {
                let id = super::registry_identity::package_node_id(
                    &relative,
                    &face.path("kind").unwrap_or_else(|| node.name.clone()),
                );
                let module = source_module_path(&relative);
                for function in nichlink_kernel::source::function_symbols(&source) {
                    rows.push((id, relative.clone(), format!("{module}::{}", function.name)));
                }
            }
        }
        visit_function_symbols(src, &node.children, rows);
    }
}

fn visit_pruning_symbols(
    src: &Path,
    nodes: &[Node],
    rows: &mut Vec<(NodeId, String, String, FaceColumns)>,
) {
    for node in nodes {
        if let Some(file) = &node.file {
            let relative = relative_display(src, file);
            if let Ok(source) = fs::read_to_string(file)
                && !nichlink_kernel::lexicon::is_registration_path(&relative)
                && let Some(face) = parsed_face(&source, &relative)
            {
                let kind = face.path("kind").unwrap_or_else(|| node.name.clone());
                let id = super::registry_identity::package_node_id(&relative, &kind);
                let module = source_module_path(&relative);
                let columns = FaceColumns::declared(&face, &kind);
                let mut found = false;
                for item in source.lines().filter_map(parse_pruning_item) {
                    found = true;
                    rows.push((
                        id,
                        relative.clone(),
                        tracked_symbol(&module, &kind, item),
                        columns.clone(),
                    ));
                }
                if !found {
                    rows.push((id, relative, "-".to_owned(), columns));
                }
            }
        }
        visit_pruning_symbols(src, &node.children, rows);
    }
}

/// One tracked symbol's path, built from the facts that name it.
/// 一条被跟踪符号的路径，由命名它的事实拼出。
///
/// `PRUNING_TABLE` and `pruning_probe` are items of the face's module, so the path
/// is `module::item`. `optional_pruning_probe` is a method on the face's own type,
/// so the path starts at that type — the face's `kind`. The row scanner this
/// replaced returned the literal `Button::optional_pruning_probe` for every face,
/// which is a wrong answer for any face whose `kind` is not `Button`, and nothing
/// in the tree could notice: those three identifiers appear nowhere else in the
/// repository, so no fixture ever contradicted it.
/// `PRUNING_TABLE` 与 `pruning_probe` 是该面模块里的条目，路径即 `module::item`。
/// `optional_pruning_probe` 是该面自己类型上的方法，路径从那个类型——面的 `kind`——开始。它替换掉的
/// 按行扫描器对每个面都返回字面量 `Button::optional_pruning_probe`，对任何 `kind` 不叫 `Button` 的
/// 面都是错答案，而树里没有任何东西能发现：那三个标识符在本仓库别处不再出现，因此没有任何夹具反驳过它。
fn tracked_symbol(module: &str, kind: &str, item: PruningItem) -> String {
    match item {
        PruningItem::OptionalProbe => format!("{kind}::{}", item.name()),
        PruningItem::Table | PruningItem::Probe => format!("{module}::{}", item.name()),
    }
}

/// One of the three item shapes whose symbols the release prunes.
/// 发布期会修剪其符号的三种条目形状之一。
#[derive(Clone, Copy)]
enum PruningItem {
    /// `static PRUNING_TABLE: …` in the face's module.
    /// 面模块里的 `static PRUNING_TABLE: …`。
    Table,
    /// `fn pruning_probe` in the face's module.
    /// 面模块里的 `fn pruning_probe`。
    Probe,
    /// `fn optional_pruning_probe` on the face's own type.
    /// 面自己类型上的 `fn optional_pruning_probe`。
    OptionalProbe,
}

impl PruningItem {
    /// The name the source writes it under.
    /// 源码里书写它的那个名字。
    fn name(self) -> &'static str {
        match self {
            Self::Table => "PRUNING_TABLE",
            Self::Probe => "pruning_probe",
            Self::OptionalProbe => "optional_pruning_probe",
        }
    }
}

fn parse_pruning_item(line: &str) -> Option<PruningItem> {
    let line = line.trim();
    if line.contains("PRUNING_TABLE")
        && (line.starts_with("static ") || line.starts_with("pub static "))
    {
        Some(PruningItem::Table)
    } else if line.contains("pruning_probe") && line.contains("fn pruning_probe") {
        Some(PruningItem::Probe)
    } else if line.contains("optional_pruning_probe") && line.contains("fn optional_pruning_probe")
    {
        Some(PruningItem::OptionalProbe)
    } else {
        None
    }
}

#[cfg(test)]
#[path = "manifests_tests.rs"]
mod manifests_tests;
