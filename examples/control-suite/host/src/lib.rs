//! XiRang 示例：README 里的 Control / Button 两层树，作为一个真实宿主库。
//! XiRang example: the README Control/Button two-level tree as a real host
//! library.
//!
//! 整个 crate 只有这里一处构建接线。`host!()` 引入构建期生成的注册计划；
//! 注册面代码保持普通 Rust，父级不维护子对象清单。
//! The crate has one build wiring point. `host!()` pulls in the plan the build
//! step generated; face code stays ordinary Rust and no parent keeps a child
//! roster.

xirang_toolchain::run_method::host!();

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
pub const FRAMEWORK: FrameworkId = FrameworkId::new("xirang.example.control-suite");

// 宿主入口的 graft 计划，用**类型化**写法：两侧都是指向真实注册面的 Rust 路径，
// 因此编译器与编辑器都能解析它们——写在 `cut(` 之后会补全宿主注册面路径，
// 写在 `graft(` 之后会补全外部 crate 路径。代价是外部实现必须被静态链接进来。
// The host's graft plan in the **typed** form: both sides are Rust paths to real
// faces, so the compiler and any editor resolve them. The cost is that the
// external implementation must be linked in.
//
// 这里声明的每个 `cut(` 都是宿主交出去的槽位，而构建期作用域收窄到这些切口命名的
// 子树：没有声明的注册面不会被这个应用发布。按钮和滑块都是可替换槽位，因此两条都写；
// 漏写一条不是"少发布一个面"这么无害，而是让那个槽位在发布态计划里失去目标。
// Every `cut(` declared here is a slot the host hands over, and the build-time
// scope narrows to the subtrees these cuts name: a face nobody declared is not
// shipped by this application. Button and slider are both replaceable slots, so
// both are declared; leaving one out does not merely ship one face less, it
// leaves that slot without a target in the release-time plan.
//
// 字符串写法仍然完全可用，只是工具无法补全它，也不需要链接外部实现：
//   cut "root/control/button" graft "button_fast"
// The string form still works and needs no link, but tooling cannot complete it.
xirang_toolchain::run_method::static_graft_plan!(
    FRAMEWORK,
    cut(crate::control::object::button::NODE_ID)
        graft(control_button_graft::button_fast::NODE_ID),
    cut(crate::control::object::slider::NODE_ID)
        graft(control_button_graft::slider_fast::NODE_ID),
);

/// 按框架和包命名空间装配这个示例的注册机。
/// Assemble the example's registry from its framework and package namespace.
///
/// The same named entry `examples/control-button` uses: two hosts, one incantation — and this one is
/// **not a workspace member**, so it is the half that proves the entry works from a crate whose
/// `build.rs` ran outside the workspace build. `tools/xirang-daily-behaviors` is what builds it
/// (measured: it was the only device naming this example, and `tools/xirang-external-rehearsal`
/// does not touch it).
/// 与 `examples/control-button` 用的是同一个具名入口：两个宿主、一句咒语——而这一份**不是工作区成员**，
/// 因此它是"该入口能从'不在工作区构建里'的 crate 用起来"的那一半。构建它的是
/// `tools/xirang-daily-behaviors`（实测：只有它点名这个示例，而 `tools/xirang-external-rehearsal`
/// 根本不碰它）。
pub fn base_registry() -> Registry {
    xirang_toolchain::host_registry!().expect("example faces register")
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

#[cfg(test)]
mod tests {
    /// What the host still compiles: `panel` stays with it, and its own plan is non-empty. The
    /// subtrees it handed away are **not** nameable here — by design, and the reason the deep paths
    /// are checked through the tool's records instead (`xirang explain …`).
    /// 宿主还编译什么：`panel` 留在它这里，它自己的计划非空。它交出去的那些子树在这里**叫不出来**——这是设计，
    /// 也正是深路径改由工具的记录来核对的原因（`xirang explain …`）。
    #[test]
    fn the_host_still_owns_what_it_did_not_hand_away() {
        assert!(!crate::panel::NODE_ID.to_string().is_empty());
        assert!(!crate::XIRANG_NAMESPACE.is_empty(), "the host keeps a namespace");
    }
}
