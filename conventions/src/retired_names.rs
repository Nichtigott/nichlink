//! The retired-name gate: a name this workspace retired stays retired.
//! 退役名门禁：本工作区已退役的名字必须保持退役。
//!
//! Renaming a symbol or converging a piece of vocabulary is a repository-wide
//! decision, and the expensive part is never the rename itself — it is the
//! second pass, done by hand, that finds the comment, the README line and the
//! test doc that still spell the old name. This gate turns that second pass into
//! a list: a name goes into [`RETIRED`] together with the name that replaced it,
//! and the gate asserts the retired spelling appears **nowhere** in the live
//! tree. The list is the one place a retirement is recorded, and adding an entry
//! is a one-line change with a visible diff.
//! 符号改名或措辞统一是全仓决定，而昂贵的从来不是改名本身——是事后靠手工做第二遍，去找那条
//! 仍写着旧名的注释、README 行与测试文档。本门禁把第二遍变成一份清单：一个名字连同它的替换者
//! 一起写进 [`RETIRED`]，门禁断言旧拼法在活树里**一处都不出现**。这份清单是退役的唯一登记处，
//! 而新增一项是一行带可见 diff 的改动。
//!
//! Boundaries, stated because a gate that cries wolf gets switched off:
//! - Historical records are exempt. `CHANGELOG.md` and the audit reports under
//!   `docs/audit*` describe what was true when they were written; rewriting them
//!   would falsify a record. The exemption is by the same rule the doc gates use
//!   for records, plus `CHANGELOG.md` by name.
//! - A retired *word* is retired only in the sense that was converged. `slot
//!   name` is a live term in this workspace for three other concepts (a plugin
//!   slot, the authoring layout's slots, the form's field dictionary) and for a
//!   property the replacement check does not consult, so the entry below lists
//!   the *enumeration phrasings* that were converged rather than the bare term —
//!   see the comment on that entry for the evidence.
//! - Matching is word-bounded for words written in ASCII, so a retired name is
//!   not reported from inside a longer identifier or a plural.
//!
//! 边界，写出来是因为乱叫的门禁会被关掉：
//! - 历史记录豁免。`CHANGELOG.md` 与 `docs/audit*` 下的审计报告描述的是写下时的事实，改写它们等于
//!   篡改记录。豁免沿用文档门禁对记录的同一条规则，外加按名字豁免的 `CHANGELOG.md`。
//! - 被退役的**词**只在被统一过的那个意义上退役。`slot name` 在本工作区是另外三个概念的活词
//!   （插件槽、创作布局的槽位、表单字段词典），也是替换校验不咨询的那个属性，因此下面那条登记的是
//!   **被统一的枚举短语**，而不是裸词——理由与证据见该条上的注释。
//! - 每一项都在两端按词边界匹配，因此退役名不会从更长的标识符、复数或更长的词里被报出来。

use std::fs;
use std::path::{Path, PathBuf};

use crate::{crate_directories, relative, rust_sources};

/// One retired spelling and what replaced it.
/// 一个已退役的拼法及替代它的名字。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Finding {
    /// File relative to the workspace root, with `/` separators.
    /// 以工作区根为基准的文件路径，使用 `/` 分隔符。
    pub file: String,
    /// One-based line the retired spelling sits on.
    /// 旧拼法所在行（从 1 开始）。
    pub line: usize,
    /// The retired spelling that was found.
    /// 被找到的退役拼法。
    pub name: String,
    /// The name that replaced it.
    /// 替代它的名字。
    pub replacement: String,
}

/// One entry of the retired-name list.
/// 退役名清单的一项。
struct RetiredName {
    /// The spelling that must not appear.
    /// 不得再出现的拼法。
    written: &'static str,
    /// The spelling that replaced it.
    /// 替代它的拼法。
    replacement: &'static str,
    /// Why it was retired, in English and Chinese.
    /// 为何退役，英文与中文。
    why: &'static str,
}

/// The names this workspace has retired, with their replacements.
/// 本工作区已退役的名字及其替代者。
///
/// Add an entry when a rename or a wording convergence lands; the gate then
/// holds the new spelling everywhere in the live tree. `Symbol` entries are
/// word-bounded, `Phrase` entries are matched as written.
/// 改名或措辞统一落地时加一项，门禁随即在活树里守住新拼法。`Symbol` 项按词边界匹配，
/// `Phrase` 项按原文匹配。
pub const RETIRED: &[(&str, &str)] = &[
    ("apply_trait_contract", "apply_trait_label"),
    ("module and slot name", "module and `registry_name`"),
    ("module or slot name", "module or `registry_name`"),
    ("module, or slot name", "module, or `registry_name`"),
    ("模块与槽位名", "模块与 `registry_name`"),
    ("模块或槽位名", "模块或 `registry_name`"),
];

/// The retired list with its matching rule and its reason, one entry per name.
/// 退役清单连同匹配方式与理由，每个名字一项。
fn retired() -> Vec<RetiredName> {
    let mut names = Vec::new();
    for (written, replacement) in RETIRED {
        let why = match *written {
            // The authoring applier was renamed; the old name survived in two doc
            // comments (`macro/src/lib.rs`) after the symbol itself was gone
            // (audit `SUR-C1`'s batch, t39).
            // 创作应用器改过名；符号本身消失后，旧名在 `macro/src/lib.rs` 的两处文档注释里留了下来
            // （t39，`SUR-C1` 那一批）。
            "apply_trait_contract" => {
                "renamed to `apply_trait_label` (run_method/authoring/operations/face_write.rs)"
            }
            // The search tool's field enumeration was converged to `registry_name`
            // (t17/t18). Only the enumeration phrasings are listed — the ones whose
            // sentence names the *module* field next to it — because the bare term is
            // a live word elsewhere for the plugin slot (`plugin-host`,
            // `plugin/contracts.rs`), the authoring layout's slots
            // (`studio/src/studio/ui`), the form's field dictionary
            // (`face_manifest.rs` writes `字段词典与槽位名`, which a rule keyed on
            // `与槽位名` reported — my own first draft of this gate did exactly that,
            // and that is the false positive that gets a gate switched off), and the
            // property `validate_snapshot_replacement` does not consult
            // (`graft_ops/resolution.rs`).')
            // 搜索工具的字段枚举已统一为 `registry_name`（t17/t18）。这里只登记**枚举短语**——即
            // 句子里同时点名 module 字段的那些——因为裸词在别处是活词：插件槽（`plugin-host`、
            // `plugin/contracts.rs`）、创作布局的槽位（`studio/src/studio/ui`）、表单字段词典
            // （`face_manifest.rs` 写的是"字段词典与槽位名"，而按 `与槽位名` 判定的规则会把它报出来——
            // 本门禁初稿正是这样误报的，而那正是让人把门禁关掉的假阳性）、以及
            // `validate_snapshot_replacement` 不咨询的那个属性（`graft_ops/resolution.rs`）。
            // 搜索工具的字段枚举已统一为 `registry_name`（t17/t18）。这里只登记**枚举短语**：裸词在别处
            // 是活词——插件槽（`plugin-host`、`plugin/contracts.rs`）、创作布局的槽位
            // （`studio/src/studio/ui`）、表单字段词典（`face_manifest.rs`、
            // `ungated_authoring_data.rs`），以及 `validate_snapshot_replacement` 不咨询的那个属性
            // （`graft_ops/resolution.rs`）——登记裸词会把它们全部报出来，而那正是让人把门禁关掉的假阳性
            // （t37/t45 的归类）。
            _ => "converged to `registry_name` (t17/t18 wording)",
        };
        names.push(RetiredName {
            written,
            replacement,
            why,
        });
    }
    names
}

/// Every file the gate reads: live Rust, live documentation, manifests, workflows
/// and the repository's scripts.
/// 门禁读取的每个文件：活 Rust、活文档、清单、工作流与仓库脚本。
fn live_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for directory in crate_directories(root) {
        files.extend(rust_sources(&directory));
    }
    files.extend(crate::doc_blocks::markdown_files(root));
    let manifest = root.join("Cargo.toml");
    if manifest.is_file() {
        files.push(manifest);
    }
    for directory in crate_directories(root) {
        let manifest = directory.join("Cargo.toml");
        if manifest.is_file() {
            files.push(manifest);
        }
    }
    let workflows = root.join(".github").join("workflows");
    if let Ok(entries) = fs::read_dir(&workflows) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && path
                    .extension()
                    .is_some_and(|extension| extension == "yml" || extension == "yaml")
            {
                files.push(path);
            }
        }
    }
    if let Ok(entries) = fs::read_dir(root.join("tools")) {
        for entry in entries.flatten() {
            if entry.path().is_file() {
                files.push(entry.path());
            }
        }
    }
    files.sort();
    files.dedup();
    files
}

/// Whether a file records history rather than living documentation, or is the
/// gate's own source.
/// 该文件是记录历史而不是活文档，或者就是本门禁自己的源码。
fn is_exempt(root: &Path, path: &Path) -> bool {
    let relative = relative(root, path);
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    name == "retired_names.rs"
        || name == "retired_names_tests.rs"
        || relative == "CHANGELOG.md"
        || relative.starts_with("docs/audit")
}

/// Every occurrence of `written` in `text`, as byte offsets.
/// `text` 中 `written` 的每次出现，以字节偏移给出。
///
/// Every entry is word-bounded, phrases included: the phrase `and slot name` was
/// converged, and the form's field dictionary writes `and slot names` — without
/// the trailing boundary test the gate would report that live dictionary as a
/// retired name, which is exactly the false positive that gets a gate switched
/// off.
/// 每一项都按词边界匹配，短语也一样：被统一的是短语 `and slot name`，而表单字段词典写的是
/// `and slot names`——没有末尾边界判定，门禁会把这个活词典当退役名报出来，而那正是让人把门禁
/// 关掉的假阳性。
fn occurrences(text: &str, written: &str) -> Vec<usize> {
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(offset) = text[from..].find(written) {
        let at = from + offset;
        from = at + written.len();
        let before = text[..at].chars().next_back();
        let after = text[at + written.len()..].chars().next();
        if before.is_some_and(is_word_character) || after.is_some_and(is_word_character) {
            continue;
        }
        found.push(at);
    }
    found
}

/// Whether a character continues a word, for the ASCII word-boundary test.
/// 该字符是否延续一个词，供 ASCII 词边界判定使用。
fn is_word_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

/// Every retired spelling still written in the live tree, sorted by file and line.
/// 活树里仍被写着的每个退役拼法，按文件与行排序。
pub fn findings(root: &Path) -> Vec<Finding> {
    let names = retired();
    let mut found = Vec::new();
    for path in live_files(root) {
        if is_exempt(root, &path) {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for (index, line) in text.lines().enumerate() {
            for name in &names {
                if !occurrences(line, name.written).is_empty() {
                    found.push(Finding {
                        file: relative(root, &path),
                        line: index + 1,
                        name: name.written.to_owned(),
                        replacement: name.replacement.to_owned(),
                    });
                }
            }
        }
    }
    found.sort();
    found.dedup();
    found
}

/// The reason one retired name was retired, for the gate's own report.
/// 某个退役名为何退役，供门禁自己的报告使用。
pub fn reason(name: &str) -> Option<&'static str> {
    retired()
        .iter()
        .find(|entry| entry.written == name)
        .map(|entry| entry.why)
}

#[cfg(test)]
#[path = "retired_names_tests.rs"]
mod retired_names_tests;
