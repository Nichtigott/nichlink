//! Registry tree: the one `Registry` type plus the pure operations over it.
//! 注册树：唯一的 `Registry` 类型与作用于其上的纯操作。
//!
//! The file tree mirrors the module tree: `tree/tree.rs` is this module, and
//! every child is a `目录/同名.rs` file it declares with `#[path]`.
//! 文件树与模块树一致：`tree/tree.rs` 就是本模块，每个子模块都是它用 `#[path]`
//! 声明的 `目录/同名.rs`。

#[path = "entry_pages/entry_pages.rs"]
mod entry_pages;
#[path = "header/header.rs"]
mod header;

#[path = "connector/connector.rs"]
pub mod connector;
#[path = "graft_ops/graft_ops.rs"]
pub mod graft_ops;
#[path = "index/index.rs"]
pub mod index;
#[path = "inspection/inspection.rs"]
pub mod inspection;
#[path = "metadata/metadata.rs"]
pub mod metadata;
#[path = "query/query.rs"]
pub mod query;
#[path = "transaction/transaction.rs"]
pub mod transaction;

#[path = "registry.rs"]
mod registry;
pub use registry::*;
