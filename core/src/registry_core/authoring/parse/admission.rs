//! Admission field rendering and parsing for authored faces.
//! 注册面 admission 字段的渲染与解析。
//!
//! Admission is written either as the compact `allow:…`/`deny:…`/`ANY` form or
//! as the `Admission::new(&[…], &[…])` constructor an editor emits. Both
//! spellings describe the same gate, so both are read here.
//! admission 要么写成紧凑的 `allow:…`/`deny:…`/`ANY`，要么写成编辑器发出的
//! `Admission::new(&[…], &[…])` 构造形式。两种写法描述同一道门禁，因此都在此读取。

use crate::registry_core::authoring::validation::rust_string;
use crate::registry_core::declaration::OwnedAdmission;

use super::{FaceParseError, quoted_list_field, split_csv_owned};

/// Read an admission expression, accepting both the compact and constructor
/// spellings.
/// 读取 admission 表达式，同时接受紧凑写法与构造形式。
pub fn parse_admission_expression(expression: &str) -> Result<String, FaceParseError> {
    let expression = expression.trim().trim_end_matches(',');
    if expression.is_empty() || expression.contains("Admission::ANY") {
        return Ok("ANY".to_owned());
    }
    let paths = quoted_list_field(expression, "allow_paths(");
    if !paths.is_empty() {
        return Ok(format!("allow:{}", paths.join(",")));
    }
    let paths = quoted_list_field(expression, "deny_paths(");
    if !paths.is_empty() {
        return Ok(format!("deny:{}", paths.join(",")));
    }
    // The editor writes the constructor form, because a face holds real Rust.
    // Reading only the `allow_paths(`/`deny_paths(` spellings meant the editor
    // could write a declaration it then refused to read back.
    // 编辑器写下的是构造函数形式，因为注册面里放的是真实 Rust。只认
    // `allow_paths(`/`deny_paths(` 会让编辑器写出自己读不回来的声明。
    if let Some((_, rest)) = expression.split_once("Admission::new(") {
        let mut parts = rest.split(']');
        let allow = parts.next().map(quoted_strings).unwrap_or_default();
        let deny = parts.next().map(quoted_strings).unwrap_or_default();
        return match (allow.is_empty(), deny.is_empty()) {
            (true, true) => Ok("ANY".to_owned()),
            (false, _) => Ok(format!("allow:{}", allow.join(","))),
            (true, false) => Ok(format!("deny:{}", deny.join(","))),
        };
    }
    Err("generated face has an invalid admission expression"
        .to_owned()
        .into())
}

/// The quoted strings of one bracketed fragment, e.g. `&["a", "b"`.
/// 一个方括号片段里的字符串，例如 `&["a", "b"`。
fn quoted_strings(fragment: &str) -> Vec<String> {
    let fragment = fragment.rsplit_once('[').map_or(fragment, |(_, rest)| rest);
    fragment
        .split(',')
        .filter_map(|value| {
            let value = value.trim().trim_end_matches(']').trim();
            value
                .strip_prefix('"')
                .and_then(|value| value.strip_suffix('"'))
                .map(str::to_owned)
        })
        .collect()
}

/// Which list a compact admission value carries.
/// 紧凑 admission 值所携带的是哪一张列表。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AdmissionMode {
    Allow,
    Deny,
}

/// One compact admission value split into its mode and paths.
/// 一个紧凑 admission 值切分成的模式与路径列表。
///
/// `parse_admission_owned` and `render_admission` each used to trim, split at
/// the first `:`, check the empty list, and choose between `allow` and `deny`
/// on their own, including three separately written error strings. A message or
/// an empty-list rule changed on one side would let a value the snapshot parser
/// accepted be refused by the renderer. This is the family's only parser; the
/// renderer formats what it returns and the snapshot parser converts it. The
/// mode is read case-insensitively and with surrounding whitespace trimmed, so
/// both sides treat `ALLOW : a` as a path list. `admission_render_and_parse_read_one_parser`
/// pins the two entry points.
/// `parse_admission_owned` 与 `render_admission` 过去各自去空白、在第一个 `:` 处切分、
/// 检查空列表、在 `allow` 与 `deny` 之间选择，连三条错误字符串都是各写一遍。只改一侧
/// 的消息或空列表规则，会让快照解析器接受的值被渲染器拒绝。这是该字段族唯一的解析器；
/// 渲染器格式化它的返回值，快照解析器转换它。模式按大小写不敏感读取并去掉两侧空白，
/// 因此两侧都把 `ALLOW : a` 当作路径列表。
/// `admission_render_and_parse_read_one_parser` 钉住两个入口。
struct CompactAdmission {
    mode: AdmissionMode,
    paths: Vec<String>,
}

/// Split a compact admission value, or `None` for the unconstrained gate.
/// 切分紧凑 admission 值；空值或 `ANY` 表示不受限门禁时返回 `None`。
fn parse_compact_admission(value: &str) -> Result<Option<CompactAdmission>, FaceParseError> {
    let value = value.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("any") {
        return Ok(None);
    }
    let (mode, paths) = value.split_once(':').ok_or_else(|| {
        "admission must be ANY, allow:path/prefix, or deny:path/prefix".to_owned()
    })?;
    let paths = split_csv_owned(paths);
    if paths.is_empty() {
        return Err("admission must list at least one path".to_owned().into());
    }
    let mode = match mode.trim().to_ascii_lowercase().as_str() {
        "allow" => AdmissionMode::Allow,
        "deny" => AdmissionMode::Deny,
        _ => {
            return Err(
                "admission must be ANY, allow:path/prefix, or deny:path/prefix"
                    .to_owned()
                    .into(),
            );
        }
    };
    Ok(Some(CompactAdmission { mode, paths }))
}

/// Parse a compact admission value into its owned snapshot form.
/// 将紧凑 admission 值解析为快照使用的拥有型形式。
pub fn parse_admission_owned(value: &str) -> Result<OwnedAdmission, FaceParseError> {
    Ok(match parse_compact_admission(value)? {
        None => OwnedAdmission {
            allowed_paths: Vec::new(),
            denied_paths: Vec::new(),
        },
        Some(admission) => match admission.mode {
            AdmissionMode::Allow => OwnedAdmission {
                allowed_paths: admission.paths,
                denied_paths: Vec::new(),
            },
            AdmissionMode::Deny => OwnedAdmission {
                allowed_paths: Vec::new(),
                denied_paths: admission.paths,
            },
        },
    })
}

/// Render a compact admission value as the `Admission` constructor expression.
/// 将紧凑 admission 值渲染成 `Admission` 构造表达式。
pub fn render_admission(value: &str) -> Result<String, FaceParseError> {
    let Some(admission) = parse_compact_admission(value)? else {
        return Ok("crate::Admission::ANY".to_owned());
    };
    let rendered = admission
        .paths
        .iter()
        .map(|path| format!("\"{}\"", rust_string(path)))
        .collect::<Vec<_>>()
        .join(", ");
    match admission.mode {
        AdmissionMode::Allow => Ok(format!("crate::Admission::new(&[{rendered}], &[])")),
        AdmissionMode::Deny => Ok(format!("crate::Admission::new(&[], &[{rendered}])")),
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_admission_expression, parse_admission_owned, render_admission};
    use crate::registry_core::declaration::OwnedAdmission;

    /// The editor writes the constructor form and must be able to read it back;
    /// both spellings describe the same admission.
    /// 编辑器写下构造函数形式，也必须能读回来；两种写法描述同一个 admission。
    #[test]
    fn admission_round_trips_through_the_constructor_form() {
        assert_eq!(
            parse_admission_expression("crate::Admission::new(&[\"a\", \"b\"], &[])"),
            Ok("allow:a,b".to_owned())
        );
        assert_eq!(
            parse_admission_expression("crate::Admission::new(&[], &[\"c\"])"),
            Ok("deny:c".to_owned())
        );
        assert_eq!(
            parse_admission_expression("crate::Admission::new(&[], &[])"),
            Ok("ANY".to_owned())
        );
        assert_eq!(
            parse_admission_expression("crate::Admission::allow_paths(&[\"a\"])"),
            Ok("allow:a".to_owned())
        );
    }

    /// The renderer and the snapshot parser now share one parser, so a malformed
    /// value is refused with identical text on both sides and a well-formed one
    /// names the same mode and paths. Before the extraction each split the line,
    /// checked the empty list, and wrote the three messages separately.
    /// 渲染器与快照解析器现在共用同一个解析器，因此畸形值在两侧得到完全相同的拒绝文本，
    /// 合法值命名同样的模式与路径。抽取之前两者各自切分该行、检查空列表，并把三条消息
    /// 各写一遍。
    #[test]
    fn admission_render_and_parse_read_one_parser() {
        for malformed in ["allow", "allow:", "deny:", "veto:a", "ANY:extra"] {
            let rendered = render_admission(malformed).unwrap_err();
            let parsed = parse_admission_owned(malformed).unwrap_err();
            assert_eq!(rendered, parsed, "value `{malformed}`");
        }
        assert_eq!(
            render_admission("allow: ui, controls ").unwrap(),
            "crate::Admission::new(&[\"ui\", \"controls\"], &[])"
        );
        assert_eq!(
            parse_admission_owned("allow: ui, controls ").unwrap(),
            OwnedAdmission {
                allowed_paths: vec!["ui".to_owned(), "controls".to_owned()],
                denied_paths: Vec::new(),
            }
        );
        assert_eq!(
            render_admission("ALLOW : ui").unwrap(),
            "crate::Admission::new(&[\"ui\"], &[])"
        );
        assert_eq!(
            render_admission("deny:c").unwrap(),
            "crate::Admission::new(&[], &[\"c\"])"
        );
        assert_eq!(render_admission("ANY").unwrap(), "crate::Admission::ANY");
        assert_eq!(
            parse_admission_owned("").unwrap(),
            OwnedAdmission {
                allowed_paths: Vec::new(),
                denied_paths: Vec::new(),
            }
        );
    }
}
