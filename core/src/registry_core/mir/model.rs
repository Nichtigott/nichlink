//! Shared MIR vocabulary: parsed candidates, evidence-aware relations, errors.
//! 共享 MIR 词汇：已解析候选、带证据等级的关系、错误。
//!
//! The parsers live beside this page: `super::text` reads rustc
//! `-Zunpretty=mir` text, `super::jsonl` reads the compact JSONL artifact,
//! `super::render` writes both artifacts back, and `super::merge` merges
//! static candidates with live edges.
//! 解析器位于本页旁边：`super::text` 读取 rustc `-Zunpretty=mir` 文本，
//! `super::jsonl` 读取紧凑 JSONL artifact，`super::render` 把两种 artifact 写回，
//! `super::merge` 将静态候选与 live 边归并。

use std::collections::BTreeSet;
use std::fmt;

use crate::registry_core::declaration::{EvidenceKind, SourceLocation};

/// One direct call sighting extracted from a MIR artifact.
/// 从 MIR artifact 中提取到的一条直接调用观测。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MirCall {
    /// Function symbol the call appears in.
    /// 调用所在的函数符号。
    pub caller: String,
    /// Function symbol being called.
    /// 被调用的函数符号。
    pub callee: String,
    /// 1-based position of this record within the parsed MIR, not a source line.
    /// 该记录在已解析 MIR 内以 1 起始的位置，不是源码行号。
    pub mir_line: usize,
}

/// One local binding sighting extracted from a MIR artifact.
/// 从 MIR artifact 中提取到的一条局部绑定观测。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MirLocal {
    /// Function symbol owning the local.
    /// 拥有该局部变量的函数符号。
    pub function: String,
    /// Local binding name as written in MIR, such as `_1`.
    /// MIR 中书写的局部绑定名，例如 `_1`。
    pub name: String,
    /// Declared type text of the binding.
    /// 该绑定的声明类型文本。
    pub type_name: String,
    /// 1-based position of this record within the parsed MIR, not a source line.
    /// 该记录在已解析 MIR 内以 1 起始的位置，不是源码行号。
    pub mir_line: usize,
}

/// Unified evidence attached to one logical call relation.
/// 一条逻辑调用关系携带的统一证据等级。
pub type CallEvidence = EvidenceKind;

/// One call edge normalized across runtime, MIR, and source evidence.
/// 跨运行时、MIR、源码证据统一表示的一条调用边。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallRelation {
    /// Function symbol making the call.
    /// 发起调用的函数符号。
    pub caller: String,
    /// Function symbol being called.
    /// 被调用的函数符号。
    pub callee: String,
    /// Strongest evidence supporting this relation: live over MIR.
    /// 支持该关系的最高证据等级：Live 优先于 MIR。
    pub evidence: EvidenceKind,
    /// Source location of the callee, when known.
    /// 已知时，被调用方的源码位置。
    pub source: Option<SourceLocation>,
    /// MIR record position for a MIR-only relation; `None` for a live edge.
    /// 仅由 MIR 支持时为 MIR 记录位置；live 边为 `None`。
    pub mir_line: Option<usize>,
    /// Runtime frame id of the caller; `None` for a static-only relation.
    /// 调用方的运行期帧 id；仅静态关系为 `None`。
    pub caller_frame: Option<u64>,
    /// Runtime frame id of the callee; `None` for a static-only relation.
    /// 被调用方的运行期帧 id；仅静态关系为 `None`。
    pub callee_frame: Option<u64>,
}

impl CallRelation {
    /// A relation whose two ends are known but whose provenance is only the text
    /// of the call site: no MIR record, no runtime frame.
    /// 两端已知、但来源只是调用点文本的关系：没有 MIR 记录，也没有运行期帧。
    ///
    /// Studio's source scan produces these, which is why the type has to be
    /// constructible without inventing a MIR line or a frame id.
    /// Studio 的源码扫描产生的正是它们，因此该类型必须能在不编造 MIR 行号或帧 id 的情况下构造。
    pub fn from_symbols(
        caller: impl Into<String>,
        callee: impl Into<String>,
        evidence: EvidenceKind,
    ) -> Self {
        Self {
            caller: caller.into(),
            callee: callee.into(),
            evidence,
            source: None,
            mir_line: None,
            caller_frame: None,
            callee_frame: None,
        }
    }
}

/// Every candidate one MIR artifact offered, before any merge.
/// 一份 MIR artifact 提供的全部候选，尚未归并。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MirGraph {
    /// Function symbols the artifact declared.
    /// artifact 声明过的函数符号。
    pub functions: BTreeSet<String>,
    /// Direct call sightings, in artifact order.
    /// 直接调用观测，按 artifact 顺序。
    pub calls: Vec<MirCall>,
    /// Local binding sightings, in artifact order.
    /// 局部绑定观测，按 artifact 顺序。
    pub locals: Vec<MirLocal>,
}

/// Why one line of a MIR artifact could not be read.
/// MIR artifact 某一行无法读取的原因。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MirParseError {
    /// 1-based line of the offending input.
    /// 出错输入以 1 起始的行号。
    pub line: usize,
    /// Description of what the line got wrong.
    /// 对该行错误之处的描述。
    pub message: String,
}

impl fmt::Display for MirParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "MIR graph line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for MirParseError {}
