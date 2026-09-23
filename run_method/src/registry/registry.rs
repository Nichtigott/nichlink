//! Registry tree. The implementation lives in the kernel `tree` module;
//! this shim keeps the historical `nichlink_run_method::registry` path.
//! 注册树。实现本体在 kernel 的 `tree` 模块；
//! 本 shim 保留 `nichlink_run_method::registry` 历史路径。

pub use nichlink::tree;
pub use nichlink::tree::*;
