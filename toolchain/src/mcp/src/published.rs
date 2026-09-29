//! The build's published records for one member, read as a tree answer.
//! 一个成员已发布的记录，作为一份树级答案来读。
//!
//! The build already derives this tree once and writes it to
//! `<package>/target/nichlink/out`; re-deriving it per member is what made a
//! workspace-rooted call cost the sum of every member's source walk. This module
//! reads those files instead — through `build_method`'s own readers, so the
//! formats keep their one parser and it is not here — and hands the answer on in
//! the record's own vocabulary: the face rows `pruning_manifest.tsv` carries
//! (`node`, `source`, `symbol`), the scope verdict `source_scope.tsv` carries,
//! and the freshness token.
//! 构建已经把这棵树推导过一次并写进 `<package>/target/nichlink/out`；逐成员重新推导正是让
//! 工作区根上的一次调用等于每个成员源码遍历之和的原因。本模块改为读那些文件——经
//! `build_method` 自己的读取器，因此各格式保持它们唯一的解析器、而且不在本模块——并用记录
//! 自己的词汇交出答案：`pruning_manifest.tsv` 携带的面行（`node`、`source`、`symbol`）、
//! `source_scope.tsv` 携带的作用域结论，以及新鲜度凭据。
//!
//! What the record does **not** carry is as load-bearing as what it does: `path`,
//! `kind`, `registry_name`, `parent` and `parent_resolved` are derived facts, not
//! published ones, so any answer that names a face by logical path still has to
//! read the sources. That is why [`Publication`] is a reading of a *tree* rather
//! than a replacement for the derivation, and why every answer built on it says
//! which of the two it used.
//! 记录**没有**什么与它有什么同样承重：`path`、`kind`、`registry_name`、`parent` 与
//! `parent_resolved` 是推导出来的事实而不是发布出来的事实，因此任何按逻辑路径点名一个面的答案
//! 仍然必须读源码。这就是为什么 [`Publication`] 是对**一棵树**的读数而不是推导的替代品，也是
//! 为什么每一份由它构成的答案都会说出自己用的是哪一种。
//!
//! Never glob `target/debug/build/<pkg>-<hash>/out/`: one package measured 134
//! hashed copies with the old ones still there, so a glob can hand back a stale
//! record. The published path is `<package>/target/nichlink/out` and nothing else.
//! 绝不要 glob `target/debug/build/<pkg>-<hash>/out/`：同一个包实测有 134 份哈希分身、旧的还
//! 在，因此 glob 可能取到陈旧记录。已发布的路径就是 `<package>/target/nichlink/out`，不是别的。

#[cfg(test)]
use std::cell::Cell;
use std::path::{Path, PathBuf};

use crate::build_time::{
    BuildScopeView, PruningRow, build_output_is_current, read_build_scope, read_pruning_manifest,
};
use nichlink_kernel::identity::NodeId;

use crate::mcp::build_evidence::out_dir;

/// The scope reason the build writes when its walk found no registration face.
/// 遍历没有找到注册面时构建写下的作用域原因。
pub(crate) const NO_FACE_REASON: &str = "no-registration-face";

/// What an answer prints when the face manifest was unreadable but the scope was not.
/// 作用域可读、面清单读不了时，答案打印的内容。
///
/// Not `no faces`: the scope file was readable, so this member *was* built, and the
/// face half is one file away from being answerable. Reporting it as faceless would
/// turn a damaged record into a claim about the package, which is the failure the two
/// states are kept apart to prevent. One spelling, in the module that produces the
/// state, so the roster row and the report cannot drift apart.
/// 而不是 `no faces`：作用域文件可读，因此这个成员**确实**被构建过，而面那一半只差一个文件就能
/// 作答。把它报成没有面会把一份损坏的记录变成关于这个包的断言，而区分这两个状态正是要阻止这件事。
/// 只有一份词形，放在产出该状态的模块里，因此普查行与报告不会漂开。
pub(crate) const FACES_UNKNOWN: &str =
    "faces unknown (no readable pruning_manifest.tsv; run `nichlink check`)";

/// What one member's own `target/nichlink/out` says.
/// 一个成员自己的 `target/nichlink/out` 说了什么。
pub(crate) struct PublishedTree {
    /// The directory the reading came from, named in every answer built on it.
    /// 读数来自哪个目录；每一份由它构成的答案都会点名它。
    pub(crate) out: PathBuf,
    /// The published `discovery.fingerprint`, or `None` when the build removed it.
    /// 已发布的 `discovery.fingerprint`；构建移除它时为 `None`。
    pub(crate) fingerprint: Option<String>,
    /// Whether the published output still describes these sources.
    /// 已发布的产物是否仍在描述这批源码。
    pub(crate) current: bool,
    /// The scope `source_scope.tsv` recorded.
    /// `source_scope.tsv` 记录的作用域。
    pub(crate) scope: BuildScopeView,
    /// The face rows `pruning_manifest.tsv` recorded, or `None` when it was unreadable.
    /// `pruning_manifest.tsv` 记录的面行；读不了时为 `None`。
    pub(crate) pruning: Option<Vec<PruningRow>>,
}

impl PublishedTree {
    /// The word for how fresh the published output is.
    /// 已发布产物新鲜度的那个词。
    ///
    /// The spelling lives in `build_evidence` for every report that asks the same
    /// question, and it is spelled the same way here: one rule, one word, no drift
    /// between two tools describing one build.
    /// 问同一个问题的每份报告，其词形都住在 `build_evidence`，这里也照那个词形写：一条规则、一个
    /// 词，描述同一次构建的两个工具之间不会漂移。
    pub(crate) fn freshness(&self) -> &'static str {
        if self.current {
            "current"
        } else {
            "stale (run `nichlink check`)"
        }
    }

    /// The one line every published answer opens its tree half with.
    /// 每一份发布答案的树那一半都以此行开头。
    ///
    /// The fingerprint is named rather than summarized: it is the token a reader
    /// compares against the next build, and `absent` is itself a fact — the
    /// pipeline removes it when validation fails.
    /// 指纹是被点名而不是被概括的：它是读取方与下一次构建对照的凭据，而 `absent` 本身就是个事实
    /// ——校验失败时管线会把它移除。
    pub(crate) fn evidence_line(&self) -> String {
        format!(
            "tree published from {} (discovery.fingerprint {})\n",
            self.out.display(),
            self.fingerprint.as_deref().unwrap_or("absent"),
        )
    }

    /// The face rows this record carries.
    /// 这份记录携带的面行。
    pub(crate) fn faces(&self) -> &[PruningRow] {
        self.pruning.as_deref().unwrap_or_default()
    }

    /// Whether the face manifest could not be read at all.
    /// 面清单是否根本读不了。
    ///
    /// "The build tracked no face" and "nothing here can say" are different
    /// answers, and a reader that flattened them would report an unreadable file as
    /// a framework crate. The roster and the report both print this state as
    /// `unknown` rather than as zero.
    /// "构建没有跟踪任何面"与"这里说不出来"是不同的答案，把它们抹平的读取方会把一个读不了的文件
    /// 报成框架 crate。普查与报告都把这个状态印成 `unknown` 而不是零。
    pub(crate) fn faces_unknown(&self) -> bool {
        self.pruning.is_none()
    }

    /// Whether the scope this record carries selected a face by identity or source.
    /// 这份记录携带的作用域是否按身份或源码选中了一个面。
    pub(crate) fn selected(&self, row: &PruningRow) -> bool {
        self.scope.all
            || self.scope.selected_ids.contains(&row.id)
            || self.scope.selected_sources.contains(&row.source)
    }

    /// Whether the record itself says this package declares no registration face.
    /// 记录本身是否说这个包不声明任何注册面。
    ///
    /// The build writes `# result all` with the reason `no-registration-face`
    /// when its walk found nothing, and that is an answer rather than a missing
    /// one. A `# result all` for any *other* reason means "every face is in
    /// scope" and the record lists no rows because it had no need to: that is not
    /// the same claim, and reading it as an empty tree is the mistake this method
    /// exists to prevent. A narrowed scope that lists no face at all is the third
    /// shape, and there the manifest is the witness: an unreadable manifest means
    /// "unknown", not "none".
    /// 遍历什么也没找到时，构建写下 `# result all` 与原因 `no-registration-face`，那是一份答案
    /// 而不是缺失的答案。任何**别的**原因下的 `# result all` 意思是"每个面都在作用域内"，记录
    /// 不列行是因为它没必要列：那不是同一个断言，而把它读成一棵空树正是本方法要阻止的错误。
    /// 第三种形状是"收窄了作用域却一个面也没列"，那里的证人是清单：清单读不了意味着"未知"而不是
    /// "一个都没有"。
    pub(crate) fn records_no_faces(&self) -> bool {
        if self.scope.all {
            self.scope.reason.as_deref() == Some(NO_FACE_REASON)
        } else {
            self.scope.selected_ids.is_empty()
                && self.pruning.as_ref().is_some_and(|rows| rows.is_empty())
        }
    }

    /// The faces this record has, by identity: what an ownership question can be
    /// answered from without reading a source file.
    /// 这份记录拥有的面，按身份：归属问题可以据此作答而不读任何源码文件。
    pub(crate) fn has_identity(&self, id: NodeId) -> bool {
        self.faces().iter().any(|row| row.id == id)
    }
}

/// What a member's own `target/nichlink/out` holds.
/// 一个成员自己的 `target/nichlink/out` 里有什么。
pub(crate) enum Publication {
    /// `source_scope.tsv` was readable, so this member has published records.
    /// `source_scope.tsv` 可读，因此这个成员有已发布的记录。
    Published(PublishedTree),
    /// No readable `source_scope.tsv`; the reader's own reason.
    /// `source_scope.tsv` 读不了；读取方自己给出的原因。
    NotBuilt(String),
}

/// Read one member's published records, or say why there are none.
/// 读取一个成员的已发布记录，或说出为什么没有。
///
/// `source_scope.tsv` is the token: it is the file the scope reader owns and the
/// first thing every build publishes, so its absence means "this member was never
/// built" rather than "this member has no faces". The two are different answers
/// and the caller spells them differently.
/// `source_scope.tsv` 就是那枚凭据：它是作用域读取方拥有的文件，也是每次构建最先发布的东西，
/// 因此它缺失意味着"这个成员从未被构建"，而不是"这个成员没有面"。两者是不同的答案，调用方也
/// 用不同的话说出来。
pub(crate) fn read(root: &Path) -> Publication {
    let out = out_dir(root);
    let scope = match read_build_scope(&out) {
        Ok(scope) => scope,
        Err(reason) => return Publication::NotBuilt(reason),
    };
    let fingerprint = std::fs::read_to_string(out.join("discovery.fingerprint"))
        .ok()
        .map(|token| token.trim().to_owned())
        .filter(|token| !token.is_empty());
    // An unreadable pruning manifest is `None` rather than an empty list: "the
    // build tracked no face" and "nothing here can say" are different answers, and
    // a merged roster that flattened them would report a package as faceless
    // because a file was missing.
    // 读不了的修剪清单是 `None` 而不是空清单："构建没有跟踪任何面"与"这里说不出来"是不同的答案，
    // 把它们抹平的合并普查会因为一个文件缺失而把一个包报成没有面。
    let pruning = read_pruning_manifest(&out).ok();
    Publication::Published(PublishedTree {
        current: build_output_is_current(root, &out),
        out,
        fingerprint,
        scope,
        pruning,
    })
}

#[cfg(test)]
thread_local! {
    /// How many times this thread derived a package's faces.
    /// 本线程推导过多少次某个包的面。
    ///
    /// A published answer claims something observable — that no face derivation
    /// happened for that member — and a claim nobody can check is a comment. The
    /// count is thread-local so a test that reads it is not racing the tests
    /// `cargo test` runs beside it on other threads, and it is test-only because
    /// nothing in a shipped bridge reads it: the claim it backs is a test's.
    /// 发布答案声称了一件可观测的事——那个成员没有发生面推导——而没人能核对的声称只是一句注释。
    /// 计数是线程局部的，因此读它的测试不会与 `cargo test` 在其它线程上并行跑的测试抢；它只存在于
    /// 测试中，因为出厂的桥里没有东西读它：它支撑的那项声称属于测试。
    static DERIVATIONS: Cell<usize> = const { Cell::new(0) };
}

/// Record that one package's faces were derived from source text.
/// 记下一次"某个包的面由源码文本推导出来"。
///
/// Test-only, and that is the whole point: [`derivations`] is what makes "this
/// answer read the published records" checkable, and the caller in `resolve` is
/// gated the same way.
/// 仅供测试，而这正是要点：[`derivations`] 让"这份答案读的是已发布记录"可被核对，而 `resolve`
/// 里的调用方以同样的方式门控。
#[cfg(test)]
pub(crate) fn note_derivation() {
    DERIVATIONS.with(|count| count.set(count.get() + 1));
}

/// How many derivations this thread has performed.
/// 本线程执行过多少次推导。
#[cfg(test)]
pub(crate) fn derivations() -> usize {
    DERIVATIONS.with(Cell::get)
}

#[cfg(test)]
#[path = "published_tests.rs"]
mod published_tests;
