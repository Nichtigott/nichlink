//! The external-crate registration-face declaration form.
//! 外部 crate 的注册面声明形式。

/// `kind` is the handle-marker type this file declares (`pub struct <Kind>;`), so
/// write that type first and reference it here: an editor cannot complete a name
/// the author has not written yet, and `kind` is captured as an identifier
/// rather than an expression, which is also why value completion does not fire
/// there. Every other field completes normally.
/// `kind` 就是本文件声明的那个 handle 标记类型（`pub struct <Kind>;`）：先写出该类型，
/// 再在这里引用它。编辑器无法补全一个作者还没写下的名字，而且 `kind` 是以标识符而非
/// 表达式捕获的——这也是它的值位不会弹候选的原因。其余字段的值都能正常补全。
/// Declare a registration face: `kind` is the only required field; every other
/// field is optional and defaults the way the generated compact form defaults
/// them — `source` to this file, `preset`/`parts` to `NoPreset`/`NoParts`,
/// `handle` to `kind`, `name`/`params` to the kind's spelling, `summary` to
/// empty, `exports`/`requires`/`provides`/`runtime_checks` to empty,
/// `needs_registry` to `false`, `registry_name` to the module's last segment,
/// `parent` to the package's root, `getting_from_other_registry` to `None`,
/// `registry_rule_path` to this file, and `registry_rule` to
/// `RegistrationRule::ANY` (see the comment in the expansion).
/// 声明一个注册面：只有 `kind` 必填，其余字段都可省略，并按生成的紧凑形式取默认值——
/// `source` 取本文件、`preset`/`parts` 取 `NoPreset`/`NoParts`、`handle` 取 `kind`、
/// `name`/`params` 取 kind 的拼写、`summary` 取空、`exports`/`requires`/`provides`/
/// `runtime_checks` 取空、`needs_registry` 取 `false`、`registry_name` 取模块名末段、
/// `parent` 取包根、`getting_from_other_registry` 取 `None`、`registry_rule_path` 取
/// 本文件、`registry_rule` 取 `RegistrationRule::ANY`（原因见展开处的注释）。
/// Declare a registration face owned by an external crate.
/// 声明由外部 crate 所有的注册面。
///
/// Unlike generated parent-specific macros, this form receives its source path
/// explicitly; an external crate is not part of the host's generated tree.
/// 与生成的父级专属宏不同，此形式显式接收源码路径；外部 crate 不在宿主
/// 自动生成的模块树中。
///
/// An external face is written with braces like every other face, so the editor is
/// told to insert them.
/// 外部注册面与其它注册面一样用花括号书写，因此这里告诉编辑器插入花括号。
#[rust_analyzer::macro_style(braces)]
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
    };
    { @tokens $($tokens:tt)* } => {
        $crate::face_fields! { @external collector: linked, $($tokens)* }
    };
    { $($tokens:tt)* } => {
        $crate::external_object! { @tokens $($tokens)* }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __external_object {
    // Arm 1: the author wrote `handle:`, so the declared marker type is used for
    // the compile-time interface assertions. This arm must come first because
    // the next arm's matcher has no `handle` slot at all; a declaration that
    // does carry one only matches here.
    // Arm 1：作者写了 `handle:`，因此编译期接口断言使用他声明的标记类型。本 arm
    // 必须排在前面，因为下一个 arm 的 matcher 根本没有 `handle` 槽位；带了该字段的
    // 声明只会在这里匹配。
    {
        collector: $collector:ident,
        $(source: $source:expr,)?
        kind: $kind:ident,
        $(preset: $preset:ty,)?
        $(parts: $parts:ty,)?
        $(name: { zh: $name_zh:expr, en: $name_en:expr $(,)? },)?
        $(summary: { zh: $summary_zh:expr, en: $summary_en:expr $(,)? },)?
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
            source: $crate::__face_expr_or!(
                $crate::registry_core::manifest_relative_source(env!("CARGO_MANIFEST_DIR"), file!());
                $($source)?
            ),
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
            // Why the generated compact arm's default is wrong here: that arm
            // omits the rule on a registry-owning face by resolving
            // `super::registry_rule::REGISTRATION_RULE`, a *relative* path that
            // only exists because the build generated a sibling rule module
            // beside the face. An external crate is outside the host's generated
            // tree, so that sibling does not exist and the same default would not
            // compile at all. The boundary is therefore the crate: an external
            // face that owns a registry and omits the field gets the permissive
            // `ANY` and must name its rule explicitly to narrow it.
            // 生成的紧凑 arm 的默认值在这里为什么是错的：那个 arm 对拥有注册机的
            // 面省略规则时，解析到 `super::registry_rule::REGISTRATION_RULE`——一个
            // **相对**路径，它成立只因为构建步骤在注册面旁边生成了兄弟规则模块。
            // 外部 crate 位于宿主生成树之外，这个兄弟模块并不存在，照搬该默认值根本
            // 编译不过。因此边界就是 crate 本身：拥有注册机却省略该字段的外部面得到
            // 宽松的 `ANY`，要收窄必须显式写出规则。
            // Pinned by `run_method/tests/external_compact_face.rs`.
            // 由 `run_method/tests/external_compact_face.rs` 钉住。
            registry_rule: $crate::__face_value_or!($crate::RegistrationRule::ANY; [$($rule)?]),
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
    // Arm 2: no `handle` was written. A `macro_rules!` matcher cannot default an
    // `ident` slot through a helper macro — matching happens before expansion, so
    // `handle: __face_ident_or!(…)` reaches `__registration_face!` as a macro
    // call, which no `$handle:ident` can match. The only place the ident can be
    // chosen is another matcher, so this arm re-dispatches to arm 1 with
    // `handle: $kind` inserted at its canonical slot and forwards every other
    // binding verbatim.
    // Arm 2：没有写 `handle`。`macro_rules!` 无法经辅助宏为 `ident` 槽位补默认值——
    // 匹配发生在展开之前，`handle: __face_ident_or!(…)` 到 `__registration_face!`
    // 时是宏调用，任何 `$handle:ident` 都匹配不上。能挑选这个标识符的地方只有另一个
    // matcher，因此本 arm 在 `handle` 的规范槽位插入 `handle: $kind` 后回派到 arm 1，
    // 其余每个绑定原样转发。
    // Pinned by `run_method/tests/external_compact_face.rs`.
    // 由 `run_method/tests/external_compact_face.rs` 钉住。
    {
        collector: $collector:ident,
        $(source: $source:expr,)?
        kind: $kind:ident,
        $(preset: $preset:ty,)?
        $(parts: $parts:ty,)?
        $(name: { zh: $name_zh:expr, en: $name_en:expr $(,)? },)?
        $(summary: { zh: $summary_zh:expr, en: $summary_en:expr $(,)? },)?
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
        $crate::__external_object! {
            collector: $collector,
            $(source: $source,)?
            kind: $kind,
            $(preset: $preset,)?
            $(parts: $parts,)?
            $(name: { zh: $name_zh, en: $name_en },)?
            $(summary: { zh: $summary_zh, en: $summary_en },)?
            $(params: $params,)?
            $(exports: [$($export),*],)?
            handle: $kind,
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
}
