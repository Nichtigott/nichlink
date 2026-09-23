//! New-project, add/edit, and plugin form state.
//! 新建项目、添加/编辑与插件表单状态。

use std::collections::{BTreeMap, BTreeSet};

use nichlink_run_method::{FACE_FIELD_COUNT, NodeId};

/// Fields used by the New Project wizard.
/// New Project 向导使用的字段。
#[derive(Clone, Debug)]
pub struct NewProjectState {
    /// Raw wizard input: directory, then package name, then project kind.
    /// 向导原始输入：目录，然后是包名，最后是项目类型。
    pub values: [String; 3],
    /// Index of the focused row, matching `values`.
    /// 当前聚焦行在 `values` 中的下标。
    pub field: usize,
    /// Whether the focused row is accepting typed input.
    /// 当前行是否正在接受键入。
    pub editing: bool,
}

impl NewProjectState {
    pub(crate) fn new() -> Self {
        Self {
            values: [
                "./nichlink-app".to_owned(),
                "nichlink-app".to_owned(),
                "binary".to_owned(),
            ],
            field: 0,
            editing: false,
        }
    }
}

/// Add/edit form state for one registration face.
/// 单个注册面的添加/编辑表单状态。
#[derive(Clone, Debug)]
pub struct AddState {
    /// One text value per canonical face field index.
    /// 每个规范注册面字段下标对应一个文本值。
    pub values: [String; FACE_FIELD_COUNT],
    /// Index of the focused field, as a canonical field index.
    /// 当前聚焦字段的规范字段下标。
    pub field: usize,
    /// Whether the focused field is accepting typed input.
    /// 当前字段是否正在接受键入。
    pub editing: bool,
    /// Fields made mandatory by the selected parent Registry contract.
    /// 由所选父 Registry 合同提升为必填的字段及其具体要求。
    pub parent_requirements: BTreeMap<usize, String>,
    /// Derived fields shown for context but not accepted as free-form input.
    /// 仅用于说明、不能自由输入的派生字段。
    pub locked_fields: BTreeSet<usize>,
}

/// Stable, task-oriented order used by both Add and Edit.
/// Add 与 Edit 共用的稳定任务顺序；所有字段始终可见，不再切换表单形态。
pub(crate) const FACE_FORM_ORDER: [usize; FACE_FIELD_COUNT] = [
    0, 1, 8, 9, 10, 11, 12, 16, 2, 3, 4, 5, 17, 18, 13, 6, 14, 15, 20, 19, 29, 21, 7, 22, 23, 24,
    25, 27, 28, 26,
];

pub(crate) fn face_field_indices(_add: &AddState) -> Vec<usize> {
    FACE_FORM_ORDER.to_vec()
}

pub(crate) fn move_face_field(add: &mut AddState, step: isize) {
    let position = FACE_FORM_ORDER
        .iter()
        .position(|field| *field == add.field)
        .unwrap_or_default();
    let next = position
        .saturating_add_signed(step)
        .min(FACE_FORM_ORDER.len().saturating_sub(1));
    add.field = FACE_FORM_ORDER[next];
}

/// Plugin selection form. The UI writes one crate anchor, never an object list.
/// 插件选择表单。界面只写入一个 crate 锚点，不维护对象清单。
#[derive(Clone, Debug)]
pub struct PluginState {
    /// Raw plugin input: source, framework, package, version, crate, checksum, mode.
    /// 插件原始输入：source、framework、package、version、crate、checksum、mode。
    pub values: [String; 7],
    /// Index of the focused row, matching `values`.
    /// 当前聚焦行在 `values` 中的下标。
    pub field: usize,
    /// Whether the focused row is accepting typed input.
    /// 当前行是否正在接受键入。
    pub editing: bool,
}

impl PluginState {
    pub(crate) fn new() -> Self {
        Self {
            values: [
                "official".to_owned(),
                "nichlink.default".to_owned(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                "extension".to_owned(),
            ],
            field: 0,
            editing: false,
        }
    }
}

impl AddState {
    pub(crate) fn new(parent: NodeId) -> Self {
        Self {
            values: [
                parent.to_string(),
                String::new(),
                "false".to_owned(),
                String::new(),
                "ANY".to_owned(),
                "ANY".to_owned(),
                "NoParts".to_owned(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                // preset, params, handle, stable name, external registry,
                // rule path, handle traits, handle contracts, part traits,
                // requires, provides
                "NoPreset".to_owned(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                "()".to_owned(),
                "()".to_owned(),
                // runtime checks, explicit flow, flow provider
                String::new(),
                String::new(),
                String::new(),
                // part contracts
                String::new(),
            ],
            field: 0,
            editing: false,
            parent_requirements: BTreeMap::new(),
            locked_fields: BTreeSet::from([18, 19, 21]),
        }
    }

    pub(crate) fn is_editable(&self, field: usize) -> bool {
        !self.locked_fields.contains(&field)
    }

    pub(crate) fn apply_parent_rule(&mut self, rule: &nichlink_run_method::OwnedRegistrationRule) {
        self.parent_requirements.clear();
        if let Some(preset) = &rule.required_preset {
            self.values[13] = preset.clone();
            self.parent_requirements
                .insert(13, format!("preset `{preset}`"));
        }
        if !rule.required_parts.is_empty() {
            self.parent_requirements.insert(
                6,
                format!("parts providing {}", rule.required_parts.join(", ")),
            );
        }
        if !rule.required_exports.is_empty() {
            self.values[7] = rule.required_exports.join(",");
            self.parent_requirements
                .insert(7, format!("exports {}", rule.required_exports.join(", ")));
        }
        if !rule.required_handle_traits.is_empty() {
            self.parent_requirements.insert(
                20,
                format!(
                    "handle implements {}",
                    rule.required_handle_traits.join(", ")
                ),
            );
        }
        if !rule.required_part_traits.is_empty() {
            self.parent_requirements.insert(
                29,
                format!("parts implement {}", rule.required_part_traits.join(", ")),
            );
        }
    }
}
