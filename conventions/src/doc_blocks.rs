//! The documentation-block gate: Rust fenced in markdown must still parse.
//! 文档代码块门禁：markdown 里围栏标记的 Rust 必须仍然能解析。
//!
//! Why this exists: the root `README.md`, every crate `README.md`, and
//! `docs/*.md` together hold every Rust-tagged fence a reader is expected to
//! follow, and the READMEs *are* the
//! crates.io landing pages for all nine published crates (`readme =
//! "README.md"` in every manifest). Nothing extracted, compiled, or even
//! parsed one of them, so a stale signature could ship and be read by every
//! user without a single gate noticing.
//! 为什么需要它：根 `README.md`、每个 crate 的 `README.md` 与 `docs/*.md` 合计持有读者
//! 预期遵循的每一个标记为 Rust 的围栏，而这些 README **就是**九个已发布 crate 在 crates.io
//! 上的落地页（每个清单都写了 `readme = "README.md"`）。此前没有任何程序抽取、编译、甚至解析过
//! 其中任何一个，因此一份过期的签名可以发布出去被每个用户读到，而没有任何门禁察觉。
//! （这里有意不写数量：散文里的数字会漂，而门禁覆盖的是全部，不是某一个数。）
//!
//! Boundary, and why it is not the stronger check: a fence is accepted when it
//! parses either as a whole file or as a statement block, because most of these
//! blocks are deliberately excerpts — a file's contents, a macro invocation, or
//! a statement using `?`. Making them *compile* would require wrapping every
//! one in a harness with imports and a crate root, and `#![doc =
//! include_str!("../README.md")]`, the usual way to doctest a README, would turn
//! every one of them into a doctest and fail the build on the excerpts that cannot compile
//! standalone. Parsing catches syntax rot today; compiling the excerpts is a
//! separate, larger piece of work and is recorded as such in the roadmap.
//! 边界，以及为什么不做更强的检查：围栏只要能作为整份文件或作为语句块解析就通过，因为这些块
//! 大多是有意的摘录——某文件的内容、一次宏调用、或一条使用 `?` 的语句。要让它们**编译**，
//! 就得给每一个套上带导入与 crate 根的脚手架；而 `#![doc = include_str!("../README.md")]`
//! 这个给 README 做 doctest 的常规做法，会把每一个块都变成 doctest，并在那些无法独立编译的
//! 摘录上让构建失败。解析能抓住今天的语法腐化；让摘录真正编译是另一件更大的工作，已在路线图
//! 中如实登记。

use std::path::{Path, PathBuf};

use crate::size::{is_mounted_as_test, looks_test_only};
use crate::{crate_directories, lines, relative, rust_sources};

/// One Rust-tagged fence that neither parses as a file nor as a statement block.
/// 一个既不能作为文件、也不能作为语句块解析的 Rust 围栏。
#[derive(Debug)]
pub struct Finding {
    /// Path relative to the workspace root, with `/` separators.
    /// 以工作区根为基准的路径，使用 `/` 分隔符。
    pub file: String,
    /// One-based line number of the opening fence.
    /// 起始围栏的行号（从 1 开始）。
    pub line: usize,
    /// The parser's message, which names the token it stopped at.
    /// 解析器的消息，其中点名了它停在哪一个词元。
    pub error: String,
}

/// Path-component prefixes the gate does not cover, because they are records.
/// 门禁不覆盖的路径分量前缀，因为它们是记录。
///
/// An audit or design document is a record of what was true when it was written,
/// and its Rust blocks are often deliberate excerpts — a `$crate` macro arm, a
/// doc-comment fragment — that were never standalone Rust. Editing such a block
/// so a new gate passes would rewrite the record, and retagging it as `text`
/// would hide that it is Rust. The living documents that a reader is expected to
/// follow (`README.md`, the crate READMEs, `docs/graft.md`, `docs/migration.md`,
/// `docs/threat-model.md`, the introduction) are all covered, and every one of
/// them parses today.
/// 审计或设计文档是"写下时事实如何"的记录，其中的 Rust 块常常是有意的摘录——一段
/// `$crate` 宏臂、一段文档注释片段——从来不是可独立成立的 Rust。为了让新门禁通过而改动这类
/// 块等于改写记录，把它们重新标成 `text` 又会掩盖"这是 Rust"。读者预期遵循的活文档
/// （`README.md`、各 crate 的 README、`docs/graft.md`、`docs/migration.md`、
/// `docs/threat-model.md`、引言）全部在覆盖范围内，且今天全部能解析。
pub const RECORD_PREFIXES: &[&str] = &["audit", "design"];

/// Every markdown file the gate covers, sorted, existing ones only.
/// 门禁覆盖的每个 markdown 文件，已排序，仅包含存在者。
pub fn markdown_files(root: &Path) -> Vec<PathBuf> {
    // Every root-level markdown file (the two READMEs, `AGENTS.md`, `CHANGELOG.md`)
    // and every markdown file under `docs/`, however deep: a document is living
    // documentation by where it is, not by how shallowly it sits.
    // 每个根级 markdown 文件（两份 README、`AGENTS.md`、`CHANGELOG.md`）以及 `docs/` 下每个
    // markdown 文件，无论多深：一份文档是不是活文档取决于它在哪，而不取决于它有多浅。
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|extension| extension == "md") {
                files.push(path);
            }
        }
    }
    collect_markdown(&root.join("docs"), &mut files);
    for directory in crate_directories(root) {
        files.push(directory.join("README.md"));
        files.push(directory.join("README.zh-CN.md"));
    }
    files.retain(|path| path.is_file() && !is_record(root, path));
    files.sort();
    files.dedup();
    files
}

/// Every markdown file under `directory`, at any depth.
/// `directory` 下每个 markdown 文件，任意深度。
fn collect_markdown(directory: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if crate::is_real_directory(&path) {
            if !crate::is_skipped_directory(&path) {
                collect_markdown(&path, files);
            }
        } else if path.extension().is_some_and(|extension| extension == "md") {
            files.push(path);
        }
    }
}

/// Whether a fence's info string names Rust.
/// 围栏的信息串是否命名 Rust。
///
/// Both halves of the gate read the same set through this one predicate. They used to
/// differ: the doc-comment half lowercased the tag and accepted `rs` as well, while the
/// markdown half compared the raw string against `rust`, so `rs` and `Rust` were covered in
/// one place and invisible in the other.
/// 门禁的两半边都通过这一个判定读同一组。它们过去并不一致：注释那半边先把 tag 小写并接受
/// `rs`，而 markdown 那半边拿原串与 `rust` 比较，于是 `rs` 与 `Rust` 在一处被覆盖、在另一处
/// 不可见。
fn names_rust(info: &str) -> bool {
    matches!(info.trim().to_ascii_lowercase().as_str(), "rust" | "rs")
}

/// One fence run: which character it is written with, how many of them, and what follows.
/// 一段围栏：用哪个字符写成、写了几个，以及它后面是什么。
///
/// CommonMark pairs a fence with a closing fence of the *same character* and *at least the
/// same length*, and the info string is what follows that whole run. Comparing a bare
/// three-character prefix instead made two shapes invisible: a ```` ``` ```` line inside a
/// `~~~rust` block was read as its close (so the broken code after that line was never
/// parsed), and a ```` ````rust ```` fence had its info string read as `` `rust ``, so the
/// block never opened. Audit `G-03`.
/// CommonMark 用**同一字符**、**长度不短于开围栏**的围栏来闭合，而 info string 是整串围栏字符
/// 之后的文本。只比三个字符的前缀让两种形状隐形：`~~~rust` 块内的一行 ```` ``` ```` 被读成它的
/// 闭合（那一行之后的坏代码从此不被解析），而 ```` ````rust ```` 围栏的 info string 被读成
/// `` `rust ``，整块从未打开。审计 `G-03`。
struct Fence {
    /// `` ` `` or `~`.
    /// `` ` `` 或 `~`。
    character: char,
    /// How many of them, which is what a closing fence has to reach.
    /// 写了几个——闭合围栏的长度要达到它。
    length: usize,
    /// What follows the run, trimmed: the info string when the line opens, empty when it closes.
    /// 围栏字符之后trimmed 的文本：开围栏时是 info string，闭合时为空。
    info: String,
}

/// The fence run `trimmed` begins, when it begins one at all.
/// `trimmed` 行首开始的那段围栏（若确实有）。
fn fence_run(trimmed: &str) -> Option<Fence> {
    let character = trimmed.chars().next()?;
    if character != '`' && character != '~' {
        return None;
    }
    let length = trimmed.chars().take_while(|run| *run == character).count();
    // A fence is at least three characters; a shorter run is inline code or prose.
    // 围栏至少三个字符；更短的串是行内代码或散文。
    if length < 3 {
        return None;
    }
    Some(Fence {
        character,
        length,
        info: trimmed[length..].trim().to_owned(),
    })
}

/// A Rust fence that is open, and what it takes to close it.
/// 一个已打开的 Rust 围栏，以及闭合它的条件。
struct OpenFence {
    /// One-based line the opening fence sat on.
    /// 开围栏所在行（从 1 开始）。
    line: usize,
    /// The code collected so far, without the fence lines.
    /// 已收集的代码，不含围栏行。
    code: String,
    /// Which character opened it, and how long its run was.
    /// 用哪个字符打开、那一串有多长。
    character: char,
    length: usize,
}

/// Whether a markdown path is a record rather than living documentation.
/// 该 markdown 路径是记录而不是活文档。
///
/// The prefix is matched against every component of the path *below `root`* — the file's
/// name and each directory it sits in — not against the file's name alone. A whole report
/// directory (`docs/audit-2026-09-28/`) is a record: with a name-only rule, a report whose
/// file name did not happen to start with `audit-` was scanned as living documentation, so
/// its Rust excerpts had to parse and its `.rs:NNN` anchors had to resolve today. The
/// reports were renamed to get the gate quiet; the boundary is what needed fixing (audit
/// `LGC-LG-53`).
/// 前缀匹配的是 **`root` 之下**路径的每一个分量——文件名与它所在的每个目录——而不只是文件名。
/// 整个报告目录（`docs/audit-2026-09-28/`）就是记录：在只看文件名的规则下，名字碰巧不以
/// `audit-` 开头的报告会被当成活文档扫描，于是它摘录的 Rust 必须能解析、它的 `.rs:NNN` 锚点必须
/// 此刻仍然成立。报告当时被改名来让门禁闭嘴；该修的是这条边界（审计 `LGC-LG-53`）。
pub fn is_record(root: &Path, path: &Path) -> bool {
    let relative = path.strip_prefix(root).unwrap_or(path);
    relative
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
            _ => None,
        })
        .any(|name| {
            RECORD_PREFIXES
                .iter()
                .any(|prefix| name.starts_with(prefix))
        })
}

/// Every fenced Rust block that does not parse, in markdown or in a doc comment.
/// 每一个无法解析的 Rust 围栏块，无论在 markdown 还是文档注释里。
///
/// Rustdoc comment examples are the other half of the same promise. Rustdoc skips
/// a block tagged `ignore`, so a macro-usage example that can never be a doctest
/// of this crate — it names the host's `crate::…` paths — is shown to every reader
/// and compiled by nobody. Those blocks are still parsed here, which is why the
/// tag `rust,ignore` matters rather than a bare `ignore`: the language has to be
/// explicit for a reader to know, and for this gate to find it.
/// Rustdoc 注释里的示例是同一个承诺的另一半。rustdoc 会跳过标了 `ignore` 的代码块，因此
/// 一个永远无法成为本 crate doctest 的宏用法示例——它命名宿主的 `crate::…` 路径——会给每个
/// 读者看到，却没有任何程序编译它。这些块仍会在这里被解析，这也正是标 `rust,ignore` 而不是
/// 裸 `ignore` 的意义：语言必须写明，读者才知道，本门禁也才找得到。
pub fn findings(root: &Path) -> Vec<Finding> {
    let mut found = markdown_findings(root);
    found.extend(doc_comment_findings(root));
    found
}

/// Every fenced Rust block in markdown that does not parse.
/// markdown 里每一个无法解析的 Rust 围栏块。
fn markdown_findings(root: &Path) -> Vec<Finding> {
    let mut found = Vec::new();
    for path in markdown_files(root) {
        let file = relative(root, &path);
        let mut open: Option<OpenFence> = None;
        for (index, line) in lines(&path).iter().enumerate() {
            let trimmed = line.trim_start();
            let fence = fence_run(trimmed);
            if let Some(open_fence) = open.as_mut() {
                // Inside a block, only a fence of the same character, at least as long, with
                // nothing after its run, closes it. Anything else is content: CommonMark has
                // two fence characters, and reading whichever one appears as the close cut
                // blocks short — a `~~~rust` block ended at a ```` ``` ```` line inside it,
                // and everything after that line was never parsed.
                // 块内只有"同一字符、长度不短、其后无内容"的围栏会闭合它。其余都是内容：CommonMark
                // 有两种围栏字符，把出现的任一种都读成闭合会把块截短——`~~~rust` 块曾在块内一行
                // ```` ``` ```` 处结束，而那一行之后的一切从此不被解析。
                let closes = fence.as_ref().is_some_and(|fence| {
                    fence.character == open_fence.character
                        && fence.length >= open_fence.length
                        && fence.info.is_empty()
                });
                if !closes {
                    open_fence.code.push_str(line);
                    open_fence.code.push('\n');
                    continue;
                }
                let finished = open.take().expect("an open fence was just matched");
                // The parser message is what makes the report actionable, so it is kept verbatim.
                // 解析器的消息让报告可操作，因此原样保留。
                if let Err(error) = parses(&finished.code) {
                    found.push(Finding {
                        file: file.clone(),
                        line: finished.line,
                        error,
                    });
                }
                continue;
            }
            let Some(fence) = fence else {
                continue;
            };
            // `rust,ignore` and friends name the same language; only the text before the
            // first comma selects it.
            // `rust,ignore` 之类命名的是同一种语言；只有第一个逗号之前的文本用于选择。
            let mut tags = fence.info.split(',');
            let language = tags.next().unwrap_or("");
            // `macro-input` is this repository's documented escape hatch for a field-list
            // excerpt that is Rust-shaped but not a file: the doc-comment half honours it,
            // and the markdown half did not, so a sanctioned tag was reported as a broken
            // block.
            // `macro-input` 是本仓库为"形状像 Rust 但不是文件的字段列表摘录"写下的逃逸口：
            // 注释那一半认它，markdown 这一半不认，于是一个被认可过的标记被报成坏块。
            if names_rust(language) && !tags.any(|tag| tag.trim() == "macro-input") {
                open = Some(OpenFence {
                    line: index + 1,
                    code: String::new(),
                    character: fence.character,
                    length: fence.length,
                });
            }
        }
        if let Some(unclosed) = open {
            // A fence that never closes is not "nothing to check": the reader sees the
            // code and the parser never gets it. The comment half of this gate already
            // flushed its open block; the markdown half did not.
            // 从不闭合的围栏不是"没有东西要检查"：读者看得到那段代码，而解析器从没拿到过它。本门禁
            // 的注释那一半已经会收尾未闭合的块，markdown 这一半此前不会。
            found.push(Finding {
                file: file.clone(),
                line: unclosed.line,
                error:
                    "unterminated Rust fence: it opens here and the file ends without a closing ```"
                        .to_owned(),
            });
        }
    }
    found
}

/// Every fenced Rust block in a `///` or `//!` doc comment that does not parse.
/// `///` 或 `//!` 文档注释里每一个无法解析的 Rust 围栏块。
///
/// Test-only files are skipped: their comments are not documentation a reader is
/// shown, and several of them carry whole source files inside string literals,
/// whose lines would otherwise be mistaken for doc comments.
/// 仅测试文件被跳过：它们的注释不是给读者看的文档，而且其中好几个把整份源码放进字符串字面量
/// 里，那些行否则会被误认成文档注释。
fn doc_comment_findings(root: &Path) -> Vec<Finding> {
    let mut found = Vec::new();
    for directory in crate_directories(root) {
        for path in rust_sources(&directory.join("src")) {
            // The same identity question the size gate asks, answered by the same rule: location and
            // name are hints, and the mount behind `#[cfg(test)]` is proof. A hint misses
            // `tree/graft_ops/fixtures.rs` — mounted as a test, named as if it were not — and this
            // gate then checked its comments as documentation a reader is shown (audit `G-05`).
            // 与尺寸门禁同一个身份问题，用同一条规则回答：位置与名字是提示，而挂在 `#[cfg(test)]`
            // 之后是证明。提示会漏掉 `tree/graft_ops/fixtures.rs`——它作为测试挂载、名字却不像——于是
            // 这道门禁把它的注释当读者文档检查（审计 `G-05`）。
            if looks_test_only(&path) || is_mounted_as_test(&directory, &path) {
                continue;
            }
            let file = relative(root, &path);
            // The block a doc comment opened, with the line its fence sat on.
            // 文档注释打开的那个块，以及它的围栏所在行号。
            let mut open: Option<OpenFence> = None;
            let finish = |open: Option<OpenFence>, found: &mut Vec<Finding>| {
                if let Some(finished) = open
                    && let Err(error) = parses(&finished.code)
                {
                    found.push(Finding {
                        file: file.clone(),
                        line: finished.line,
                        error,
                    });
                }
            };
            for (index, line) in lines(&path).iter().enumerate() {
                let trimmed = line.trim_start();
                let comment = trimmed
                    .strip_prefix("///")
                    .or_else(|| trimmed.strip_prefix("//!"));
                let Some(comment) = comment else {
                    // Any other line ends the block, exactly as rustdoc ends it.
                    // 任何其他行都会结束该块，与 rustdoc 的行为一致。
                    finish(open.take(), &mut found);
                    continue;
                };
                let text = comment.strip_prefix(' ').unwrap_or(comment);
                // The same CommonMark pairing the markdown half uses: rustdoc reads these
                // fences, so a `~~~rust` block here is a block too, and only a fence of the
                // same character and at least the same length closes a block.
                // 与 markdown 那半边相同的 CommonMark 配对：rustdoc 读的正是这些围栏，因此这里一段
                // `~~~rust` 也是一个块，而只有同一字符、长度不短的围栏才会闭合一个块。
                let fence = fence_run(text.trim_start());
                if let Some(open_fence) = open.as_mut() {
                    let closes = fence.as_ref().is_some_and(|fence| {
                        fence.character == open_fence.character
                            && fence.length >= open_fence.length
                            && fence.info.is_empty()
                    });
                    if !closes {
                        open_fence.code.push_str(text);
                        open_fence.code.push('\n');
                        continue;
                    }
                    let finished = open.take().expect("an open fence was just matched");
                    finish(Some(finished), &mut found);
                }
                let Some(fence) = fence else {
                    continue;
                };
                let mut tags = fence.info.split(',').map(str::trim);
                if !names_rust(tags.next().unwrap_or("")) {
                    continue;
                }
                // `macro-input` marks an excerpt whose shape is decided by a
                // macro matcher rather than by Rust: `name: { zh: … }` is how
                // an author writes a field inside a face macro, and a bare
                // brace in field-value position is not valid Rust on its own.
                // Those shapes are pinned by the macro front end's own tests
                // (`run_method/tests/face_*.rs`); a documentation gate cannot
                // parse them, and pretending otherwise would only hide them.
                // `macro-input` 标出的是形状由宏匹配器而非 Rust 决定的摘录：
                // `name: { zh: … }` 是作者在注册面宏里写字段的方式，而字段值位置上的
                // 裸花括号本身不是合法 Rust。那些形状由宏前端自己的测试钉住
                // （`run_method/tests/face_*.rs`）；文档门禁解析不了它们，假装能解析
                // 只会把它们藏起来。
                if tags.any(|tag| tag == "macro-input") {
                    continue;
                }
                open = Some(OpenFence {
                    line: index + 1,
                    code: String::new(),
                    character: fence.character,
                    length: fence.length,
                });
            }
            finish(open, &mut found);
        }
    }
    found
}

/// Whether a fence's contents parse as a file, a statement block, or fields.
/// 围栏内容是否能作为文件、语句块或字段列表解析。
fn parses(code: &str) -> Result<(), String> {
    // A stack overflow is not a catchable panic, and `syn` is recursive descent
    // with no depth guard of its own, so a fence that nested past the kernel's
    // measurement would take down the whole gate battery instead of failing it.
    // The measurement is the kernel's `guard_nesting`, which is also what the
    // kernel's own parse entries use; keeping one copy is what stops the two from
    // drifting apart.
    // 栈溢出不是可捕获的 panic，而 `syn` 是无自带深度守卫的递归下降解析器，因此嵌套越过内核
    // 度量的围栏会带走整套门禁而不是让它失败。度量来自内核的 `guard_nesting`，内核自己的解析
    // 入口用的也是它；只留一份正是防止两者漂移的办法。
    nichlink::registry_core::syntax::guard_nesting(code).map_err(|error| error.to_string())?;
    // A fence whose whole body is one identifier is a word, not an excerpt, and *every*
    // reading below accepts it: a path expression is a valid statement tail, so
    // `{ TODO }` parses. The audit measured `TODO` passing while `fix this later` did not;
    // the cause is the statement reading rather than the field list. Tag such a block
    // `text`.
    // 整段只有一个标识符的围栏是一个单词而不是摘录，而下面**每一种**读法都接受它：路径表达式是
    // 合法的语句尾，因此 `{ TODO }` 能解析。审计实测 `TODO` 能过而 `fix this later` 不能，成因是
    // 语句读法而不是字段列表。这样的块请标成 `text`。
    if is_one_identifier(code) {
        return Err(
            "a single identifier is not a Rust excerpt; tag the block as `text`".to_owned(),
        );
    }
    if syn::parse_file(code).is_ok() {
        return Ok(());
    }
    // Most excerpts are statements rather than items, so a block is tried second
    // and its message is reported only when every reading fails.
    // 大多数摘录是语句而不是项，因此第二次尝试按块解析，只有所有读法都失败时才报告它的消息。
    let wrapped = format!("{{\n{code}\n}}");
    let block_error = match syn::parse_str::<syn::Block>(&wrapped) {
        Ok(_) => return Ok(()),
        Err(error) => error.to_string(),
    };
    // The last reading is a struct-literal field list, which is how the face
    // macros' examples are written: `flow_provider: crate::ControlHandle` is
    // neither a file nor a statement, and retagging it as `text` would hide that
    // it is Rust. The type name is a fresh identifier — only the syntax is
    // checked, and a typo in brackets, commas or nesting is still a failure.
    // 最后一种读法是结构体字面量的字段列表，注册面宏的示例正是这么写的：
    // `flow_provider: crate::ControlHandle` 既不是文件也不是语句，而把它重新标成 `text`
    // 会掩盖它是 Rust 这件事。类型名是一个新鲜标识符——这里只检查语法，而括号、逗号或嵌套的
    // 笔误仍然会失败。
    let as_fields = format!("fn excerpt() {{ let _ = Excerpt {{ {code} }}; }}");
    if syn::parse_file(&as_fields).is_ok() {
        return Ok(());
    }
    Err(block_error)
}

/// Whether the excerpt is one bare identifier and nothing else.
/// 该摘录是否只有一个裸标识符。
fn is_one_identifier(code: &str) -> bool {
    let trimmed = code.trim();
    !trimmed.is_empty()
        && trimmed
            .chars()
            .next()
            .is_some_and(|first| first.is_alphabetic() || first == '_')
        && trimmed
            .chars()
            .all(|character| character.is_alphanumeric() || character == '_')
}

#[cfg(test)]
#[path = "doc_blocks_tests.rs"]
mod doc_blocks_tests;
