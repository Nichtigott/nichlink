//! Documentation anchors that must still resolve.
//! 必须仍然可解析的文档锚点。
//!
//! A document that says "the check lives in `core/src/…/inspection.rs:74`" makes a
//! promise a reader can check in one step. Nothing checked it, so the promise
//! decayed: `docs/roadmap-1.0.md` cited `run_method/src/macros/face_objects.rs:242-243`
//! for the arm that expands `$preset`/`$parts`, in a file that is now 172 lines
//! long. [`crate::doc_blocks`] already refuses a fenced Rust block that no longer
//! parses; this gate refuses a reference that no longer points anywhere.
//! 一份写着"检查在 `core/src/…/inspection.rs:74`"的文档，给读者留下一步就能核实的承诺。没有
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

use std::fs;
use std::path::{Path, PathBuf};

use crate::doc_blocks::markdown_files;
use crate::{crate_directories, relative, rust_sources};

/// One documented `<file>.rs:<line>` reference that no longer resolves.
/// 一处不再可解析的文档 `<file>.rs:<line>` 引用。
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

/// Every stale anchor in the living documentation, sorted by document and anchor.
/// 活文档里每一处失效锚点，按文档与锚点排序。
pub fn findings(root: &Path) -> Vec<Finding> {
    let sources = workspace_sources(root);
    let roots = root_directories(root);
    let mut found = Vec::new();
    for document in markdown_files(root) {
        let text = fs::read_to_string(&document)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", document.display()));
        for anchor in anchors(&text) {
            let matches = resolve(&anchor.path, &sources);
            let target = match matches.as_slice() {
                [only] => (*only).clone(),
                [] => {
                    // Written from the workspace root and naming nothing: this is the
                    // rename-and-forget shape the gate exists for.
                    // 从工作区根写起却什么都没命名到：这正是门禁为之存在的"改名后忘记更新"
                    // 的形状。
                    if roots
                        .iter()
                        .any(|name| anchor.path.starts_with(&format!("{name}/")))
                    {
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
            let lines = fs::read_to_string(&target)
                .map(|text| text.lines().count())
                .unwrap_or(0);
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
            }
        }
    }
    found.sort_by(|left, right| {
        (&left.document, &left.anchor).cmp(&(&right.document, &right.anchor))
    });
    found.dedup();
    found
}

/// Every `.rs` file in the workspace, as `(root-relative path, path)`, sorted.
/// 工作区里每个 `.rs` 文件，形如 `(根相对路径, 路径)`，已排序。
fn workspace_sources(root: &Path) -> Vec<(String, PathBuf)> {
    let mut files = Vec::new();
    for directory in crate_directories(root) {
        for path in rust_sources(&directory) {
            files.push((relative(root, &path), path));
        }
    }
    files.sort();
    files.dedup();
    files
}

/// The names of the workspace root's own directories.
/// 工作区根自己那些目录的名字。
///
/// A written path that starts with one of these is claiming to be written from the
/// workspace root, so naming nothing is a broken reference rather than shorthand.
/// 以其中之一开头的写下的路径，是在声称自己从工作区根写起，因此什么都没命名到就是坏引用，
/// 而不是简写。
fn root_directories(root: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut names = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| crate::is_real_directory(path))
        .filter_map(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(str::to_owned)
        })
        .collect::<Vec<_>>();
    names.sort();
    names
}

/// The workspace files a written path could name.
/// 一个写下的路径可能命名到的那些工作区文件。
///
/// The exact root-relative path wins; otherwise a *unique* path ending in the
/// written one does, which is what lets a document inside a crate section write
/// `app/lifecycle.rs` for `studio/src/studio/app/lifecycle.rs`.
/// 正好等于根相对路径者优先；否则由唯一一个以它结尾的路径命中——这正是让 crate 小节里的文档
/// 用 `app/lifecycle.rs` 指 `studio/src/studio/app/lifecycle.rs` 的原因。
fn resolve<'a>(written: &str, sources: &'a [(String, PathBuf)]) -> Vec<&'a PathBuf> {
    let exact = sources
        .iter()
        .filter(|(path, _)| path == written)
        .map(|(_, file)| file)
        .collect::<Vec<_>>();
    if !exact.is_empty() {
        return exact;
    }
    let suffix = format!("/{written}");
    sources
        .iter()
        .filter(|(path, _)| path.ends_with(&suffix))
        .map(|(_, file)| file)
        .collect()
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

/// Every `.rs:<line>` reference in `text`, in order.
/// `text` 中每一处 `.rs:<line>` 引用，按出现顺序。
fn anchors(text: &str) -> Vec<Anchor> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut cursor = 0;
    while let Some(offset) = text[cursor..].find(".rs:") {
        let at = cursor + offset;
        let mut start = at;
        while start > 0 && is_path_byte(bytes[start - 1]) {
            start -= 1;
        }
        let path = text[start..at + 3].to_owned();
        let digits = at + 4;
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
                path,
                first,
                last: last.unwrap_or(first),
            });
        }
        cursor = after.max(at + 4);
    }
    found
}

/// A byte that can be part of a written path.
/// 可以作为路径一部分的字节。
fn is_path_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'/' | b'-')
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A throwaway checkout with the given documents and sources.
    /// 一个只含给定文档与源码的一次性检出。
    fn synthetic(documents: &[(&str, &str)], sources: &[(&str, &str)]) -> PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "nichlink-anchors-{}-{sequence}",
            std::process::id()
        ));
        for (relative, contents) in sources.iter().chain(documents) {
            let path = root.join(relative);
            fs::create_dir_all(path.parent().expect("parent")).expect("fixture dir");
            fs::write(&path, contents).expect("fixture file");
        }
        crate::fixture_manifest(&root);
        root
    }

    /// The reference this gate exists for: a line number past the end of the file.
    /// 本门禁为之存在的引用：行号超出文件末尾。
    #[test]
    fn a_reference_past_the_end_of_a_file_is_reported() {
        let root = synthetic(
            &[(
                "docs/probe.md",
                "see `core/src/probe.rs:99` for the check\n",
            )],
            &[("core/src/probe.rs", "fn probe() {}\n")],
        );
        let found = findings(&root);
        assert_eq!(found.len(), 1, "{found:#?}");
        assert_eq!(found[0].document, "docs/probe.md");
        assert_eq!(found[0].anchor, "core/src/probe.rs:99");
        assert!(found[0].reason.contains("1 lines"), "{found:#?}");
        let _ = fs::remove_dir_all(&root);
    }

    /// A reference to a file that is gone is reported too: a rename is the other
    /// way an anchor rots.
    /// 指向已消失文件的引用同样被报出：改名是锚点腐化的另一种方式。
    #[test]
    fn a_reference_to_a_missing_file_is_reported() {
        let root = synthetic(
            &[("docs/probe.md", "moved to `core/src/gone.rs:1`\n")],
            &[("core/src/probe.rs", "fn probe() {}\n")],
        );
        let found = findings(&root);
        assert_eq!(found.len(), 1, "{found:#?}");
        assert_eq!(found[0].reason, "no such file");
        let _ = fs::remove_dir_all(&root);
    }

    /// A bare name that two files share is not guessed at, and a bare name no file
    /// owns is prose.
    /// 两个文件共有的裸名不去猜；没有文件拥有的裸名是散文。
    #[test]
    fn an_ambiguous_or_unknown_bare_name_is_left_alone() {
        let root = synthetic(
            &[(
                "docs/probe.md",
                "`same.rs:99` is ambiguous, `elsewhere.rs:99` is not ours\n",
            )],
            &[
                ("core/src/same.rs", "fn a() {}\n"),
                ("cli/src/same.rs", "fn b() {}\n"),
            ],
        );
        assert_eq!(findings(&root), Vec::new());
        let _ = fs::remove_dir_all(&root);
    }

    /// A bare name that is unique is checked, because it can be resolved without
    /// guessing.
    /// 唯一的裸名会被检查，因为它无需猜测即可解析。
    #[test]
    fn a_unique_bare_name_is_checked() {
        let root = synthetic(
            &[("docs/probe.md", "the arm is in `probe.rs:99`\n")],
            &[("core/src/probe.rs", "fn probe() {}\n")],
        );
        let found = findings(&root);
        assert_eq!(found.len(), 1, "{found:#?}");
        assert_eq!(found[0].anchor, "probe.rs:99");
        let _ = fs::remove_dir_all(&root);
    }

    /// An audit document is a record: its anchors are as true as they were when it
    /// was written, and the gate leaves it alone.
    /// 审计文档是记录：它的锚点在写下时是真的，门禁不碰它。
    #[test]
    fn an_anchor_inside_a_record_is_exempt() {
        let root = synthetic(
            &[(
                "docs/audit-probe.md",
                "it used to be `core/src/probe.rs:99`\n",
            )],
            &[("core/src/probe.rs", "fn probe() {}\n")],
        );
        assert_eq!(findings(&root), Vec::new());
        let _ = fs::remove_dir_all(&root);
    }

    /// The documentation in this checkout resolves.
    /// 本检出里的文档可解析。
    #[test]
    fn the_shipped_documentation_anchors_resolve() {
        let found = findings(&crate::workspace_root());
        assert!(
            found.is_empty(),
            "a documented anchor no longer points anywhere; fix the reference or the \
             sentence that needs it: {found:#?}"
        );
    }
}
