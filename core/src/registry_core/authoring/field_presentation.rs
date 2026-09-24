//! Registration-face field presentation metadata.
// 注册面字段的展示元数据。
//
// One row per slot of the authoring layout (`super::face_field`), shared by the
// Studio form and the file authoring API. Values stay in the caller; only static
// labels, roles, defaults, and help text live here. The help text states what
// the field does, not what it sounds like it should do: a row whose sentence
// contradicts the code is worse than no row.
// 创作布局（`super::face_field`）每个槽位一行，Studio 表单与文件创作 API 共用。取值留在
// 调用方，这里只有静态标签、角色、默认值与帮助文本。帮助文本陈述字段实际做什么，而不是
// 听起来像做什么：一句与代码相反的说明比没有说明更糟。

use super::face_field::*;

/// Field role shown next to each label in the field guide.
/// 字段指南中标签旁展示的角色。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FaceFieldRole {
    /// The author must supply a value.
    /// 作者必须提供取值。
    Required,
    /// The value is computed from sibling fields.
    /// 取值由同排其他字段推导。
    Derived,
    /// The value is derived and cannot be edited directly.
    /// 取值由推导得到，不能直接编辑。
    ReadOnly,
    /// The field may be left empty.
    /// 该字段可以留空。
    Optional,
    /// The field applies only in the situation its help text names.
    /// 该字段仅在帮助文本所述情形下适用。
    Conditional,
}

impl FaceFieldRole {
    /// The short glyph the field guide prints beside the label.
    /// 字段指南在标签旁打印的短标记。
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Required => "*",
            Self::Derived => "◇",
            Self::ReadOnly => "↳",
            Self::Optional => "·",
            Self::Conditional => "?",
        }
    }

    /// The upper-case role name the field guide prints.
    /// 字段指南打印的大写角色名。
    pub const fn label(self) -> &'static str {
        match self {
            Self::Required => "REQUIRED",
            Self::Derived => "DERIVED",
            Self::ReadOnly => "DERIVED / READ ONLY",
            Self::Optional => "OPTIONAL",
            Self::Conditional => "WHEN USED",
        }
    }
}

/// Static presentation metadata for one registration-face field.
/// 一个注册面字段的静态展示元数据。
#[derive(Clone, Copy, Debug)]
pub struct FaceFieldPresentation {
    /// The section heading the field belongs to.
    /// 该字段所属的章节标题。
    pub group: &'static str,
    /// The human-readable field label.
    /// 人类可读的字段标签。
    pub label: &'static str,
    /// How the field is supplied, shown in the field guide.
    /// 字段的提供方式，在字段指南中展示。
    pub role: FaceFieldRole,
    /// The static fallback shown when no value is derived.
    /// 未能推导取值时展示的静态回退值。
    pub default: &'static str,
    /// One-line guidance shown for the field.
    /// 该字段展示的单行说明。
    pub help: &'static str,
}

/// Presentation metadata for the authoring layout, by slot index.
/// 按槽位下标返回创作布局的展示元数据。
pub fn face_field_presentation(index: usize) -> FaceFieldPresentation {
    let field = |group, label, role, default, help| FaceFieldPresentation {
        group,
        label,
        role,
        default,
        help,
    };
    match index {
        PARENT => field(
            "IDENTITY",
            "parent registry",
            FaceFieldRole::Required,
            "selected tree node",
            "Registry that receives this face: the selected tree node, or a path/node identity typed here.",
        ),
        MODULE => field(
            "IDENTITY",
            "module name",
            FaceFieldRole::Required,
            "none",
            "Rust module directory and file name. Use snake_case.",
        ),
        NEEDS_REGISTRY => field(
            "REGISTRY",
            "owns child registry",
            FaceFieldRole::Optional,
            "false",
            "Create the same Registry type below this face so children can register recursively.",
        ),
        TREE_SLOT => field(
            "REGISTRY",
            "tree slot",
            FaceFieldRole::ReadOnly,
            "module name",
            "Name this face shows in the registration tree. It follows the declaring module, so it is shown rather than authored.",
        ),
        REGISTRY_RULE => field(
            "REGISTRY",
            "child structure rule",
            FaceFieldRole::Conditional,
            "ANY",
            "Minimum structure required from children. Used only when this face owns a Registry.",
        ),
        ADMISSION => field(
            "REGISTRY",
            "allowed dependencies",
            FaceFieldRole::Conditional,
            "ANY",
            "External registration branches that descendants may consume. This is not a construction rule.",
        ),
        PARTS => field(
            "IMPLEMENTATION",
            "parts type",
            FaceFieldRole::Optional,
            "NoParts",
            "Concrete state/parts type supplied by this face.",
        ),
        EXPORTS => field(
            "CAPABILITIES",
            "exports",
            FaceFieldRole::Optional,
            "none",
            "Interfaces exposed at the registration seam. A parent structure rule may require them.",
        ),
        KIND => field(
            "IDENTITY",
            "Rust type",
            FaceFieldRole::Optional,
            "PascalCase(module)",
            "Kind used by Rust and registration identity. It is derived from module name unless overridden.",
        ),
        NAME_ZH => field(
            "IDENTITY",
            "display name zh",
            FaceFieldRole::Optional,
            "Rust type",
            "Chinese display name. It defaults to the Rust type.",
        ),
        NAME_EN => field(
            "IDENTITY",
            "display name en",
            FaceFieldRole::Optional,
            "Rust type",
            "English display name. It defaults to the Rust type.",
        ),
        SUMMARY_ZH => field(
            "IDENTITY",
            "summary zh",
            FaceFieldRole::Optional,
            "none",
            "Short Chinese description shown to people and AI tools.",
        ),
        SUMMARY_EN => field(
            "IDENTITY",
            "summary en",
            FaceFieldRole::Optional,
            "none",
            "Short English description shown to people and AI tools.",
        ),
        PRESET => field(
            "IMPLEMENTATION",
            "preset type",
            FaceFieldRole::Optional,
            "NoPreset",
            "Preset contract that states which parts the implementation expects.",
        ),
        STABLE_NAME => field(
            "IDENTITY",
            "stable identity",
            FaceFieldRole::Optional,
            "source identity",
            "Explicit identity preserved across source moves. Leave empty unless external plans must survive a move.",
        ),
        GETTING_FROM_OTHER_REGISTRY => field(
            "REGISTRY",
            "dependency registry",
            FaceFieldRole::Optional,
            "none",
            "Registry whose providers this face's requires edges may draw on. It names a resolution source; admission is what grants the access.",
        ),
        REGISTRY_RULE_PATH => field(
            "REGISTRY",
            "rule source",
            FaceFieldRole::ReadOnly,
            "registry_rule/registry_rule.rs",
            "Canonical rule file beside a face that owns a Registry. It follows the face's own location.",
        ),
        HANDLE_TRAITS => field(
            "CONTRACTS",
            "handle trait labels",
            FaceFieldRole::ReadOnly,
            "trait path names",
            "Searchable trait labels derived from the handle contract paths. They are not typed separately.",
        ),
        HANDLE_CONTRACTS => field(
            "CONTRACTS",
            "handle trait paths",
            FaceFieldRole::Optional,
            "none",
            "Rust trait paths implemented by the handle and checked by the compiler.",
        ),
        PART_TRAITS => field(
            "CONTRACTS",
            "parts trait labels",
            FaceFieldRole::ReadOnly,
            "trait path names",
            "Searchable parts-trait labels derived from the parts contract paths. They are not typed separately.",
        ),
        REQUIRES => field(
            "CAPABILITIES",
            "requires",
            FaceFieldRole::Optional,
            "none",
            "Capabilities consumed from named providers, written as capability=>provider.",
        ),
        PROVIDES => field(
            "CAPABILITIES",
            "provides",
            FaceFieldRole::Optional,
            "none",
            "Capabilities advertised to dependency resolution. Unlike exports, these satisfy requires edges.",
        ),
        RUNTIME_CHECKS => field(
            "DEBUG",
            "runtime checks",
            FaceFieldRole::Optional,
            "none",
            "Value checks retained according to the selected trace mode.",
        ),
        FLOW => field(
            "DATA FLOW",
            "graft flow contract",
            FaceFieldRole::Optional,
            "none",
            "Versioned input/output seam used to validate replacement grafts.",
        ),
        FLOW_PROVIDER => field(
            "DATA FLOW",
            "flow provider type",
            FaceFieldRole::Conditional,
            "none",
            "Rust provider type for the declared flow contract.",
        ),
        PART_CONTRACTS => field(
            "CONTRACTS",
            "parts trait paths",
            FaceFieldRole::Optional,
            "none",
            "Rust trait paths implemented by Parts and checked by the compiler.",
        ),
        _ => field(
            "OTHER",
            "unknown",
            FaceFieldRole::Optional,
            "none",
            "Unknown registration field.",
        ),
    }
}

/// Effective default shown for one field, derived from the sibling values.
/// 按同排其他取值推导某个字段的默认展示值。
pub fn face_field_default(values: &[String], index: usize) -> String {
    let module = values.get(MODULE).map_or("", String::as_str).trim();
    let rust_type = values.get(KIND).map_or("", String::as_str).trim();
    let kind = if rust_type.is_empty() {
        pascal_case(module)
    } else {
        rust_type.to_owned()
    };
    match index {
        MODULE => "<required>".to_owned(),
        TREE_SLOT => auto_value(module),
        KIND | NAME_ZH | NAME_EN => auto_value(&kind),
        STABLE_NAME => "<source identity>".to_owned(),
        REGISTRY_RULE_PATH => "<canonical beside face>".to_owned(),
        HANDLE_TRAITS => {
            derived_trait_names(values.get(HANDLE_CONTRACTS).map_or("", String::as_str))
        }
        PART_TRAITS => derived_trait_names(values.get(PART_CONTRACTS).map_or("", String::as_str)),
        index => {
            let default = face_field_presentation(index).default;
            if default == "none" {
                "—".to_owned()
            } else {
                format!("<{default}>")
            }
        }
    }
}

/// Convert a snake_case module name into its derived PascalCase type name.
/// 将 snake_case 模块名转换为派生的 PascalCase 类型名。
pub fn pascal_case(module: &str) -> String {
    module
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut characters = part.chars();
            characters
                .next()
                .map(|first| first.to_ascii_uppercase().to_string() + characters.as_str())
                .unwrap_or_default()
        })
        .collect()
}

fn auto_value(value: &str) -> String {
    if value.is_empty() {
        "<auto>".to_owned()
    } else {
        format!("<auto: {value}>")
    }
}

fn derived_trait_names(paths: &str) -> String {
    let names = paths
        .split(',')
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .filter_map(|path| path.rsplit("::").next())
        .collect::<Vec<_>>();
    if names.is_empty() {
        "—".to_owned()
    } else {
        format!("<derived: {}>", names.join(", "))
    }
}
