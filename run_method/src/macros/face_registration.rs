//! Registration-face declaration and submission macros.
//! 注册面声明与提交宏。

/// Expand one fully normalized registration face into consts and submission.
/// 把一个完整规范化的注册面展开为常量与提交调用。
///
/// Internal: `object!` and `external_object!` normalize their shorthand into
/// this arm, so host code should call those instead. Every field is assumed
/// present, and the collector ident selects which linker section receives the
/// registration.
/// 内部宏：`object!` 与 `external_object!` 把简写规范化为这一分支，宿主应改用那些宏。
/// 它假定每个字段都已给出，收集器标识决定注册信息进入哪个链接器段。
#[macro_export]
macro_rules! __registration_face {
    {
        source: $source:expr,
        collector: $collector:ident,
        kind: $kind:ident,
        preset: $preset:ty,
        preset_name: $preset_name:expr,
        parts: $parts:ty,
        parts_name: $parts_name:expr,
        name: { zh: $name_zh:expr, en: $name_en:expr $(,)? },
        summary: { zh: $summary_zh:expr, en: $summary_en:expr $(,)? },
        params: $params:expr,
        exports: [$($export:expr),* $(,)?],
        handle: $handle:ident,
        handle_name: $handle_name:expr,
        $(stable_name: $stable_name:literal,)?
        needs_registry: $needs_registry:expr,
        registry_name: $registry_name:expr,
        parent: $parent:expr,
        getting_from_other_registry: $getting:expr,
        registry_rule_path: $rule_path:expr,
        registry_rule: $rule:expr,
        $(admission: $admission:expr,)?
        $(handle_traits: [$($handle_trait:literal),* $(,)?],)?
        $(handle_contracts: [$($handle_contract:path),* $(,)?],)?
        $(part_traits: [$($part_trait:literal),* $(,)?],)?
        $(part_contracts: [$($part_contract:path),* $(,)?],)?
        requires: [$($require:expr => $provider:expr),* $(,)?],
        provides: [$($provide:expr),* $(,)?],
        expected_output: $expected_output:expr,
        actual_output: $actual_output:expr,
        $(flow: $flow:expr,)?
        $(flow_provider: $flow_provider:path,)?
        $(plugin: $plugin:expr,)?
        runtime_checks: [$($runtime_check:expr),* $(,)?] $(,)?
    } => {
        const _: () = $crate::assert_contract::<$preset, $parts>();
        $crate::__assert_impls!($handle; [$($($handle_contract),*)?]);
        $crate::__assert_impls!($parts; [$($($part_contract),*)?]);

        // These two constants are part of the author's public API — a typed graft
        // cut names `crate::…::NODE_ID` — and an out-of-project face mounts its own
        // file, so the declaring crate's `#![warn(missing_docs)]` sees them. They
        // are documented here rather than hidden because an author is supposed to
        // use them; `#[doc(hidden)]` would only silence the warning by lying about
        // the surface.
        // 这两个常量属于作者的公开 API——类型化 graft 切口就是命名
        // `crate::…::NODE_ID`——而项目外的面由作者自己挂载文件，因此声明方 crate 的
        // `#![warn(missing_docs)]` 会看到它们。在这里补文档而不是隐藏，是因为作者本就
        // 应当使用它们；`#[doc(hidden)]` 只会靠谎报表面来消除警告。

        /// The face's compile-time identity, for typed graft cuts and parent links.
        /// 该注册面的编译期身份，供类型化 graft 切口与父级链接使用。
        ///
        /// It hashes the package namespace, the declaration's relative source path
        /// and its kind — never the registry slot name, so renaming a slot does not
        /// move an identity.
        /// 它哈希包命名空间、声明的相对源码路径与 kind——绝不含注册槽位名，因此槽位改名
        /// 不会移动身份。
        pub const NODE_ID: $crate::NodeId = $crate::NodeId::from_namespaced_path(
            env!("CARGO_PKG_NAME"),
            $source,
            stringify!($kind),
        );

        /// The same declaration in the owned form registration consumes.
        /// 同一份声明的拥有型快照，注册过程消费它。
        pub const REGISTRATION: $crate::RegistrationInfo = $crate::RegistrationInfo {
            namespace: env!("CARGO_PKG_NAME"),
            id: NODE_ID,
            parent: $parent,
            kind: stringify!($kind),
            preset: $preset_name,
            parts: $parts_name,
            params: $params,
            handle: $handle_name,
            stable_name: $crate::__stable_name!($($stable_name)?),
            name: $crate::LocalizedText { zh: $name_zh, en: $name_en },
            summary: $crate::LocalizedText { zh: $summary_zh, en: $summary_en },
            exports: &[$($export),*],
            needs_registry: $needs_registry,
            registry_name: $registry_name,
            getting_from_other_registry: $getting,
            registry_rule_path: $rule_path,
            registry_rule: $rule,
            admission: $crate::__admission!($($admission)?),
            requires: &[$($crate::RequirementSpec {
                capability: $require,
                provider: $provider,
            }),*],
            provides: &[$($provide),*],
            contract: $crate::ObjectContract {
                required_parts: <$preset as $crate::PresetContract>::REQUIRED_PARTS,
                provided_parts: <$parts as $crate::PartsContract>::PROVIDED_PARTS,
                expected_output: $expected_output,
                actual_output: $actual_output,
            },
            flow: $crate::__flow_select!($($flow)?; $($flow_provider)?),
            flow_provider: $crate::__flow_provider!($($flow_provider)?),
            handle_traits: $crate::__string_list!($($($handle_trait),*)?),
            part_traits: $crate::__string_list!($($($part_trait),*)?),
            runtime_checks: &[$($runtime_check),*],
            plugin: $crate::__plugin!($($plugin)?),
            source: $crate::SourceLocation {
                file: $source,
                line: line!(),
                column: column!(),
                function: $handle_name,
            },
        };

        // The collector is a development control plane. Release applications
        // use the generated StaticPlan and do not retain inventory sections.
        // 收集器属于开发控制面；正式应用使用生成的 StaticPlan，不保留 inventory 段。
        $crate::__submit_registration!($collector; REGISTRATION);
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __submit_registration {
    (development; $registration:ident) => {};
    (debug; $registration:ident) => {
        #[cfg(debug_assertions)]
        ::nichlink_debug_method::submit! { $registration }
    };
    (linked; $registration:ident) => {
        // Collection is owned by nichlink-debug; core keeps declarations pure.
        // 收集由 nichlink-debug 持有；core 只保留纯声明。
    };
}
