//! Registry tree: the one `Registry` type plus the pure operations over it.
//! 注册树：唯一的 `Registry` 类型与作用于其上的纯操作。
//!
//! The file tree mirrors the module tree: `tree/tree.rs` is this module, and
//! every child is declared with `#[path]`. A child whose only file is its own
//! source sits beside this one (`tree/<x>.rs`, the flattened form the A1 pass
//! produced); a child that owns more than one file keeps its directory
//! (`tree/graft_ops/graft_ops.rs`, `tree/ports/ports.rs`).
//! 文件树与模块树一致：`tree/tree.rs` 就是本模块，每个子模块都用 `#[path]` 声明。
//! 只有一个文件的子模块与本文件并列（`tree/<x>.rs`，即 A1 收平后的形态）；拥有
//! 一个以上文件的子模块保留自己的目录（`tree/graft_ops/graft_ops.rs`、
//! `tree/ports/ports.rs`）。

#[path = "entry_pages.rs"]
mod entry_pages;
#[path = "header.rs"]
mod header;

#[path = "connector.rs"]
pub mod connector;
#[path = "graft_ops/graft_ops.rs"]
pub mod graft_ops;
#[path = "index.rs"]
pub mod index;
#[path = "inspection.rs"]
pub mod inspection;
#[path = "metadata.rs"]
pub mod metadata;
#[path = "ports/ports.rs"]
pub mod ports;
#[path = "query.rs"]
pub mod query;
#[path = "transaction.rs"]
pub mod transaction;

#[path = "registry.rs"]
mod registry;
pub use registry::*;
