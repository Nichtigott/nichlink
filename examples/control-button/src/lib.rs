//! NichLink 示例：README 里的 Control / Button 两层树，作为一个真实宿主库。
//! NichLink example: the README Control/Button two-level tree as a real host
//! library.
//!
//! 整个 crate 只有这里一处构建接线。`host!()` 引入构建期生成的注册计划；
//! 注册面代码保持普通 Rust，父级不维护子对象清单。
//! The crate has one build wiring point. `host!()` pulls in the plan the build
//! step generated; face code stays ordinary Rust and no parent keeps a child
//! roster.

nichlink_run_method::host!();

// 这个 crate 自己调用 `host!()`，所以类型化 graft 计划里的 `crate::...` 与生成
// 树解析到同一个 crate。宿主如果把库和二进制分开，计划必须写在调用 `host!()`
// 的那一个里；写在另一个 crate 里的 Rust 路径无法在这里解析。
// This crate calls `host!()` itself, so `crate::...` in a typed graft plan
// resolves in the same crate as the generated tree. A host that splits a library
// and a binary must keep the plan in whichever one calls `host!()`.

// `host!()` 已在 crate 根重导出 kernel，协议名词因此已在作用域内；再显式导入
// 会用私有项遮蔽那个公开重导出。
// `host!()` re-exports the kernel at the crate root, so the protocol nouns are
// already in scope; importing them again would shadow that public re-export.

/// 这个示例的宿主身份。graft 要求覆盖双方共享同一个 framework。
/// The example's host identity. A graft requires both sides to share it.
pub const FRAMEWORK: FrameworkId = FrameworkId::new("nichlink.example.control-button");

// 宿主入口的 graft 计划，用**类型化**写法：两侧都是指向真实注册面的 Rust 路径，
// 因此编译器与编辑器都能解析它们——写在 `cut(` 之后会补全宿主注册面路径，
// 写在 `graft(` 之后会补全外部 crate 路径。代价是外部实现必须被静态链接进来。
// The host's graft plan in the **typed** form: both sides are Rust paths to real
// faces, so the compiler and any editor resolve them. The cost is that the
// external implementation must be linked in.
//
// 字符串写法仍然完全可用，只是工具无法补全它，也不需要链接外部实现：
//   cut "root/control/button" graft "button_fast"
// The string form still works and needs no link, but tooling cannot complete it.
nichlink_run_method::static_graft_plan!(
    FRAMEWORK,
    cut(crate::control::object::button::NODE_ID)
        graft(control_button_graft::button_fast::NODE_ID),
);

/// 按框架和包命名空间装配这个示例的注册机。
/// Assemble the example's registry from its framework and package namespace.
pub fn base_registry() -> Registry {
    let mut registry = Registry::root_for_namespace(FRAMEWORK, env!("CARGO_PKG_NAME"));
    registry
        .register_all(&registrations())
        .expect("example faces register");
    registry
}

/// 打印注册树的逻辑路径，供示例二进制和集成测试共用。
/// Print the registry tree's logical paths; shared by the example binary and the
/// integration tests.
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
