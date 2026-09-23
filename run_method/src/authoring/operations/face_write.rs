//! Field-write policy and the one applier both add and edit reach.
//! 字段写入策略，以及 add 与 edit 共用的唯一应用器。

use super::*;

/// How the shared applier writes values into a manifest.
/// 共享应用器如何把字段写入清单。
#[derive(Clone, Copy)]
pub(super) enum FaceWrite {
    /// Creating a face: trim each value and keep the manifest default when blank.
    /// 创建注册面：裁剪每个值；为空时保留清单默认值。
    Create,
    /// Editing a face: write every value verbatim, blank included.
    /// 编辑注册面：逐字写入每个值，包括空值。
    Edit,
}

impl FaceWrite {
    /// The field order this path has always used, kept verbatim so the first
    /// validation error it reports does not move.
    /// 本路径一直使用的字段顺序，逐字保留，避免它报告的首个校验错误发生变化。
    pub(super) fn field_order(self) -> &'static [&'static str] {
        match self {
            Self::Create => CREATE_FIELD_ORDER,
            Self::Edit => EDIT_FIELD_ORDER,
        }
    }
}

/// The add path's field order.
/// add 路径的字段顺序。
const CREATE_FIELD_ORDER: &[&str] = &[
    "kind",
    "preset",
    "parts",
    "name_zh",
    "name_en",
    "summary_zh",
    "summary_en",
    "params",
    "exports",
    "handle",
    "stable_name",
    "getting_from_other_registry",
    "handle_contracts",
    "part_contracts",
    "requires",
    "provides",
    "expected_output",
    "actual_output",
    "runtime_checks",
    "flow",
    "flow_provider",
    "handle_traits",
    "part_traits",
    "needs_registry",
    "registry_name",
    "registration_rule",
    "admission",
];

/// The edit path's field order.
/// edit 路径的字段顺序。
const EDIT_FIELD_ORDER: &[&str] = &[
    "kind",
    "preset",
    "parts",
    "name_zh",
    "name_en",
    "summary_zh",
    "summary_en",
    "params",
    "exports",
    "handle",
    "stable_name",
    "needs_registry",
    "registry_name",
    "getting_from_other_registry",
    "registration_rule",
    "admission",
    "handle_contracts",
    "part_contracts",
    "handle_traits",
    "part_traits",
    "requires",
    "provides",
    "expected_output",
    "actual_output",
    "runtime_checks",
    "flow",
    "flow_provider",
];

/// Write all 28 shared fields into `face`. Both public entry points reach the
/// field-by-field handling below through this one loop, so a new field is wired
/// once instead of once per struct.
/// 将全部 28 个共用字段写入 `face`。两个公开入口都经由这一个循环抵达下面的逐字段
/// 处理，因此新增字段只需接一次线，而不是每个结构体各接一次。
pub(super) fn apply_module_face_values(
    face: &mut FaceManifest,
    values: &ModuleFaceValues<'_>,
    write: FaceWrite,
) -> Result<(), String> {
    for field in write.field_order() {
        apply_face_value(face, values, field, write)?;
    }
    Ok(())
}

/// One field's conversion, validation, and application.
/// 单个字段的转换、校验与应用。
pub(super) fn apply_face_value(
    face: &mut FaceManifest,
    values: &ModuleFaceValues<'_>,
    field: &str,
    write: FaceWrite,
) -> Result<(), String> {
    match field {
        "kind" => edit_kind(face, values.kind, write),
        "needs_registry" => face.edit("needs_registry", &values.needs_registry.to_string()),
        // Trait labels are derived from contract paths, and both paths share the
        // same editing rule, so the contract helper is the whole handling.
        // trait 标签由契约路径推导；两条路径的编辑规则相同，因此契约辅助函数
        // 就是全部处理。
        "handle_traits" => apply_trait_contract(
            face,
            "handle_traits",
            values.handle_traits,
            values.handle_contracts,
        ),
        "part_traits" => apply_trait_contract(
            face,
            "part_traits",
            values.part_traits,
            values.part_contracts,
        ),
        // These are validated and written on every edit, blank included.
        // 这些字段每次编辑都会校验并写入，包括空值。
        "registration_rule" | "admission" => edit_required(face, field, values.value(field), write),
        _ => edit_optional(face, field, values.value(field), write),
    }
}

/// Write the kind after normalizing it. A blank create keeps the kind derived
/// from the module name; a blank edit is still validated and reported.
/// 规范化后写入 kind。创建时的空值保留由模块名派生的 kind；编辑时的空值仍会
/// 校验并报错。
pub(super) fn edit_kind(
    face: &mut FaceManifest,
    kind: &str,
    write: FaceWrite,
) -> Result<(), String> {
    if matches!(write, FaceWrite::Create) && kind.trim().is_empty() {
        return Ok(());
    }
    face.edit("kind", &normalize_kind_name(kind.trim()))
}

/// Write a field that may legitimately be blank. A create trims and skips a
/// blank value so the manifest default survives; an edit writes it through.
/// 写入允许为空的字段。创建裁剪并跳过空值，保留清单默认值；编辑则如实写入。
pub(super) fn edit_optional(
    face: &mut FaceManifest,
    field: &str,
    value: &str,
    write: FaceWrite,
) -> Result<(), String> {
    match write {
        FaceWrite::Create => {
            let value = value.trim();
            if value.is_empty() {
                Ok(())
            } else {
                face.edit(field, value)
            }
        }
        FaceWrite::Edit => face.edit(field, value),
    }
}

/// Write a field the face always carries. A create trims it; an edit keeps it
/// exactly as the patch delivered it.
/// 写入注册面始终携带的字段。创建裁剪它；编辑按补丁原样保留。
pub(super) fn edit_required(
    face: &mut FaceManifest,
    field: &str,
    value: &str,
    write: FaceWrite,
) -> Result<(), String> {
    match write {
        FaceWrite::Create => face.edit(field, value.trim()),
        FaceWrite::Edit => face.edit(field, value),
    }
}

pub(super) fn apply_trait_contract(
    face: &mut FaceManifest,
    label_field: &str,
    labels: &str,
    contract_paths: &str,
) -> Result<(), String> {
    if labels.trim().is_empty() && contract_paths.trim().is_empty() {
        return Ok(());
    }
    let labels = if contract_paths.trim().is_empty() {
        labels.trim().to_owned()
    } else {
        trait_names_from_paths(contract_paths)?
    };
    face.edit(label_field, &labels)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build an add request and an edit patch carrying the same 28 values, so one
    /// fixture drives both public shapes and proves they stay in step.
    /// 构建携带同一组 28 个取值的 add 请求与 edit 补丁；一份夹具同时驱动两个公开
    /// 结构体，证明二者保持一致。
    macro_rules! shared_face_fields {
        ($($field:ident = $value:expr),* $(,)?) => {
            (
                NewModuleFace {
                    $($field: $value,)*
                    parent: ROOT_NODE_ID,
                },
                ModuleFacePatch {
                    $($field: $value,)*
                },
            )
        };
    }

    /// One fully populated pair of shared values.
    /// 一组完整填充的共用取值。
    fn fixture() -> (NewModuleFace<'static>, ModuleFacePatch<'static>) {
        shared_face_fields!(
            module = "widget",
            kind = "Widget",
            preset = "NoPreset",
            parts = "NoParts",
            name_zh = "部件",
            name_en = "Widget",
            summary_zh = "摘要",
            summary_en = "Summary",
            params = "WidgetParams",
            exports = "a,b",
            handle = "Widget",
            stable_name = "widget",
            needs_registry = false,
            registry_name = "widget",
            getting_from_other_registry = "",
            registration_rule = "ANY",
            admission = "ANY",
            handle_traits = "",
            handle_contracts = "",
            part_traits = "",
            part_contracts = "",
            requires = "",
            provides = "widget",
            expected_output = "()",
            actual_output = "()",
            runtime_checks = "",
            flow = "",
            flow_provider = "",
        )
    }

    /// The manifest an add starts from, before any field is applied.
    /// add 在应用任何字段之前所依据的清单。
    fn blank_manifest() -> FaceManifest {
        FaceManifest::new(
            "widget",
            "Widget",
            ROOT_NODE_ID,
            "<root>",
            "root",
            "widget/widget.rs",
        )
    }

    /// The 28 shared fields are handled once: both public shapes flow through the
    /// same applier, so identical values produce identical faces.
    /// 28 个共用字段只处理一次：两个公开结构体流经同一个应用器，相同取值产生相同
    /// 注册面。
    #[test]
    fn identical_values_reach_the_same_face_from_add_and_edit() {
        let (request, patch) = fixture();
        let new_values = ModuleFaceValues::from_new(&request);
        let patch_values = ModuleFaceValues::from_patch(&patch);
        for field in CREATE_FIELD_ORDER
            .iter()
            .copied()
            .chain(EDIT_FIELD_ORDER.iter().copied())
        {
            assert_eq!(
                new_values.value(field),
                patch_values.value(field),
                "shared field `{field}`"
            );
        }

        let mut created = blank_manifest();
        apply_module_face_values(&mut created, &new_values, FaceWrite::Create)
            .expect("create applies");
        let mut edited = blank_manifest();
        apply_module_face_values(&mut edited, &patch_values, FaceWrite::Edit)
            .expect("edit applies");
        assert_eq!(created.values, edited.values);
    }

    /// A blank add value keeps the manifest default, while a blank edit value is
    /// written through: the two rules the public structs used to carry separately.
    /// 创建时的空值保留清单默认值，编辑时的空值则如实写入：公开结构体过去各自
    /// 携带的两条规则。
    #[test]
    fn a_blank_create_keeps_defaults_while_a_blank_edit_writes_them_through() {
        let (request, patch) = fixture();
        let blank_request = NewModuleFace {
            preset: "",
            ..request
        };
        let blank_patch = ModuleFacePatch {
            preset: "",
            ..patch
        };

        let mut created = blank_manifest();
        apply_module_face_values(
            &mut created,
            &ModuleFaceValues::from_new(&blank_request),
            FaceWrite::Create,
        )
        .expect("create applies");
        assert_eq!(
            created.values.get("preset").map(String::as_str),
            Some("NoPreset")
        );

        let mut edited = blank_manifest();
        apply_module_face_values(
            &mut edited,
            &ModuleFaceValues::from_patch(&blank_patch),
            FaceWrite::Edit,
        )
        .expect("edit applies");
        assert_eq!(edited.values.get("preset").map(String::as_str), Some(""));
    }
}
