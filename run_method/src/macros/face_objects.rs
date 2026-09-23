//! Object declaration macros: generated aliases and their hidden implementation.
//! 对象声明宏：生成的别名及其隐藏实现。

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
        $(preset: $preset:ty,)?
        $(parts: $parts:ty,)?
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
        // Why the direct implementation is wrong: `stringify!($preset)` would
        // name a default that only exists as a type token, so an omitted binding
        // would record `"$crate :: NoPreset"` while every authored and defaulting
        // path records `"NoPreset"` — one default, two names. Resolving the type
        // and the name through the same optional helper keeps them from one
        // place.
        // Boundary: the full form may omit either binding; a written one is
        // forwarded verbatim and an absent one becomes `NoPreset`/`NoParts`.
        // Pinned by `run_method/tests/face_arm_defaults.rs`.
        // 直白写法错在哪：`stringify!($preset)` 会把一个只以类型 token 形式存在的默认值
        // 写成名字，于是省略绑定会记录 `"$crate :: NoPreset"`，而所有显式路径与取默认值
        // 的路径记录 `"NoPreset"`——同一默认值两个名字。把类型与名字经同一个可选辅助宏
        // 取值，二者便出自一处。
        // 边界：完整写法可省略任一绑定；写下的原样转发，省略的变成
        // `NoPreset`/`NoParts`。
        // 由 `run_method/tests/face_arm_defaults.rs` 钉住。
        $crate::__registration_face! {
            source: $crate::registry_core::manifest_relative_source(env!("CARGO_MANIFEST_DIR"), file!()),
            collector: $collector,
            kind: $kind,
            preset: $crate::__face_ty_or!($crate::NoPreset; $($preset)?),
            preset_name: $crate::__face_ty_name_or!("NoPreset"; $($preset)?),
            parts: $crate::__face_ty_or!($crate::NoParts; $($parts)?),
            parts_name: $crate::__face_ty_name_or!("NoParts"; $($parts)?),
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

    // Compact form with a custom handle. `preset` and `parts` are independently
    // optional, so this arm owns every handle-carrying shape: neither binding,
    // either one alone, or both. `handle` is the one post-`kind` field that stays
    // mandatory — that is what makes this the handle arm rather than the
    // default-handle arm below.
    //
    // Why the direct implementation is wrong: this arm used to require both
    // bindings, and the omitting shape was re-dispatched with the literal tokens
    // `preset: $crate::NoPreset, parts: $crate::NoParts`. Those tokens then met
    // `stringify!` and recorded `"$crate :: NoPreset"` instead of `"NoPreset"` —
    // one default, two names, and exactly the compiled/owned divergence B2 set
    // out to remove. Resolving each omitted binding through
    // `__face_ty_or!`/`__face_ty_name_or!` takes the type and the name from one
    // place and lets this arm terminate the expansion instead of re-dispatching
    // into itself forever.
    // Boundary: `handle` stays required; `preset`/`parts` resolve independently,
    // so a written one is forwarded verbatim and an omitted one becomes
    // `NoPreset`/`NoParts`.
    // Pinned by `run_method/tests/face_arm_defaults.rs` and
    // `run_method/tests/face_preset_parts.rs`.
    // 使用自定义 handle 的精简写法。`preset` 与 `parts` 各自可选，因此本 arm 拥有所有
    // 带 handle 的形态：两个都不写、只写其一、或两个都写。`handle` 是 `kind` 之后唯一
    // 保持必填的字段——这正是它成为 handle arm、而不是下面那个默认 handle arm 的原因。
    //
    // 直白写法错在哪：本 arm 过去要求两个绑定都写，省略的形态被回派时塞进字面 token
    // `preset: $crate::NoPreset, parts: $crate::NoParts`，它们随后撞上 `stringify!`，
    // 记录成 `"$crate :: NoPreset"` 而不是 `"NoPreset"`——同一默认值两个名字，正是 B2
    // 要消除的编译期/owned 分歧。让每个省略绑定经
    // `__face_ty_or!`/`__face_ty_name_or!` 解析，类型与名字便出自一处，且本 arm 直接终结
    // 展开，不再无限回派进自身。
    // 边界：`handle` 仍必填；`preset`/`parts` 各自独立解析，写下的原样转发，省略的变成
    // `NoPreset`/`NoParts`。
    // 由 `run_method/tests/face_arm_defaults.rs` 与
    // `run_method/tests/face_preset_parts.rs` 钉住。
    {
        collector: $collector:ident,
        kind: $kind:ident,
        $(preset: $preset:ty,)?
        $(parts: $parts:ty,)?
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
            preset: $crate::__face_ty_or!($crate::NoPreset; $($preset)?),
            preset_name: $crate::__face_ty_name_or!("NoPreset"; $($preset)?),
            parts: $crate::__face_ty_or!($crate::NoParts; $($parts)?),
            parts_name: $crate::__face_ty_name_or!("NoParts"; $($parts)?),
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
            // The author's expression wins; an omitted rule on a
            // registry-owning face resolves to the canonical sibling rule
            // module, and every other face keeps `ANY`. The resolver is a proc
            // macro because the canonical spelling is a *relative* path.
            // 作者写下的表达式优先；拥有注册机的面省略规则时解析到同目录的规范
            // 规则模块，其余面保留 `ANY`。解析器是过程宏，因为规范写法是**相对**路径。
            registry_rule: $crate::__face_rule_or!(
                $crate::RegistrationRule::ANY;
                $($needs_registry)?;
                $($rule)?
            ),
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
        // Why the direct implementation is wrong: this arm is the only one that
        // accepts a face with no `handle:` field, so it also has to accept the
        // `preset:`/`parts:` the author may have written above it. Every earlier
        // arm requires `handle`, so a custom preset/parts declaration without a
        // handle lands here. Hardcoding the defaults in this expansion silently
        // rewrites the author's types and their names in `REGISTRATION`; the
        // matcher already binds them, which is exactly what makes the mismatch
        // invisible to the compiler.
        // Boundary: presence is decided per binding. This arm's matcher, like the
        // custom-handle arm above, makes `preset` and `parts` independently
        // optional, and both resolve an omitted binding through the same
        // `__face_ty_or!`/`__face_ty_name_or!` pair, so an omitted one keeps
        // `NoPreset`/`NoParts` and a written one is forwarded verbatim.
        // Pinned by `run_method/tests/face_preset_parts.rs` and
        // `run_method/tests/face_arm_defaults.rs`.
        // 直白写法错在哪：本 arm 是唯一接受“没有 `handle:` 字段”的注册面的分支，
        // 因此也必须接受作者在上面写下的 `preset:`/`parts:`。更早的 arm 都要求
        // `handle`，所以不带 handle 的自定义 preset/parts 声明会落到这里。在展开里写死
        // 默认值会静默改写作者的类型及其在 `REGISTRATION` 里的名字；而匹配器其实已经绑定了
        // 它们——这正是编译器看不出这处不一致的原因。
        // 边界：按每个绑定单独判断有无。本 arm 的匹配器与上面的自定义 handle arm 一样，
        // 允许 preset 与 parts 各自省略，且两者都经同一对
        // `__face_ty_or!`/`__face_ty_name_or!` 解析省略绑定，因此省略者保持
        // `NoPreset`/`NoParts`，写下的原样转发。
        // 由 `run_method/tests/face_preset_parts.rs` 与
        // `run_method/tests/face_arm_defaults.rs` 钉住。
        $crate::__registration_face! {
            source: $crate::registry_core::manifest_relative_source(env!("CARGO_MANIFEST_DIR"), file!()),
            collector: $collector,
            kind: $kind,
            preset: $crate::__face_ty_or!($crate::NoPreset; $($preset)?),
            preset_name: $crate::__face_ty_name_or!("NoPreset"; $($preset)?),
            parts: $crate::__face_ty_or!($crate::NoParts; $($parts)?),
            parts_name: $crate::__face_ty_name_or!("NoParts"; $($parts)?),
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
            // The author's expression wins; an omitted rule on a
            // registry-owning face resolves to the canonical sibling rule
            // module, and every other face keeps `ANY`. The resolver is a proc
            // macro because the canonical spelling is a *relative* path.
            // 作者写下的表达式优先；拥有注册机的面省略规则时解析到同目录的规范
            // 规则模块，其余面保留 `ANY`。解析器是过程宏，因为规范写法是**相对**路径。
            registry_rule: $crate::__face_rule_or!(
                $crate::RegistrationRule::ANY;
                $($needs_registry)?;
                $($rule)?
            ),
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

    // A kind-only face is not a separate arm: the previous arm makes every
    // field after `kind` optional, so `{ collector, kind }` already matches
    // there and a later smallest-face arm can never fire. Keeping it would
    // suggest a reachable spelling that in fact takes the arm above.
    // 只有 kind 的注册面不需要单独的 arm：上一个 arm 让 `kind` 之后的每个字段都可省，
    // 因此 `{ collector, kind }` 已经在那里匹配，后面的“最小注册面”arm 永远不会触发。
    // 留着它只会暗示存在一种实际会走上一个 arm 的可达写法。

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
