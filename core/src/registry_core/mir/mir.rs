//! Static MIR candidates and evidence-aware call relation merging.
//! Pure protocol logic: parsing rustc `-Zunpretty=mir` text and JSONL
//! artifacts, and merging static candidates with live edges supplied by the
//! caller. Never treats static candidates as live facts.
//! 静态 MIR 候选与带证据等级的调用关系归并。
//! 纯协议逻辑：解析 rustc `-Zunpretty=mir` 文本与 JSONL artifact，
//! 并把静态候选与调用方提供的 live 边归并；绝不把静态候选当成 live 事实。
//!
//! The vocabulary lives in `model`, the textual parser in `text`, the JSONL
//! parser in `jsonl`, artifact rendering in `render`, and the merge in
//! `merge`. This root mounts them and re-exports every kernel path unchanged.
//! 词汇表在 `model`，文本解析器在 `text`，JSONL 解析器在 `jsonl`，
//! artifact 渲染在 `render`，归并在 `merge`。本模块根挂载它们并原样重导出
//! 每个 kernel 路径。

#[path = "jsonl.rs"]
mod jsonl;
#[path = "merge.rs"]
mod merge;
#[path = "model.rs"]
mod model;
#[path = "render.rs"]
mod render;
#[path = "text.rs"]
mod text;

pub use merge::{merge_call_relations, same_symbol};
pub use model::{CallEvidence, CallRelation, MirCall, MirGraph, MirLocal, MirParseError};
