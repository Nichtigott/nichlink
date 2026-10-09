//! Documentation anchors that must still resolve.
//! 必须仍然可解析的文档锚点。
//!
//! A document that says "the check lives in `kernel/src/…/inspection.rs:74`" makes a
//! promise a reader can check in one step. Nothing checked it, so the promise
//! decayed: `docs/roadmap-1.0.md` cited `run_method/src/macros/face_objects.rs:242-243`
//! for the arm that expands `$preset`/`$parts`, in a file that is now 172 lines
//! long. [`crate::doc_blocks`] already refuses a fenced Rust block that no longer
//! parses; this gate refuses a reference that no longer points anywhere.
//! 一份写着"检查在 `kernel/src/…/inspection.rs:74`"的文档，给读者留下一步就能核实的承诺。没有
//! 任何东西核实它，于是承诺腐化了：`docs/roadmap-1.0.md` 为展开 `$preset`/`$parts` 的宏臂引用了
//! `run_method/src/macros/face_objects.rs:242-243`，而那个文件现在只有 172 行。
//! [`crate::doc_blocks`] 已经会拒绝不再能解析的 Rust 围栏；本门禁拒绝不再指向任何地方的引用。
//!
//! Scope, stated so it is not mistaken for more than it is: the gate checks that the
//! file exists and that the line number is inside it. It cannot tell that a line
//! number is *stale by a few lines* — the anchor still resolves and the sentence
//! still describes the wrong line, which only a reader can catch. Anchors in audit
//! and design documents are exempt for the same reason their code blocks are: they
//! record what was true when they were written.
//! 边界，明说以免被当成比实际更强的东西：本门禁检查文件存在、行号落在文件内。它无法判断某个
//! 行号**只差几行**——锚点仍然可解析，而那句话仍指向错的行，只有读者能发现。审计与设计文档里的
//! 锚点同它们的代码块一样豁免：它们记录的是写下时的事实。
//!
//! Coverage, and why it stops where it does: the scan keys on the written path's suffix, so
//! it reads Rust sources, the workspace manifests and `.github/workflows/*.yml` — the three
//! kinds a document cites by line, the release workflow's tag guard among them. Two kinds
//! stay out, at a stated price: a `.md` target names prose, so a line number in it carries no
//! symbol to check, and widening the scan to markdown would sweep the self-references of every
//! living document; an extensionless script (`tools/nichlink-publish:88`) has no suffix to key
//! on. Both therefore rot silently, and this paragraph is the record.
//! 覆盖范围，以及它为什么停在这里：扫描以写下路径的后缀为判据，因此它读 Rust 源码、工作区清单与
//! `.github/workflows/*.yml`——文档按行引用的正是这三类，发布工作流的 tag 守卫也在其中。有两类
//! 留在外面，代价写明：`.md` 目标命名的是散文，其中的行号没有可供核对的符号，而把扫描扩到
//! markdown 会扫过每一份活文档的自引用；无扩展名的脚本（`tools/nichlink-publish:88`）没有可作
//! 判据的后缀。两者因此会静默腐化，而这一段就是那份记录。

use std::fs;
use std::path::{Path, PathBuf};

use crate::doc_blocks::markdown_files;
use crate::{crate_directories, relative, rust_sources};

/// One documented `<file>:<line>` reference that no longer resolves.
/// 一处不再可解析的文档 `<file>:<line>` 引用。
#[derive(Debug, PartialEq, Eq)]
pub struct Finding {
    /// Document holding the reference, relative to the workspace root.
    /// 持有该引用的文档，以工作区根为基准。
    pub document: String,
    /// The reference as it was written.
    /// 引用被写下的样子。
    pub anchor: String,
    /// Why it does not resolve.
    /// 它为何不可解析。
    pub reason: String,
}

/// The prose of a document: a fenced code block is a **citation**, not a reference.
/// 一份文档的正文：围栏代码块是**引文**，不是引用。
///
/// Fenced blocks are where raw output goes, and raw output is full of `path:line` — a compiler's
/// `--> …/out/generated_lib.rs:182:1`, a diagnostic's `source=board/object/tile2/tile2.rs:11`. Those name
/// build artifacts and files outside this checkout **by construction**, so judging them as anchors made
/// two rules of this repository impossible to satisfy at once: "record the command's raw output" and
/// "keep every anchor resolving". Measured while writing `docs/closeout-2026-10-09-0.2.x.md`: the two
/// findings were both inside fenced blocks, quoting a compiler error and a build diagnostic.
/// 围栏块是放原始输出的地方，而原始输出满是 `path:line`——编译器的 `--> …/out/generated_lib.rs:182:1`、
/// 诊断的 `source=board/object/tile2/tile2.rs:11`。它们**按构造**点名的就是构建产物与检出之外的文件，因此
/// 把它们判成锚点，会让本仓库的两条规则无法同时成立："记录命令的原始输出"与"保持每个锚点可解析"。写
/// `docs/closeout-2026-10-09-0.2.x.md` 时实测：那两条发现都落在围栏块里，引的是一段编译错误与一条构建诊断。
///
/// An indented block is not stripped: this repository's documents use fences, and guessing at Markdown's
/// other block spellings is how a gate starts letting real references through.
/// 缩进块不被剥掉：本仓库的文档用围栏，而靠猜 Markdown 其它的块拼法是门禁开始放行真引用的方式。
fn prose_only(text: &str) -> String {
    let mut prose = String::new();
    let mut fenced = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if !fenced {
            prose.push_str(line);
        }
        prose.push('\n');
    }
    prose
}

/// Every stale anchor in the living documentation, sorted by document and anchor.
/// 活文档里每一处失效锚点，按文档与锚点排序。
pub fn findings(root: &Path) -> Vec<Finding> {
    let sources = workspace_sources(root);
    let mut found = Vec::new();
    for document in markdown_files(root) {
        let text = fs::read_to_string(&document)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", document.display()));
        for anchor in anchors(&prose_only(&text)) {
            let matches = resolve_written_path(&anchor.path, &sources);
            let target = match matches.as_slice() {
                [only] => (*only).clone(),
                [] => {
                    // Written as a path and naming nothing: this is the rename-and-forget
                    // shape the gate exists for. A bare filename that matches nothing is
                    // still left alone, because it is prose shorthand before it is a
                    // reference; the slash is what says the author meant a path — and it
                    // is exactly the shape `src/definitely-gone.rs:99` used to hide in,
                    // because that check asked whether the *first* segment was a workspace
                    // root rather than whether the reference was written as a path.
                    // 写成路径却什么都没命名到：这正是门禁为之存在的"改名后忘记更新"的形状。
                    // 什么都没匹配到的裸文件名仍被放过，因为在成为引用之前它先是散文简写；是斜杠说明
                    // 作者指的是路径——而 `src/definitely-gone.rs:99` 正是过去藏身的形状，因为那道
                    // 检查问的是**第一段**是不是工作区根目录，而不是这处引用是否写成了路径。
                    if normalize(&anchor.path).contains('/') {
                        found.push(Finding {
                            document: relative(root, &document),
                            anchor: anchor.text(),
                            reason: "no such file".to_owned(),
                        });
                    }
                    continue;
                }
                // Two files can end with the same written path, and a bare name two
                // crates share is prose shorthand rather than a reference: the gate
                // stays silent instead of guessing which one was meant.
                // 两个文件可能以同一个写下的路径结尾，而两个 crate 共有的裸名是散文简写而不是
                // 引用：门禁保持沉默，而不是猜作者指的是哪一个。
                _ => continue,
            };
            let body = fs::read_to_string(&target).unwrap_or_default();
            let lines = body.lines().count();
            // Line numbers are 1-based, and `path.rs:0` used to pass every check: the
            // comparison asked whether the number was past the end, and zero never is.
            // 行号从 1 起，而 `path.rs:0` 过去能通过所有检查：那次比较问的是数字是否越过末尾，而 0
            // 永远不会越过。
            if anchor.first == 0 || anchor.last == 0 {
                found.push(Finding {
                    document: relative(root, &document),
                    anchor: anchor.text(),
                    reason: "line 0 is not a 1-based line number".to_owned(),
                });
                continue;
            }
            let wanted = anchor.last.max(anchor.first);
            if wanted > lines {
                found.push(Finding {
                    document: relative(root, &document),
                    anchor: anchor.text(),
                    reason: format!(
                        "line {wanted} is past the end of {} ({lines} lines)",
                        relative(root, &target)
                    ),
                });
                continue;
            }
            // A line number is not a reference by itself: the number can stay inside the
            // file while the code moves under it. When the document pairs the reference
            // with a token, the token is what makes the line number checkable.
            // 行号本身不是一处引用：代码在下面移动时，数字仍可能留在文件内。当文档把引用与一个
            // token 成对写出时，那个 token 才是让行号可被检查的东西。
            if let Some(token) = &anchor.token
                && let Some(fragment) = token_fragment(token)
            {
                // A range names a region, so the token may sit on any line of it; a single
                // line number names that line.
                // 区间命名的是一个区域，因此 token 可以落在其中任意一行；单个行号命名的就是那一行。
                let cited = body
                    .lines()
                    .skip(anchor.first.saturating_sub(1))
                    .take(wanted.saturating_sub(anchor.first) + 1)
                    .collect::<Vec<_>>()
                    .join("\n");
                if !cited.contains(fragment) {
                    found.push(Finding {
                        document: relative(root, &document),
                        anchor: anchor.text(),
                        reason: format!(
                            "{} does not contain `{token}`; the reference has drifted — point at \
                             the symbol, or move the number",
                            anchor.text()
                        ),
                    });
                }
            }
        }
    }
    found.sort_by(|left, right| {
        (&left.document, &left.anchor).cmp(&(&right.document, &right.anchor))
    });
    found.dedup();
    found
}

/// The file kinds a documented `path:line` reference may name: the suffixes the scan keys
/// on, and the kinds [`workspace_sources`] therefore collects.
/// 文档 `path:line` 引用可以命名的文件种类：扫描据以触发的后缀，也是 [`workspace_sources`]
/// 因此收集的种类。
const READ_EXTENSIONS: &[&str] = &[".rs", ".toml", ".yml", ".yaml"];

/// Every file a documented anchor may name, as `(root-relative path, path)`, sorted.
/// 文档锚点可以命名的每个文件，形如 `(根相对路径, 路径)`，已排序。
fn workspace_sources(root: &Path) -> Vec<(String, PathBuf)> {
    let mut files = Vec::new();
    for directory in crate_directories(root) {
        for path in rust_sources(&directory) {
            files.push((relative(root, &path), path));
        }
    }
    files.extend(manifests(root));
    files.extend(workflows(root));
    files.sort();
    files.dedup();
    files
}

/// The root manifest and every member's manifest, as `(root-relative path, path)`.
/// 根清单与每个成员的清单，形如 `(根相对路径, 路径)`。
fn manifests(root: &Path) -> Vec<(String, PathBuf)> {
    std::iter::once(root.join("Cargo.toml"))
        .chain(
            crate_directories(root)
                .into_iter()
                .map(|directory| directory.join("Cargo.toml")),
        )
        .filter(|manifest| manifest.is_file())
        .map(|manifest| (relative(root, &manifest), manifest))
        .collect()
}

/// Every workflow in `.github/workflows`, as `(root-relative path, path)`.
/// `.github/workflows` 里的每个工作流，形如 `(根相对路径, 路径)`。
fn workflows(root: &Path) -> Vec<(String, PathBuf)> {
    let Ok(entries) = fs::read_dir(root.join(".github").join("workflows")) else {
        return Vec::new();
    };
    let mut files = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .is_some_and(|extension| extension == "yml" || extension == "yaml")
        })
        .map(|path| (relative(root, &path), path))
        .collect::<Vec<_>>();
    files.sort();
    files
}

/// The workspace files a written path could name.
/// 一个写下的路径可能命名到的那些工作区文件。
///
/// The exact root-relative path wins; otherwise a *unique* path ending in the
/// written one does, which is what lets a document inside a crate section write
/// `app/lifecycle.rs` for `studio/src/studio/app/lifecycle.rs`.
/// 正好等于根相对路径者优先；否则由唯一一个以它结尾的路径命中——这正是让 crate 小节里的文档
/// 用 `app/lifecycle.rs` 指 `studio/src/studio/app/lifecycle.rs` 的原因。
fn resolve_written_path<'a>(written: &str, sources: &'a [(String, PathBuf)]) -> Vec<&'a PathBuf> {
    // A written path may climb with `..` — `declaration/../../tree/transaction/…` is how one
    // roadmap row names a file two levels up — and the suffix test below compares strings, so
    // the segments are collapsed first. Without this the gate called a file that exists
    // "no such file", which is the false positive that gets a gate switched off.
    // 写下的路径可以用 `..` 向上爬——roadmap 有一行正是用 `declaration/../../tree/transaction/…`
    // 命名向上两级的文件——而下面的后缀测试比较的是字符串，因此先把这些段折叠掉。没有这一步，门禁会
    // 把一个确实存在的文件说成 "no such file"，而那种假阳性正是让人把门禁关掉的东西。
    let normalized = normalize(written);
    let exact = sources
        .iter()
        .filter(|(path, _)| path == &normalized)
        .map(|(_, file)| file)
        .collect::<Vec<_>>();
    if !exact.is_empty() {
        return exact;
    }
    let suffix = format!("/{normalized}");
    sources
        .iter()
        .filter(|(path, _)| path.ends_with(&suffix))
        .map(|(_, file)| file)
        .collect()
}

/// `written` with its `.` and `..` segments resolved lexically.
/// 把 `written` 里的 `.` 与 `..` 段按词法解掉。
fn normalize(written: &str) -> String {
    let mut segments: Vec<&str> = Vec::new();
    for segment in written.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            other => segments.push(other),
        }
    }
    segments.join("/")
}

/// One `path.rs:first[-last]` reference found in a document.
/// 文档里找到的一处 `path.rs:first[-last]` 引用。
#[derive(Debug)]
struct Anchor {
    /// The path exactly as written, `.../file.rs`.
    /// 按原文写下的路径，`.../file.rs`。
    path: String,
    /// The first line named.
    /// 命名的第一行。
    first: usize,
    /// The last line named, equal to `first` when no range was written.
    /// 命名的最后一行；没有写区间时等于 `first`。
    last: usize,
    /// The literal the reference is paired with, when the document writes
    /// `` `<literal>` (`path.rs:line`) ``.
    /// 与引用成对的那个字面量，当文档写作 `` `<字面量>`（`path.rs:<line>`）`` 时。
    token: Option<String>,
}

impl Anchor {
    /// The reference as the document wrote it.
    /// 引用在文档里被写下的样子。
    fn text(&self) -> String {
        if self.last == self.first {
            format!("{}:{}", self.path, self.first)
        } else {
            format!("{}:{}-{}", self.path, self.first, self.last)
        }
    }
}

/// Every `path:line` reference in `text` for a [`READ_EXTENSIONS`] file kind, in order.
/// `text` 中每一处指向 [`READ_EXTENSIONS`] 文件种类的 `path:line` 引用，按出现顺序。
///
/// A colon is not a reference by itself, so the trigger is the *written path's suffix*: the
/// scan used to key on `.rs:` alone, which left every citation of a manifest or a workflow
/// (`Cargo.toml:12`, `.github/workflows/release.yml:66`) outside the gate forever.
/// 冒号本身不是引用，因此判据是**写下路径的后缀**：扫描过去只以 `.rs:` 触发，把所有指向清单或
/// 工作流的引用（`Cargo.toml:12`、`.github/workflows/release.yml:66`）永远挡在门禁之外。
fn anchors(text: &str) -> Vec<Anchor> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut cursor = 0;
    while let Some(offset) = text[cursor..].find(':') {
        let at = cursor + offset;
        cursor = at + 1;
        let digits = at + 1;
        if !bytes.get(digits).is_some_and(u8::is_ascii_digit) {
            continue;
        }
        let mut start = at;
        while start > 0 && is_path_byte(bytes[start - 1]) {
            start -= 1;
        }
        let path = &text[start..at];
        if !READ_EXTENSIONS
            .iter()
            .any(|extension| path.ends_with(extension))
        {
            continue;
        }
        let mut after = digits;
        while after < bytes.len() && bytes[after].is_ascii_digit() {
            after += 1;
        }
        let first = text[digits..after].parse::<usize>().ok();
        let mut last = first;
        // A range is written `-` or `–`; the en dash is what a Chinese document
        // reaches for, and it is two bytes, so the index advances by its length.
        // 区间写作 `-` 或 `–`；中文文档会用到后者，而它是两字节，因此下标按它的长度前进。
        if let Some(open) = text[after..].chars().next()
            && matches!(open, '-' | '–')
        {
            let range = after + open.len_utf8();
            let mut end = range;
            while end < bytes.len() && bytes[end].is_ascii_digit() {
                end += 1;
            }
            if let Ok(close) = text[range..end].parse::<usize>() {
                last = Some(close);
                after = end;
            }
        }
        if let Some(first) = first {
            found.push(Anchor {
                token: paired_token(text, start),
                path: path.to_owned(),
                first,
                last: last.unwrap_or(first),
            });
        }
        cursor = after.max(digits);
    }
    found
}

/// A byte that can be part of a written path.
/// 可以作为路径一部分的字节。
fn is_path_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'/' | b'-')
}

/// The literal a reference is paired with, when written `` `<literal>` (`path.rs:line`) ``.
/// 引用成对的那个字面量，当写作 `` `<字面量>`（`path.rs:<line>`）`` 时。
///
/// Only the direct pairing is read. A group that cites several files in one parenthesis
/// (`` (`a.rs:1`, `b.rs:2`) ``) holds one literal for two references, and guessing which of
/// them it belongs to would report a drift the document does not have.
/// 只读直接的成对写法。一个括号里引用多个文件（``（`a.rs:1`、`b.rs:2`）``）时，一个字面量对应
/// 两处引用，猜它属于哪一处会报出文档里并不存在的漂移。
fn paired_token(text: &str, start: usize) -> Option<String> {
    let before = &text[..start];
    let rest = before.trim_end().strip_suffix('`')?;
    let rest = rest.trim_end();
    let rest = rest
        .strip_suffix('(')
        .or_else(|| rest.strip_suffix('（'))?
        .trim_end()
        .strip_suffix('`')?;
    let open = rest.rfind('`')?;
    let literal = rest[open + 1..].to_owned();
    if token_fragment(&literal).is_some() {
        Some(literal)
    } else {
        None
    }
}

/// The part of a paired literal that must appear on the line it names, if it is a token.
/// 成对字面量里必须出现在它所命名那一行上的部分——前提是它确实是个 token。
///
/// A literal with whitespace in it is a sentence about the code, not a token of it
/// (`runtime health check failed`), and a sentence is reworded rather than moved; only
/// code-shaped literals are compared. The fragment stops at the first placeholder marker,
/// because `<unknown:{node}>` is a format string in the source rather than literal text,
/// and loses a trailing `/` for the same reason `<edited>/…` is an elision.
/// 含空白的字面量是关于代码的一句话而不是代码里的一个 token（`runtime health check
/// failed`），而句子会被改写而不是被搬走，因此只比对代码形状的字面量。片段在第一个占位符标记处
/// 结束，因为 `<unknown:{node}>` 在源码里是格式串而不是字面文本；同样理由，尾部的 `/` 会被去掉，
/// 因为 `<edited>/…` 是一种省略写法。
fn token_fragment(literal: &str) -> Option<&str> {
    if literal.contains(char::is_whitespace) {
        return None;
    }
    let end = literal
        .char_indices()
        .find(|(_, character)| matches!(character, '{' | ':' | '…'))
        .map(|(index, _)| index)
        .unwrap_or(literal.len());
    let fragment = literal[..end].trim_end_matches('/');
    (!fragment.is_empty()).then_some(fragment)
}

#[cfg(test)]
#[path = "doc_anchors_tests.rs"]
mod doc_anchors_tests;
