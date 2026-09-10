//! Build-time syntax parsing. The parsers live in the kernel `syntax`
//! feature; this module keeps the historical `crate::syntax` import paths.
//! 构建期语法解析。解析器本体在 kernel 的 `syntax` feature 中；
//! 本模块保留 `crate::syntax` 历史导入路径。

pub use nichlink::registry_core::syntax::{
    FaceSyntax, GraftSyntax, ParentSyntax, application_entries, graft_entries, parse_face,
    source_references,
};
