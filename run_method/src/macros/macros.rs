//! Registration-face macros.
//! 注册面宏。

/// Declare the single host entry used by source-scope discovery.
/// 声明源码作用域发现使用的唯一宿主入口。
#[macro_export]
macro_rules! application {
    (entry = $entry:path $(,)?) => {
        #[doc(hidden)]
        pub const NICHLINK_APPLICATION_ENTRY: &str = stringify!($entry);
    };
}

/// Declare this crate as a NichLink host and pull in the registration plan
/// captured at build time.
/// 声明当前 crate 为 NichLink 宿主，并引入构建时捕获的注册计划。
///
/// `nichlink-build` renders the discovered registration tree to
/// `OUT_DIR/generated_lib.rs`; this macro includes it at the crate root so
/// `builtin_static_plan()` and the per-level `{name}_object!` aliases are
/// available crate-wide. It expands to
/// `include!(concat!(env!("OUT_DIR"), "/generated_lib.rs"))` — writing that
/// line directly is an equivalent, advanced alternative.
/// `nichlink-build` 把发现的注册树渲染到 `OUT_DIR/generated_lib.rs`；
/// 此宏将其包含到 crate 根，使 `builtin_static_plan()` 与各层级的
/// `{name}_object!` 别名在整个 crate 内可用。它展开为
/// `include!(concat!(env!("OUT_DIR"), "/generated_lib.rs"))`，
/// 直接书写该行是等价的高级写法。
#[macro_export]
macro_rules! host {
    () => {
        include!(concat!(env!("OUT_DIR"), "/generated_lib.rs"));
    };
}

/// Declare graft selectors for build-time capture without constructing a
/// runtime `GraftPlan`.
/// 声明供构建阶段捕获的 graft selector，不构造运行时 `GraftPlan`。
///
/// Put this at the host crate entry. `nichlink-build` validates the grammar and
/// stores the cuts in the generated `StaticPlan`. Use the dynamic
/// [`graft_plan!`](crate::graft_plan) expression only when code needs to build
/// or edit a plan at runtime.
/// 将它放在宿主 crate 入口。`nichlink-build` 校验语法并把切口写入生成的
/// `StaticPlan`；只有运行时代码确实要构造或编辑计划时才使用动态
/// [`graft_plan!`](crate::graft_plan) 表达式。
#[macro_export]
macro_rules! static_graft_plan {
    ($framework:expr, $($cuts:tt)+) => {
        const _: $crate::FrameworkId = $framework;
        const _: &str = stringify!($($cuts)+);
    };
}

/// Build a persistent external graft overlay without touching source files.
/// 构造持久化外部 graft 覆盖计划，不移动或修改任何源码文件。
///
/// ```
/// # use nichlink_run_method::{FrameworkId, graft_plan};
/// # let framework = FrameworkId::new("example");
/// let plan = graft_plan!(framework,
///     cut ["root/a1/b2"] graft "canvas_fast",
///     cut ["root/a"] full graft "a_fast",
/// );
/// assert_eq!(plan.cuts.len(), 2);
/// ```
#[macro_export]
macro_rules! graft_plan {
    ($framework:expr, $($cuts:tt)+) => {{
        let mut plan = $crate::GraftPlan::new($framework);
        $crate::__graft_plan_cuts!(plan; $($cuts)+);
        plan
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __graft_plan_cuts {
    ($plan:ident;) => {};
    ($plan:ident; cut [$start:literal to $end:literal] full graft $graft:literal $(, $($rest:tt)*)?) => {{
        let mut cut = $crate::GraftCut::range($start, $end, $graft);
        cut.subtree = true;
        $plan.cuts.push(cut);
        $crate::__graft_plan_cuts!($plan; $($($rest)*)?);
    }};
    ($plan:ident; cut [$start:literal to $end:literal] graft $graft:literal $(, $($rest:tt)*)?) => {{
        $plan.cuts
            .push($crate::GraftCut::range($start, $end, $graft));
        $crate::__graft_plan_cuts!($plan; $($($rest)*)?);
    }};
    ($plan:ident; cut [$path:literal] full graft $graft:literal $(, $($rest:tt)*)?) => {{
        $plan.cuts.push($crate::GraftCut::subtree($path, $graft));
        $crate::__graft_plan_cuts!($plan; $($($rest)*)?);
    }};
    ($plan:ident; cut [$path:literal] graft $graft:literal $(, $($rest:tt)*)?) => {{
        $plan.cuts.push($crate::GraftCut::new($path, $graft));
        $crate::__graft_plan_cuts!($plan; $($($rest)*)?);
    }};
    ($plan:ident; cut $path:literal full graft $graft:literal $(, $($rest:tt)*)?) => {{
        $plan.cuts.push($crate::GraftCut::subtree($path, $graft));
        $crate::__graft_plan_cuts!($plan; $($($rest)*)?);
    }};
    ($plan:ident; cut $path:literal graft $graft:literal $(, $($rest:tt)*)?) => {{
        $plan.cuts.push($crate::GraftCut::new($path, $graft));
        $crate::__graft_plan_cuts!($plan; $($($rest)*)?);
    }};
}

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

        pub const NODE_ID: $crate::NodeId = $crate::NodeId::from_namespaced_path(
            env!("CARGO_PKG_NAME"),
            $source,
            stringify!($kind),
        );

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

#[doc(hidden)]
#[macro_export]
macro_rules! __admission {
    () => {
        $crate::Admission::ANY
    };
    ($value:expr) => {
        $value
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __string_list {
    () => { &[] };
    ($($value:literal),* $(,)?) => { &[$($value),*] };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __flow {
    () => {
        $crate::FlowContract::NONE
    };
    ($value:expr) => {
        $value
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __flow_select {
    ($flow:expr; $provider:path) => {
        $flow
    };
    ($flow:expr; ) => {
        $flow
    };
    (; $provider:path) => {
        <$provider as $crate::FlowContractProvider>::FLOW_CONTRACT
    };
    (; ) => {
        $crate::FlowContract::NONE
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __flow_provider {
    () => {
        None
    };
    ($provider:path) => {
        Some(stringify!($provider))
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __plugin {
    () => {
        None
    };
    ($value:expr) => {
        Some($value)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __stable_name {
    () => {
        None
    };
    ($value:literal) => {
        Some($value)
    };
}

/// Emit a type-level interface check without storing a vtable.
/// 生成类型级接口检查，不保存 vtable。
///
/// The example is intentionally rejected by rustc. Keeping it here makes the
/// compile-time contract a regression test instead of a convention.
/// 这个示例故意让 rustc 拒绝；把它放在这里，能把编译期合同变成回归测试，
/// 而不是只靠约定。
///
/// ```compile_fail
/// use nichlink_run_method::__assert_impls;
///
/// struct MissingHandle;
/// trait RequiredHandle {}
///
/// __assert_impls!(MissingHandle; [RequiredHandle]);
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! __assert_impls {
    ($ty:ty; []) => {};
    ($ty:ty; [$($interface:path),+ $(,)?]) => {
        const _: fn() = || {
            fn assert_impl<T>()
            where
                $(T: $interface,)*
            {
            }
            assert_impl::<$ty>();
        };
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __face_expr_or {
    ($fallback:expr;) => {
        $fallback
    };
    ($fallback:expr; $value:expr) => {
        $value
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __face_string_or {
    ($fallback:expr; []) => {
        $fallback
    };
    ($fallback:expr; [$value:ident]) => {
        stringify!($value)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __face_value_or {
    ($fallback:expr; []) => {
        $fallback
    };
    ($fallback:expr; [$value:expr]) => {
        $value
    };
}

/// Internal declaration primitive used by generated parent-specific macros.
/// 供生成的父级专属宏使用的内部声明原语。
#[doc(hidden)]
#[macro_export]
macro_rules! __nichlink_object {
    (collector: $collector:ident, $($tokens:tt)*) => {
        $crate::__control_object! { collector: $collector, $($tokens)* }
    };
    ($($tokens:tt)*) => {
        $crate::__control_object! { collector: development, $($tokens)* }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __control_object {
    {
        collector: $collector:ident,
        kind: $kind:ident,
        preset: $preset:ty,
        parts: $parts:ty,
        name: { zh: $name_zh:literal, en: $name_en:literal },
        summary: { zh: $summary_zh:literal, en: $summary_en:literal },
        params: $params:literal,
        exports: [$($export:literal),* $(,)?],
        handle: $handle:ident,
        $(stable_name: $stable_name:literal,)?
        needs_registry: $needs_registry:expr,
        registry_name: $registry_name:ident,
        parent: $parent:expr,
        getting_from_other_registry: $getting:expr,
        registry_rule_path: $rule_path:expr,
        registry_rule: $rule:expr,
        $(admission: $admission:expr,)?
        $(handle_traits: [$($handle_trait:literal),* $(,)?],)?
        $(handle_contracts: [$($handle_contract:path),* $(,)?],)?
        $(part_traits: [$($part_trait:literal),* $(,)?],)?
        $(part_contracts: [$($part_contract:path),* $(,)?],)?
        requires: [$($require:literal => $provider:literal),* $(,)?],
        provides: [$($provide:literal),* $(,)?],
        expected_output: $expected_output:literal,
        actual_output: $actual_output:literal,
        $(flow: $flow:expr,)?
        $(flow_provider: $flow_provider:path,)?
        $(plugin: $plugin:expr,)?
        runtime_checks: [$($runtime_check:expr),* $(,)?] $(,)?
    } => {
        $crate::__registration_face! {
            source: $crate::registry_core::manifest_relative_source(env!("CARGO_MANIFEST_DIR"), file!()),
            collector: $collector,
            kind: $kind,
            preset: ($preset),
            preset_name: stringify!($preset),
            parts: ($parts),
            parts_name: stringify!($parts),
            name: { zh: $name_zh, en: $name_en },
            summary: { zh: $summary_zh, en: $summary_en },
            params: $params,
            exports: [$($export),*],
            handle: $handle,
            handle_name: stringify!($handle),
            $(stable_name: $stable_name,)?
            needs_registry: $needs_registry,
            registry_name: stringify!($registry_name),
            parent: $parent,
            getting_from_other_registry: $getting,
            registry_rule_path: $rule_path,
            registry_rule: $rule,
            $(admission: $admission,)?
            $(handle_traits: [$($handle_trait),*],)?
            $(handle_contracts: [$($handle_contract),*],)?
            $(part_traits: [$($part_trait),*],)?
            $(part_contracts: [$($part_contract),*],)?
            requires: [$($require => $provider),*],
            provides: [$($provide),*],
            expected_output: $expected_output,
            actual_output: $actual_output,
            $(flow: $flow,)?
            $(flow_provider: $flow_provider,)?
            $(plugin: $plugin,)?
            runtime_checks: [$($runtime_check),*],
        }
    };

    // Compact form. Fields are intentionally ordered like the generated
    // source, but every field after `kind` is optional and has a safe default.
    // 精简写法按生成源码顺序排列；除 kind 外均可省略，并使用安全默认值。
    {
        collector: $collector:ident,
        kind: $kind:ident,
        preset: $preset:ty,
        parts: $parts:ty,
        $(name: { zh: $name_zh:expr, en: $name_en:expr },)?
        $(summary: { zh: $summary_zh:expr, en: $summary_en:expr },)?
        $(params: $params:expr,)?
        $(exports: [$($export:expr),* $(,)?],)?
        handle: $handle:ident,
        $(stable_name: $stable_name:literal,)?
        $(needs_registry: $needs_registry:expr,)?
        $(registry_name: $registry_name:ident,)?
        $(parent: $parent:expr,)?
        $(getting_from_other_registry: $getting:expr,)?
        $(registry_rule_path: $rule_path:expr,)?
        $(registry_rule: $rule:expr,)?
        $(admission: $admission:expr,)?
        $(handle_traits: [$($handle_trait:literal),* $(,)?],)?
        $(handle_contracts: [$($handle_contract:path),* $(,)?],)?
        $(part_traits: [$($part_trait:literal),* $(,)?],)?
        $(part_contracts: [$($part_contract:path),* $(,)?],)?
        $(requires: [$($require:expr => $provider:expr),* $(,)?],)?
        $(provides: [$($provide:expr),* $(,)?],)?
        $(expected_output: $expected_output:expr,)?
        $(actual_output: $actual_output:expr,)?
        $(flow: $flow:expr,)?
        $(flow_provider: $flow_provider:path,)?
        $(plugin: $plugin:expr,)?
        $(runtime_checks: [$($runtime_check:expr),* $(,)?],)?
        $(,)?
    } => {
        $crate::__registration_face! {
            source: $crate::registry_core::manifest_relative_source(env!("CARGO_MANIFEST_DIR"), file!()),
            collector: $collector,
            kind: $kind,
            preset: $preset,
            preset_name: stringify!($preset),
            parts: $parts,
            parts_name: stringify!($parts),
            name: {
                zh: $crate::__face_value_or!(stringify!($kind); [$($name_zh)?]),
                en: $crate::__face_value_or!(stringify!($kind); [$($name_en)?]),
            },
            summary: {
                zh: $crate::__face_value_or!(""; [$($summary_zh)?]),
                en: $crate::__face_value_or!(""; [$($summary_en)?]),
            },
            params: $crate::__face_expr_or!(stringify!($kind); $($params)?),
            exports: [$($($export),*)?],
            handle: $handle,
            handle_name: stringify!($handle),
            $(stable_name: $stable_name,)?
            needs_registry: $crate::__face_expr_or!(false; $($needs_registry)?),
            registry_name: $crate::__face_string_or!($crate::registry_core::last_path_segment(module_path!()); [$($registry_name)?]),
            parent: $crate::__face_expr_or!($crate::root_node_id(env!("CARGO_PKG_NAME")); $($parent)?),
            getting_from_other_registry: $crate::__face_expr_or!(None; $($getting)?),
            registry_rule_path: $crate::__face_expr_or!($crate::registry_core::manifest_relative_source(env!("CARGO_MANIFEST_DIR"), file!()); $($rule_path)?),
            registry_rule: $crate::__face_expr_or!($crate::RegistrationRule::ANY; $($rule)?),
            $(admission: $admission,)?
            $(handle_traits: [$($handle_trait),*],)?
            $(handle_contracts: [$($handle_contract),*],)?
            $(part_traits: [$($part_trait),*],)?
            $(part_contracts: [$($part_contract),*],)?
            requires: [$($($require => $provider),*)?],
            provides: [$($($provide),*)?],
            expected_output: $crate::__face_expr_or!("()"; $($expected_output)?),
            actual_output: $crate::__face_expr_or!("()"; $($actual_output)?),
            $(flow: $flow,)?
            $(flow_provider: $flow_provider,)?
            $(plugin: $plugin,)?
            runtime_checks: [$($($runtime_check),*)?],
        }
    };
    // Compact form with a custom handle but default preset and parts.
    // 使用自定义 handle、但采用默认 preset/parts 的精简形式。
    {
        collector: $collector:ident,
        kind: $kind:ident,
        $(name: { zh: $name_zh:expr, en: $name_en:expr },)?
        $(summary: { zh: $summary_zh:expr, en: $summary_en:expr },)?
        $(params: $params:expr,)?
        $(exports: [$($export:expr),* $(,)?],)?
        handle: $handle:ident,
        $(stable_name: $stable_name:literal,)?
        $(needs_registry: $needs_registry:expr,)?
        $(registry_name: $registry_name:ident,)?
        $(parent: $parent:expr,)?
        $(getting_from_other_registry: $getting:expr,)?
        $(registry_rule_path: $rule_path:expr,)?
        $(registry_rule: $rule:expr,)?
        $(admission: $admission:expr,)?
        $(handle_traits: [$($handle_trait:literal),* $(,)?],)?
        $(handle_contracts: [$($handle_contract:path),* $(,)?],)?
        $(part_traits: [$($part_trait:literal),* $(,)?],)?
        $(part_contracts: [$($part_contract:path),* $(,)?],)?
        $(requires: [$($require:expr => $provider:expr),* $(,)?],)?
        $(provides: [$($provide:expr),* $(,)?],)?
        $(expected_output: $expected_output:expr,)?
        $(actual_output: $actual_output:expr,)?
        $(flow: $flow:expr,)?
        $(flow_provider: $flow_provider:path,)?
        $(plugin: $plugin:expr,)?
        $(runtime_checks: [$($runtime_check:expr),* $(,)?],)?
        $(,)?
    } => {
        $crate::__control_object! {
            collector: $collector,
            kind: $kind,
            preset: $crate::NoPreset,
            parts: $crate::NoParts,
            $(name: { zh: $name_zh, en: $name_en },)?
            $(summary: { zh: $summary_zh, en: $summary_en },)?
            $(params: $params,)?
            $(exports: [$($export),*],)?
            handle: $handle,
            $(stable_name: $stable_name,)?
            $(needs_registry: $needs_registry,)?
            $(registry_name: $registry_name,)?
            $(parent: $parent,)?
            $(getting_from_other_registry: $getting,)?
            $(registry_rule_path: $rule_path,)?
            $(registry_rule: $rule,)?
            $(admission: $admission,)?
            $(handle_traits: [$($handle_trait),*],)?
            $(handle_contracts: [$($handle_contract),*],)?
            $(part_traits: [$($part_trait),*],)?
            $(part_contracts: [$($part_contract),*],)?
            $(requires: [$($require => $provider),*],)?
            $(provides: [$($provide),*],)?
            $(expected_output: $expected_output,)?
            $(actual_output: $actual_output,)?
            $(flow: $flow,)?
            $(flow_provider: $flow_provider,)?
            $(plugin: $plugin,)?
            $(runtime_checks: [$($runtime_check),*],)?
        }
    };
    {
        collector: $collector:ident,
        kind: $kind:ident,
        $(preset: $preset:ty,)?
        $(parts: $parts:ty,)?
        $(name: { zh: $name_zh:expr, en: $name_en:expr },)?
        $(summary: { zh: $summary_zh:expr, en: $summary_en:expr },)?
        $(params: $params:expr,)?
        $(exports: [$($export:expr),* $(,)?],)?
        $(stable_name: $stable_name:literal,)?
        $(needs_registry: $needs_registry:expr,)?
        $(registry_name: $registry_name:ident,)?
        $(parent: $parent:expr,)?
        $(getting_from_other_registry: $getting:expr,)?
        $(registry_rule_path: $rule_path:expr,)?
        $(registry_rule: $rule:expr,)?
        $(admission: $admission:expr,)?
        $(handle_traits: [$($handle_trait:literal),* $(,)?],)?
        $(handle_contracts: [$($handle_contract:path),* $(,)?],)?
        $(part_traits: [$($part_trait:literal),* $(,)?],)?
        $(part_contracts: [$($part_contract:path),* $(,)?],)?
        $(requires: [$($require:expr => $provider:expr),* $(,)?],)?
        $(provides: [$($provide:expr),* $(,)?],)?
        $(expected_output: $expected_output:expr,)?
        $(actual_output: $actual_output:expr,)?
        $(flow: $flow:expr,)?
        $(flow_provider: $flow_provider:path,)?
        $(plugin: $plugin:expr,)?
        $(runtime_checks: [$($runtime_check:expr),* $(,)?],)?
        $(,)?
    } => {
        $crate::__registration_face! {
            source: $crate::registry_core::manifest_relative_source(env!("CARGO_MANIFEST_DIR"), file!()),
            collector: $collector,
            kind: $kind,
            preset: $crate::NoPreset,
            preset_name: "NoPreset",
            parts: $crate::NoParts,
            parts_name: "NoParts",
            name: {
                zh: $crate::__face_value_or!(stringify!($kind); [$($name_zh)?]),
                en: $crate::__face_value_or!(stringify!($kind); [$($name_en)?]),
            },
            summary: {
                zh: $crate::__face_value_or!(""; [$($summary_zh)?]),
                en: $crate::__face_value_or!(""; [$($summary_en)?]),
            },
            params: $crate::__face_expr_or!(stringify!($kind); $($params)?),
            exports: [$($($export),*)?],
            handle: $kind,
            handle_name: stringify!($kind),
            $(stable_name: $stable_name,)?
            needs_registry: $crate::__face_expr_or!(false; $($needs_registry)?),
            registry_name: $crate::__face_string_or!($crate::registry_core::last_path_segment(module_path!()); [$($registry_name)?]),
            parent: $crate::__face_expr_or!($crate::root_node_id(env!("CARGO_PKG_NAME")); $($parent)?),
            getting_from_other_registry: $crate::__face_expr_or!(None; $($getting)?),
            registry_rule_path: $crate::__face_expr_or!($crate::registry_core::manifest_relative_source(env!("CARGO_MANIFEST_DIR"), file!()); $($rule_path)?),
            registry_rule: $crate::__face_expr_or!($crate::RegistrationRule::ANY; $($rule)?),
            $(admission: $admission,)?
            $(handle_traits: [$($handle_trait),*],)?
            $(handle_contracts: [$($handle_contract),*],)?
            $(part_traits: [$($part_trait),*],)?
            $(part_contracts: [$($part_contract),*],)?
            requires: [$($($require => $provider),*)?],
            provides: [$($($provide),*)?],
            expected_output: $crate::__face_expr_or!("()"; $($expected_output)?),
            actual_output: $crate::__face_expr_or!("()"; $($actual_output)?),
            $(flow: $flow,)?
            $(flow_provider: $flow_provider,)?
            $(plugin: $plugin,)?
            runtime_checks: [$($($runtime_check),*)?],
        }
    };

    // The smallest useful face: kind is also the default handle and registry
    // slot. The trailing comma is optional for hand-written declarations.
    // 最小可用注册面：kind 同时作为默认 handle 和注册槽位，末尾逗号可省略。
    {
        collector: $collector:ident,
        kind: $kind:ident $(,)?
    } => {
        $crate::__registration_face! {
            source: $crate::registry_core::manifest_relative_source(env!("CARGO_MANIFEST_DIR"), file!()),
            collector: $collector,
            kind: $kind,
            preset: ($crate::NoPreset),
            preset_name: "NoPreset",
            parts: ($crate::NoParts),
            parts_name: "NoParts",
            name: { zh: stringify!($kind), en: stringify!($kind) },
            summary: { zh: "", en: "" },
            params: stringify!($kind),
            exports: [],
            handle: $kind,
            handle_name: stringify!($kind),
            needs_registry: false,
            registry_name: stringify!($kind),
            parent: $crate::root_node_id(env!("CARGO_PKG_NAME")),
            getting_from_other_registry: None,
            registry_rule_path: $crate::registry_core::manifest_relative_source(env!("CARGO_MANIFEST_DIR"), file!()),
            registry_rule: $crate::RegistrationRule::ANY,
            requires: [],
            provides: [],
            expected_output: "()",
            actual_output: "()",
            runtime_checks: [],
        }
    };

    // Anything the strict arms above decline — fields in another order, `;`
    // separators, a forgotten separator, a misspelled field — goes to the front
    // end, which reorders tolerantly and reports the exact token it rejects. A
    // well-formed face never reaches this arm, so its expansion is unchanged.
    // 上面所有严格 arm 都不接受的声明——字段顺序不同、用 `;` 分隔、漏写分隔符、字段名
    // 拼错——交给前端：它宽容重排，并把被拒绝的那个 token 精确报出来。合法注册面
    // 永远不会走到这里，展开因此保持不变。
    {
        collector: $collector:ident,
        $($tokens:tt)*
    } => {
        ::nichlink_run_method::face_fields! { @control collector: $collector, $($tokens)* }
    };

}

/// The field vocabulary of the authoring macros, as real Rust fields.
/// 作者侧宏的字段词表，以真实 Rust 字段表示。
///
/// Nothing constructs this type. The face macros splice the author's own tokens
/// into a literal of it under `cfg(rust_analyzer)`, which is what lets an editor
/// complete the field names inside `crate::<name>_object! { … }` and list them in
/// the order the macros accept, because an editor can only read a real field
/// list — a macro's token tree is opaque to it.
/// 没有任何代码构造这个类型。注册面宏在 `cfg(rust_analyzer)` 下把作者写的 token
/// 拼进它的一个字面量，编辑器因此能在 `crate::<name>_object! { … }` 里补全字段名，
/// 并按宏接受的顺序列出候选——编辑器读不了宏的 token 树，只能读真实的字段列表。
///
/// That literal is deliberately **not** valid, type-correct Rust, and it is not
/// meant to be compiled: every field is typed `()` while the author writes real
/// values, and spellings such as `name: { zh: "…", en: "…" }` are not
/// expressions at all. It is gated behind `cfg(rust_analyzer)`, so an ordinary
/// build never sees it, and `--cfg rust_analyzer` is an editor setting rather
/// than a supported way to compile a host. Its only job is the field list.
/// 那份字面量**故意**不是合法且类型正确的 Rust，也不打算被编译：字段一律写作 `()`
/// 而作者写的是真实值，`name: { zh: "…", en: "…" }` 这类写法更不是表达式。它由
/// `cfg(rust_analyzer)` 把关，普通构建看不到它；`--cfg rust_analyzer` 是编辑器设置，
/// 不是受支持的编译宿主方式。它唯一的职责就是那张字段列表。
///
/// The declaration order below is the order the compact macro arm accepts, and
/// an editor lists fields in that order.
/// 下面的声明顺序就是紧凑 arm 接受的顺序，编辑器也按这个顺序列出字段。
///
/// One authoring key is not a Rust expression — `requires: [a => b]` — so a
/// declaration that uses it makes the IDE-only literal unparsable from that
/// field on. That is deliberate: the splice only serves completion and is never
/// compiled, and the key stays available for the editor to suggest.
/// 有一个作者侧键不是 Rust 表达式——`requires: [a => b]`——因此用到它的声明会让这份
/// 仅供 IDE 的字面量从该字段起无法解析。这是有意的：拼接只为补全服务、从不参与
/// 编译，而这个键仍需要被编辑器提示出来。
#[doc(hidden)]
pub struct FaceFields {
    pub source: (),
    pub kind: (),
    pub preset: (),
    pub parts: (),
    pub name: (),
    pub summary: (),
    pub params: (),
    pub exports: (),
    pub handle: (),
    pub stable_name: (),
    pub needs_registry: (),
    pub registry_name: (),
    pub parent: (),
    pub getting_from_other_registry: (),
    pub registry_rule_path: (),
    pub registry_rule: (),
    pub admission: (),
    pub handle_traits: (),
    pub handle_contracts: (),
    pub part_traits: (),
    pub part_contracts: (),
    pub requires: (),
    pub provides: (),
    pub expected_output: (),
    pub actual_output: (),
    pub flow: (),
    pub flow_provider: (),
    pub plugin: (),
    pub runtime_checks: (),
}

/// Splice an author's face tokens into a real field list, for editors only.
/// 把作者写的注册面 token 拼进真实字段列表，仅供编辑器使用。
///
/// The face macros invoke this under `cfg(rust_analyzer)`. Expanding it is what
/// turns an opaque macro token tree into a field list an editor can read, and
/// rustc never expands it, so the authoring syntax stays free to use spellings
/// that are not Rust expressions (`name: { zh: "…", en: "…" }`, for example).
/// 注册面宏在 `cfg(rust_analyzer)` 下调用它。正是这次展开把不透明的宏 token 树变成
/// 编辑器能读的字段列表；rustc 从不展开它，因此作者侧语法可以继续使用并非 Rust
/// 表达式的写法（例如 `name: { zh: "…", en: "…" }`）。
/// Splice an author's face tokens into a real field list, for editors only.
/// 把作者写的注册面 token 拼进真实字段列表，仅供编辑器使用。
///
/// Face macros invoke it under `cfg(rust_analyzer)`, which belongs to the crate
/// being analyzed: the author's own crate. Expanding it turns an opaque macro
/// token tree into a field list an editor can read. `rustc` never expands it,
/// so the authoring syntax stays free to use spellings that are not Rust
/// expressions (`name: { zh: "…", en: "…" }`, for example).
/// 注册面宏在 `cfg(rust_analyzer)` 下调用它——该 cfg 属于被分析的 crate，也就是
/// 作者自己的 crate。正是这次展开把不透明的宏 token 树变成编辑器能读的字段列表；
/// `rustc` 从不展开它，因此作者侧语法可以继续使用并非 Rust 表达式的写法（例如
/// `name: { zh: "…", en: "…" }`）。
#[doc(hidden)]
#[macro_export]
macro_rules! __face_fields {
    ($($tokens:tt)*) => {
        const _: () = {
            let _ = $crate::macros::FaceFields { $($tokens)* };
        };
    };
}

/// `kind` is the handle-marker type this file declares (`pub struct <Kind>;`), so
/// write that type first and reference it here: an editor cannot complete a name
/// the author has not written yet, and `kind` is captured as an identifier
/// rather than an expression, which is also why value completion does not fire
/// there. Every other field completes normally.
/// `kind` 就是本文件声明的那个 handle 标记类型（`pub struct <Kind>;`）：先写出该类型，
/// 再在这里引用它。编辑器无法补全一个作者还没写下的名字，而且 `kind` 是以标识符而非
/// 表达式捕获的——这也是它的值位不会弹候选的原因。其余字段的值都能正常补全。
/// Declare a registration face: `kind` first, then any of `preset`, `parts`,
/// `name`, `summary`, `params`, `exports`, `handle`, `stable_name`,
/// `needs_registry`, `registry_name`, `parent`, `getting_from_other_registry`,
/// `registry_rule_path`, `registry_rule`, `admission`, `handle_traits`,
/// `handle_contracts`, `part_traits`, `part_contracts`, `requires`,
/// `provides`, `expected_output`, `actual_output`, `flow`, `flow_provider`,
/// `plugin`, `runtime_checks` — in that order, each one optional.
/// 声明一个注册面：先写 `kind`，其后可依次使用 `preset`、`parts`、`name`、
/// `summary`、`params`、`exports`、`handle`、`stable_name`、`needs_registry`、
/// `registry_name`、`parent`、`getting_from_other_registry`、
/// `registry_rule_path`、`registry_rule`、`admission`、`handle_traits`、
/// `handle_contracts`、`part_traits`、`part_contracts`、`requires`、
/// `provides`、`expected_output`、`actual_output`、`flow`、`flow_provider`、
/// `plugin`、`runtime_checks`——顺序如上，每一项都可省略。
/// Declare a registration face owned by an external crate.
/// 声明由外部 crate 所有的注册面。
///
/// Unlike generated parent-specific macros, this form receives its source path
/// explicitly; an external crate is not part of the host's generated tree.
/// 与生成的父级专属宏不同，此形式显式接收源码路径；外部 crate 不在宿主
/// 自动生成的模块树中。
#[macro_export]
macro_rules! external_object {
    { @tokens collector: $collector:ident, $($tokens:tt)* } => {
        // The author's fields go through the same front end as a generated
        // alias, so `;` separators and any order are accepted here too; it
        // re-dispatches to `__external_object!`, whose matcher names the source
        // file first.
        // 作者的字段与生成的别名走同一个前端，因此这里同样接受 `;` 与任意顺序；
        // 前端会回派到 `__external_object!`——它的 matcher 首要指出源文件。
        $crate::face_fields! { @external collector: $collector, $($tokens)* }
        #[cfg(rust_analyzer)]
        $crate::__face_fields! { $($tokens)* }
    };
    { @tokens $($tokens:tt)* } => {
        $crate::face_fields! { @external collector: linked, $($tokens)* }
        #[cfg(rust_analyzer)]
        $crate::__face_fields! { $($tokens)* }
    };
    { $($tokens:tt)* } => {
        $crate::external_object! { @tokens $($tokens)* }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __external_object {
    {
        collector: $collector:ident,
        source: $source:expr,
        kind: $kind:ident,
        preset: $preset:ty,
        parts: $parts:ty,
        name: { zh: $name_zh:literal, en: $name_en:literal },
        summary: { zh: $summary_zh:literal, en: $summary_en:literal },
        params: $params:literal,
        exports: [$($export:literal),* $(,)?],
        handle: $handle:ident,
        $(stable_name: $stable_name:literal,)?
        needs_registry: $needs_registry:expr,
        registry_name: $registry_name:ident,
        parent: $parent:expr,
        getting_from_other_registry: $getting:expr,
        registry_rule_path: $rule_path:expr,
        registry_rule: $rule:expr,
        $(admission: $admission:expr,)?
        $(handle_traits: [$($handle_trait:literal),* $(,)?],)?
        $(handle_contracts: [$($handle_contract:path),* $(,)?],)?
        $(part_traits: [$($part_trait:literal),* $(,)?],)?
        $(part_contracts: [$($part_contract:path),* $(,)?],)?
        requires: [$($require:literal => $provider:literal),* $(,)?],
        provides: [$($provide:literal),* $(,)?],
        expected_output: $expected_output:literal,
        actual_output: $actual_output:literal,
        $(flow: $flow:expr,)?
        $(flow_provider: $flow_provider:path,)?
        $(plugin: $plugin:expr,)?
        runtime_checks: [$($runtime_check:expr),* $(,)?] $(,)?
    } => {
        $crate::__registration_face! {
            source: $source,
            collector: $collector,
            kind: $kind,
            preset: ($preset),
            preset_name: stringify!($preset),
            parts: ($parts),
            parts_name: stringify!($parts),
            name: { zh: $name_zh, en: $name_en },
            summary: { zh: $summary_zh, en: $summary_en },
            params: $params,
            exports: [$($export),*],
            handle: $handle,
            handle_name: stringify!($handle),
            $(stable_name: $stable_name,)?
            needs_registry: $needs_registry,
            registry_name: stringify!($registry_name),
            parent: $parent,
            getting_from_other_registry: $getting,
            registry_rule_path: $rule_path,
            registry_rule: $rule,
            $(admission: $admission,)?
            $(handle_traits: [$($handle_trait),*],)?
            $(handle_contracts: [$($handle_contract),*],)?
            $(part_traits: [$($part_trait),*],)?
            $(part_contracts: [$($part_contract),*],)?
            requires: [$($require => $provider),*],
            provides: [$($provide),*],
            expected_output: $expected_output,
            actual_output: $actual_output,
            $(flow: $flow,)?
            $(flow_provider: $flow_provider,)?
            $(plugin: $plugin,)?
            runtime_checks: [$($runtime_check),*],
        }
    };
}

/// Record one logical function invocation with its source location.
/// 用一条宏调用记录一次逻辑函数调用，并保留源码位置。
///
/// The closure is deliberately the only required wrapper. It keeps the
/// tracing boundary explicit, works on stable Rust, and does not introduce a
/// trait object or a global callback into the hot path.
/// 闭包是唯一必须写出的边界，兼容 stable Rust；不会引入 trait object，
/// 也不会在热路径上增加全局回调。
#[macro_export]
macro_rules! trace_call {
    ($trace:expr, $node:expr, $function:literal, $operation:expr) => {{ $trace.with($node, $function, $operation) }};
}

/// Record a fallible invocation according to the trace policy.
/// 按追踪策略记录一个可失败的函数调用。
///
/// In `errors-only` mode, successful scopes are discarded and `Err`/panic
/// scopes remain available for diagnostics. Other modes keep their normal
/// behavior.
/// 在 `errors-only` 模式下成功作用域会回滚，`Err` 或 panic 作用域会保留供诊断；
/// 其它模式保持正常收集行为。
#[macro_export]
macro_rules! trace_call_result {
    ($trace:expr, $operation:expr) => {{ $trace.with_result($operation) }};
}

/// Capture a function input or local value at the macro call site.
/// 在宏调用位置记录函数输入或局部值。
#[macro_export]
macro_rules! trace_value {
    ($trace:expr, $name:literal, $type_name:literal, $value:expr, $kind:expr) => {{ $trace.local($name, $type_name, $value, $kind) }};
}

/// Capture a transformed value and connect it to its producer.
/// 记录转换后的值，并把它连接到生产者。
#[macro_export]
macro_rules! trace_transform {
    ($trace:expr, $input:expr, $name:literal, $type_name:literal, $value:expr) => {{ $trace.transform($input, $name, $type_name, $value) }};
}

/// Capture a value consumed by another function parameter.
/// 记录值被另一个函数参数消费的位置。
#[macro_export]
macro_rules! trace_consume {
    ($trace:expr, $input:expr, $function:literal, $parameter:literal) => {{ $trace.consume($input, $function, $parameter) }};
}

/// Capture a function return value and connect it to its input.
/// 记录函数返回值，并把它连接到输入值。
#[macro_export]
macro_rules! trace_return {
    ($trace:expr, $input:expr, $name:literal, $type_name:literal, $value:expr) => {{ $trace.return_value($input, $name, $type_name, $value) }};
}

#[cfg(test)]
mod tests {
    use nichlink::registry_core::declaration::FACE_FIELD_ORDER;

    /// The editor's field mirror and the order the front end sorts into must stay
    /// one vocabulary; a field added to one alone would silently stop being
    /// completed or silently stop being accepted.
    /// 编辑器用的字段镜像与前端排序依据必须是同一份词表；只往一边加字段，会让它悄悄
    /// 失去补全，或悄悄不再被接受。
    #[test]
    fn the_field_mirror_matches_the_declared_order() {
        let source = include_str!("macros.rs");
        let start = source
            .find("pub struct FaceFields {")
            .expect("field mirror");
        let end = source[start..].find("\n}").expect("field mirror end") + start;
        let mirrored = source[start..end]
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                let name = line.strip_prefix("pub ")?.strip_suffix(": (),")?;
                Some(name)
            })
            .collect::<Vec<_>>();
        assert_eq!(mirrored, FACE_FIELD_ORDER);
    }
}
