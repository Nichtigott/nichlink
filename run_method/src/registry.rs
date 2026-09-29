//! Historical-path shim: `nichlink_run_method::registry`.
//! 历史路径 shim：`nichlink_run_method::registry`。
//!
//! Registry tree. The implementation lives in the kernel `tree` module; this shim
//! keeps the historical path alive for hosts (audit `NAM-10`).
//! 注册树。实现本体在 kernel 的 `tree` 模块；本 shim 为宿主保留历史路径（审计 `NAM-10`）。

pub use nichlink_kernel::tree;
pub use nichlink_kernel::tree::*;
