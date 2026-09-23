//! Field dictionary shared by the Studio form and the file authoring API.
// Studio 表单与文件创作 API 共用的字段词典。
//
// The field order mirrors every field accepted by a parent-specific object macro.
// Keeping one shared order prevents Add/Edit from silently dropping metadata.
// 字段顺序覆盖父级专属 object 宏的全部可编辑字段；统一顺序可避免
// Add/Edit 静默丢失注册面信息。
/// The 30 registration-face fields in the order the object macro accepts them.
/// object 宏接受的 30 个注册面字段，按接受顺序排列。
pub const FACE_FIELD_NAMES: [&str; 30] = [
    "parent",
    "module",
    "needs registry",
    "tree slot",
    "child structure rule",
    "allowed dependencies",
    "parts type",
    "exports",
    "Rust type",
    "display name zh",
    "display name en",
    "summary zh",
    "summary en",
    "preset type",
    "parameter metadata",
    "handle type",
    "stable identity",
    "external source note",
    "rule source",
    "handle trait labels",
    "handle trait paths",
    "parts trait labels",
    "requires",
    "provides",
    "expected object output",
    "actual object output",
    "runtime checks",
    "graft flow contract",
    "flow provider type",
    "parts trait paths",
];

/// How many registration-face fields the dictionary holds.
/// 字段词典容纳的注册面字段数量。
pub const FACE_FIELD_COUNT: usize = FACE_FIELD_NAMES.len();
