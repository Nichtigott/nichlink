//! Plugin manifest metadata.
//! 插件 manifest 元数据。
//!
//! `PluginManifest` lives in the kernel crate because registration
//! declarations carry it; it is re-exported here for compatibility.
//! PluginManifest 定义在 kernel（注册声明持有它），此处为兼容而重导出。

pub use nichlink::PluginManifest;
