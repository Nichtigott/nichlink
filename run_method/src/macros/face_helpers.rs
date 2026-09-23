//! Field-value helper macros for the face registration ladder.
//! 注册面宏阶梯的字段取值辅助宏。

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

/// Pick an optional type, falling back when the author omitted it.
/// 取一个可选类型；作者省略时回退到默认值。
///
/// A `macro_rules!` arm cannot splice `$($ty)?` directly into type position —
/// the absent case would leave a dangling `preset:` with no type — so the arm
/// passes the bracketed capture through this helper, which has one arm per
/// presence. `$fallback` must be a type, not an expression, because a custom
/// preset may be generic.
/// `macro_rules!` 的 arm 不能把 `$($ty)?` 直接拼进类型位置——省略时 `preset:`
/// 后面会什么都没有——因此 arm 把带括号的捕获交给这个辅助宏，由它按“有无”各出一个
/// arm。`$fallback` 必须是类型而不是表达式，因为自定义 preset 可能是泛型。
#[doc(hidden)]
#[macro_export]
macro_rules! __face_ty_or {
    ($fallback:ty;) => {
        $fallback
    };
    ($fallback:ty; $value:ty) => {
        $value
    };
}

/// Name an optional type for the registration record.
/// 为注册记录命名一个可选类型。
///
/// The registration record stores `stringify!` of the author's spelling, so the
/// default and the explicit form must be distinguishable in the record. Dropping
/// the binding entirely would make `preset` read `NoPreset` even when the author
/// wrote something else, which is the bug that `face_preset_parts.rs` pins.
/// 注册记录保存作者写法的 `stringify!`，因此默认值与显式写法必须在记录里可区分。
/// 完全丢掉绑定会让作者明明写了别的类型、`preset` 却读作 `NoPreset`，这正是
/// `face_preset_parts.rs` 钉住的缺陷。
#[doc(hidden)]
#[macro_export]
macro_rules! __face_ty_name_or {
    ($fallback:expr;) => {
        $fallback
    };
    ($fallback:expr; $value:ty) => {
        stringify!($value)
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
