//! A face that writes `handle:` must record the same default `preset`/`parts`
//! names as a face that omits `handle:`, and it must be able to state either
//! binding alone.
//! 写了 `handle:` 的注册面必须与省略 `handle:` 的注册面记录相同的默认
//! `preset`/`parts` 名字，并且必须能单独写出其中任意一个绑定。

/// A custom preset whose recorded name is distinguishable from the default.
/// 一个自定义 preset，其记录名可与默认值区分。
struct ProbePreset;

impl nichlink_run_method::PresetContract for ProbePreset {
    type Output = ();
    const REQUIRED_PARTS: &'static [&'static str] = &["probe"];
}

/// A custom parts whose recorded name is distinguishable from the default.
/// 一个自定义 parts，其记录名可与默认值区分。
struct ProbeParts;

impl nichlink_run_method::PartsContract for ProbeParts {
    type Output = ();
    const PROVIDED_PARTS: &'static [&'static str] = &["probe"];
}

/// `handle:` with neither `preset:` nor `parts:`. This is the shape all three
/// example faces take, and the one that used to record `"$crate :: NoPreset"`
/// and `"$crate :: NoParts"`: the defaulting arm re-dispatched those literal
/// tokens and the arm that received them `stringify!`-ed the tokens instead of
/// naming the default.
/// 写了 `handle:`、既没写 `preset:` 也没写 `parts:`。三个例子面都是这个形态，也是
/// 过去记录成 `"$crate :: NoPreset"` 与 `"$crate :: NoParts"` 的形态：取默认的 arm
/// 把这两个字面 token 回派出去，接住它们的 arm 对 token 做了 `stringify!`，而不是命名
/// 默认值。
mod handle_without_preset_or_parts {
    nichlink_run_method::__control_object! {
        collector: development,
        kind: HandleDefaults,
        handle: HandleDefaults,
    }
}

/// The no-`handle` counterpart of the shape above. Both arms must land on the
/// same default names, so the test compares them instead of trusting each in
/// isolation.
/// 上面形态的无 `handle` 版本。两个 arm 必须落到同一组默认名字上，因此测试直接比较
/// 二者，而不是各自单独相信。
mod without_handle_defaults {
    nichlink_run_method::__control_object! {
        collector: development,
        kind: NoHandleDefaults,
    }
}

/// `handle:` with `preset:` only: the written name survives and `parts` falls
/// back to the default.
/// 写了 `handle:` 且只写 `preset:`：写下的名字保留，`parts` 回退到默认值。
mod handle_with_preset_only {
    use super::ProbePreset;

    nichlink_run_method::__control_object! {
        collector: development,
        kind: HandlePresetOnly,
        preset: ProbePreset,
        handle: HandlePresetOnly,
    }
}

/// `handle:` with `parts:` only, the mirror of the shape above.
/// 写了 `handle:` 且只写 `parts:`，是上一形态的镜像。
mod handle_with_parts_only {
    use super::ProbeParts;

    nichlink_run_method::__control_object! {
        collector: development,
        kind: HandlePartsOnly,
        parts: ProbeParts,
        handle: HandlePartsOnly,
    }
}

/// `handle:` with both bindings written.
/// 写了 `handle:` 且两个绑定都写下。
mod handle_with_preset_and_parts {
    use super::{ProbeParts, ProbePreset};

    nichlink_run_method::__control_object! {
        collector: development,
        kind: HandleBoth,
        preset: ProbePreset,
        parts: ProbeParts,
        handle: HandleBoth,
    }
}

/// F1 regression pin: an omitted binding is named `NoPreset`/`NoParts`, never
/// the expanded tokens the default type is spelled with.
/// F1 回归钉：省略的绑定命名为 `NoPreset`/`NoParts`，绝不是默认类型写出来时那串
/// 展开后的 token。
#[test]
fn a_handle_face_records_the_plain_default_names() {
    assert_eq!(
        handle_without_preset_or_parts::REGISTRATION.preset,
        "NoPreset"
    );
    assert_eq!(
        handle_without_preset_or_parts::REGISTRATION.parts,
        "NoParts"
    );
    // Both handle arms must agree with the no-handle arm on the default names:
    // that equivalence, not either value alone, is the regression.
    // 两个带 handle 的 arm 都必须在默认名字上与无 handle 的 arm 一致：这条等价关系——
    // 而不是任一单独取值——才是回归点。
    assert_eq!(
        handle_without_preset_or_parts::REGISTRATION.preset,
        without_handle_defaults::REGISTRATION.preset
    );
    assert_eq!(
        handle_without_preset_or_parts::REGISTRATION.parts,
        without_handle_defaults::REGISTRATION.parts
    );
    assert_eq!(without_handle_defaults::REGISTRATION.preset, "NoPreset");
    assert_eq!(without_handle_defaults::REGISTRATION.parts, "NoParts");
    // The contract must read the default types too, so the fix is not only a
    // label change.
    // 合同也必须读取默认类型，因此这处修复不只是标签变化。
    assert!(
        handle_without_preset_or_parts::REGISTRATION
            .contract
            .required_parts
            .is_empty()
    );
    assert!(
        handle_without_preset_or_parts::REGISTRATION
            .contract
            .provided_parts
            .is_empty()
    );
}

/// F4: either binding may be written alone beside a custom `handle`; the
/// omitted one still takes the default.
/// F4：两个绑定都可以在自定义 `handle` 旁单独书写；省略的那个仍然取默认值。
#[test]
fn either_binding_may_be_written_alone_beside_a_handle() {
    assert_eq!(handle_with_preset_only::REGISTRATION.preset, "ProbePreset");
    assert_eq!(handle_with_preset_only::REGISTRATION.parts, "NoParts");
    assert_eq!(handle_with_parts_only::REGISTRATION.preset, "NoPreset");
    assert_eq!(handle_with_parts_only::REGISTRATION.parts, "ProbeParts");
}

/// Both bindings written beside a `handle`: the explicit names and their
/// contracts survive.
/// 在 `handle` 旁把两个绑定都写下：显式名字及其合同都保留。
#[test]
fn both_bindings_written_beside_a_handle_survive() {
    assert_eq!(
        handle_with_preset_and_parts::REGISTRATION.preset,
        "ProbePreset"
    );
    assert_eq!(
        handle_with_preset_and_parts::REGISTRATION.parts,
        "ProbeParts"
    );
    assert_eq!(
        handle_with_preset_and_parts::REGISTRATION
            .contract
            .required_parts,
        ["probe"]
    );
}
