//! NichLink 示例：README 里的 Control / Button 两层树，作为一个真实宿主 crate。
//! NichLink example: the README Control/Button two-level tree as a real host.
//!
//! 这里是整个 crate 唯一的构建接线点。`host!()` 引入构建期生成的注册计划；
//! 注册面代码保持普通 Rust，父级不维护子对象清单。
//! This is the crate's single build wiring point. `host!()` pulls in the plan the
//! build step generated; face code stays ordinary Rust and no parent keeps a
//! child roster.

nichlink_run_method::host!();

// `host!()` re-exports the kernel at the crate root, so the protocol nouns are
// already in scope here. Importing them again would shadow that public
// re-export with a private one.
// `host!()` 已在 crate 根重导出 kernel，协议名词因此已在作用域内；再显式导入
// 会用私有项遮蔽那个公开重导出。

/// 入口需要引用构建期宏，这里把它一并重导出。
/// Re-export the build-time macro so the entry can name it.
pub use nichlink_run_method::static_graft_plan;

/// 这个示例的宿主身份。graft 要求覆盖双方共享同一个 framework。
/// The example's host identity. A graft requires both sides to share it.
pub const FRAMEWORK: FrameworkId = FrameworkId::new("nichlink.example.control-button");

/// 按框架和包命名空间装配这个示例的注册机。
/// Assemble the example's registry from its framework and package namespace.
pub fn base_registry() -> Registry {
    let mut registry = Registry::root_for_namespace(FRAMEWORK, env!("CARGO_PKG_NAME"));
    registry
        .register_all(&registrations())
        .expect("example faces register");
    registry
}

/// 打印注册树的逻辑路径，供二进制和集成测试共用。
/// Print the registry tree's logical paths; shared by the binary and the tests.
pub fn outline() -> Vec<String> {
    let registry = base_registry();
    let mut rows = registry
        .depth_first()
        .iter()
        .map(|info| {
            format!(
                "{}  kind={}  source={}",
                registry.path_for(info.id).unwrap_or_default(),
                info.kind,
                info.source.file
            )
        })
        .collect::<Vec<_>>();
    rows.sort();
    rows
}
