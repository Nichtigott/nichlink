//! Plugin protocol. The implementation lives in the kernel `plugin` module;
//! this shim keeps the historical `nichlink_run_method::plugin` path.
//! 插件协议。实现本体在 kernel 的 `plugin` 模块；
//! 本 shim 保留 `nichlink_run_method::plugin` 历史路径。

pub use nichlink::plugin;
pub use nichlink::plugin::*;
