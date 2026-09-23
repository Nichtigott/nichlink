//! A face declared without a `handle:` must keep the preset and parts its
//! author wrote.
//! 未写 `handle:` 的注册面必须保留作者写下的 preset 与 parts。

/// A custom preset and parts pair, distinct from the permissive defaults so the
/// assertion can tell "forwarded" from "silently replaced".
/// 一对自定义 preset/parts，刻意不同于宽松默认值，断言才能区分"被转发"与"被静默替换"。
struct ProbePreset;

impl nichlink_run_method::PresetContract for ProbePreset {
    type Output = ();
    const REQUIRED_PARTS: &'static [&'static str] = &["probe"];
}

struct ProbeParts;

impl nichlink_run_method::PartsContract for ProbeParts {
    type Output = ();
    const PROVIDED_PARTS: &'static [&'static str] = &["probe"];
}

/// No `handle:` field: this is the arm that defaults the handle to `kind`.
/// 没有 `handle:` 字段：这正是把 handle 默认为 `kind` 的那个 arm。
mod probe {
    use super::{ProbeParts, ProbePreset};

    nichlink_run_method::__control_object! {
        collector: development,
        kind: ProbeFace,
        preset: ProbePreset,
        parts: ProbeParts,
        name: { zh: "探针面", en: "Probe face" },
    }
}

/// The record must name the author's types, not the macro's defaults.
/// 记录必须写出作者的类型，而不是宏的默认值。
#[test]
fn a_custom_preset_and_parts_survive_the_defaulting_arm() {
    assert_eq!(probe::REGISTRATION.preset, "ProbePreset");
    assert_eq!(probe::REGISTRATION.parts, "ProbeParts");
    // The contract reads the same types, so this is not just a label.
    // 合同读的是同一批类型，因此这不只是标签。
    assert_eq!(probe::REGISTRATION.contract.required_parts, ["probe"]);
    assert_eq!(probe::REGISTRATION.contract.provided_parts, ["probe"]);
}
