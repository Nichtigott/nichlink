// Registration-face field presentation metadata.
// 注册面字段的展示元数据。
//
// The 30-field table describes the registration-face protocol itself, shared
// by the Studio form and the file authoring API. Values stay in the caller;
// only static labels, roles, defaults, and help text live here.
// 30 字段表描述注册面协议本身，Studio 表单与文件创作 API 共用。取值留在
// 调用方，这里只有静态标签、角色、默认值与帮助文本。

/// Field role shown next to each label in the field guide.
/// 字段指南中标签旁展示的角色。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FaceFieldRole {
    Required,
    Derived,
    ReadOnly,
    Optional,
    Conditional,
}

impl FaceFieldRole {
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Required => "*",
            Self::Derived => "◇",
            Self::ReadOnly => "↳",
            Self::Optional => "·",
            Self::Conditional => "?",
        }
    }

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
    pub group: &'static str,
    pub label: &'static str,
    pub role: FaceFieldRole,
    pub default: &'static str,
    pub help: &'static str,
}

/// Presentation metadata for the 30 registration-face fields, by index.
/// 按索引返回 30 个注册面字段的展示元数据。
pub fn face_field_presentation(index: usize) -> FaceFieldPresentation {
    let field = |group, label, role, default, help| FaceFieldPresentation {
        group,
        label,
        role,
        default,
        help,
    };
    match index {
        0 => field(
            "IDENTITY",
            "parent registry",
            FaceFieldRole::Derived,
            "selected tree node",
            "Registry that receives this face. A readable path or node identity is accepted.",
        ),
        1 => field(
            "IDENTITY",
            "module name",
            FaceFieldRole::Required,
            "none",
            "Rust module directory and file name. Use snake_case.",
        ),
        2 => field(
            "REGISTRY",
            "owns child registry",
            FaceFieldRole::Optional,
            "false",
            "Create the same Registry type below this face so children can register recursively.",
        ),
        3 => field(
            "REGISTRY",
            "tree slot",
            FaceFieldRole::Derived,
            "module name",
            "Name shown in the registration tree. Override only when it must differ from the source module.",
        ),
        4 => field(
            "REGISTRY",
            "child structure rule",
            FaceFieldRole::Conditional,
            "ANY",
            "Minimum structure required from children. Used only when this face owns a Registry.",
        ),
        5 => field(
            "REGISTRY",
            "allowed dependencies",
            FaceFieldRole::Conditional,
            "ANY",
            "External registration branches that descendants may consume. This is not a construction rule.",
        ),
        6 => field(
            "IMPLEMENTATION",
            "parts type",
            FaceFieldRole::Derived,
            "NoParts",
            "Concrete state/parts type supplied by this face.",
        ),
        7 => field(
            "CAPABILITIES",
            "exports",
            FaceFieldRole::Optional,
            "none",
            "Interfaces exposed at the registration seam. A parent structure rule may require them.",
        ),
        8 => field(
            "IDENTITY",
            "Rust type",
            FaceFieldRole::Derived,
            "PascalCase(module)",
            "Kind used by Rust and registration identity. It is derived from module name unless overridden.",
        ),
        9 => field(
            "IDENTITY",
            "display name zh",
            FaceFieldRole::Derived,
            "Rust type",
            "Chinese display name. It defaults to the Rust type.",
        ),
        10 => field(
            "IDENTITY",
            "display name en",
            FaceFieldRole::Derived,
            "Rust type",
            "English display name. It defaults to the Rust type.",
        ),
        11 => field(
            "IDENTITY",
            "summary zh",
            FaceFieldRole::Optional,
            "none",
            "Short Chinese description shown to people and AI tools.",
        ),
        12 => field(
            "IDENTITY",
            "summary en",
            FaceFieldRole::Optional,
            "none",
            "Short English description shown to people and AI tools.",
        ),
        13 => field(
            "IMPLEMENTATION",
            "preset type",
            FaceFieldRole::Derived,
            "NoPreset",
            "Preset contract that states which parts the implementation expects.",
        ),
        14 => field(
            "IMPLEMENTATION",
            "parameter metadata",
            FaceFieldRole::Derived,
            "Rust type",
            "Searchable parameter name or schema. This is metadata, not a Rust type assertion.",
        ),
        15 => field(
            "IMPLEMENTATION",
            "handle type",
            FaceFieldRole::Derived,
            "Rust type",
            "Rust type that implements the face contract.",
        ),
        16 => field(
            "IDENTITY",
            "stable identity",
            FaceFieldRole::Optional,
            "source identity",
            "Explicit identity preserved across source moves. Leave empty unless external plans must survive a move.",
        ),
        17 => field(
            "REGISTRY",
            "external source note",
            FaceFieldRole::Optional,
            "none",
            "Provenance metadata only. Admission grants access; requires selects a dependency provider.",
        ),
        18 => field(
            "REGISTRY",
            "rule source",
            FaceFieldRole::Derived,
            "registry_rule/registry_rule.rs",
            "Canonical rule file beside a face that owns a Registry.",
        ),
        19 => field(
            "CONTRACTS",
            "handle trait labels",
            FaceFieldRole::Derived,
            "trait path names",
            "Searchable trait labels. Studio derives them when compiler-checked paths are provided.",
        ),
        20 => field(
            "CONTRACTS",
            "handle trait paths",
            FaceFieldRole::Optional,
            "none",
            "Rust trait paths implemented by the handle and checked by the compiler.",
        ),
        21 => field(
            "CONTRACTS",
            "parts trait labels",
            FaceFieldRole::Derived,
            "trait path names",
            "Searchable parts-trait labels. Studio derives them from compiler-checked paths.",
        ),
        22 => field(
            "CAPABILITIES",
            "requires",
            FaceFieldRole::Optional,
            "none",
            "Capabilities consumed from named providers, written as capability=>provider.",
        ),
        23 => field(
            "CAPABILITIES",
            "provides",
            FaceFieldRole::Optional,
            "none",
            "Capabilities advertised to dependency resolution. Unlike exports, these satisfy requires edges.",
        ),
        24 => field(
            "DATA FLOW",
            "expected object output",
            FaceFieldRole::Derived,
            "()",
            "Object-construction output expected at registration. Grafts use the flow contract below.",
        ),
        25 => field(
            "DATA FLOW",
            "actual object output",
            FaceFieldRole::Derived,
            "()",
            "Object-construction output declared by this implementation. It must match the expected output.",
        ),
        26 => field(
            "DEBUG",
            "runtime checks",
            FaceFieldRole::Optional,
            "none",
            "Value checks retained according to the selected trace mode.",
        ),
        27 => field(
            "DATA FLOW",
            "graft flow contract",
            FaceFieldRole::Optional,
            "none",
            "Versioned input/output seam used to validate replacement grafts.",
        ),
        28 => field(
            "DATA FLOW",
            "flow provider type",
            FaceFieldRole::Conditional,
            "none",
            "Rust provider type for the declared flow contract.",
        ),
        29 => field(
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
    let module = values.get(1).map_or("", String::as_str).trim();
    let rust_type = values.get(8).map_or("", String::as_str).trim();
    let kind = if rust_type.is_empty() {
        pascal_case(module)
    } else {
        rust_type.to_owned()
    };
    match index {
        1 => "<required>".to_owned(),
        3 => auto_value(module),
        8 => auto_value(&kind),
        9 | 10 | 14 | 15 => auto_value(&kind),
        16 => "<source identity>".to_owned(),
        18 => "<canonical beside face>".to_owned(),
        19 => derived_trait_names(values.get(20).map_or("", String::as_str)),
        21 => derived_trait_names(values.get(29).map_or("", String::as_str)),
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
