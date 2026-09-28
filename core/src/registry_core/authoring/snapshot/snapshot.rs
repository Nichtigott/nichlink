//! Conversion from authored field values to a registration snapshot.
//! 将创作字段值转换成注册快照。

use std::collections::BTreeMap;

use crate::registry_core::declaration::{
    OwnedFlowContract, OwnedLocalizedText, OwnedObjectContract, OwnedSourceLocation,
    RegistrationSnapshot, RuntimeCheckSpec,
};
use crate::registry_core::identity::NodeId;

use super::parse::{
    parse_admission_owned, parse_flow_value, parse_registration_rule_owned, split_csv_owned,
    try_parse_requirements_owned,
};
use super::validation::rule_path_for_source;

/// Build a registration snapshot from authored face field values.
/// 根据创作注册面的字段值构建注册快照。
///
/// `default_namespace` supplies the namespace when the face left it blank;
/// callers bound to process state pass their resolved namespace here.
/// `default_namespace` 在注册面未填写命名空间时提供回落值；
/// 绑定进程状态的调用方在此传入自己解析出的命名空间。
pub fn snapshot_from_values(
    values: &BTreeMap<String, String>,
    default_namespace: &str,
) -> Result<RegistrationSnapshot, String> {
    let value = |key| values.get(key).map(String::as_str).unwrap_or("");
    let source = value("source").to_owned();
    let kind = value("kind").to_owned();
    let declaration_line = value("declaration_line").parse().unwrap_or(1);
    let parent = value("parent_node")
        .parse::<NodeId>()
        .map_err(|_| "generated face has an invalid parent node identity".to_owned())?;
    let needs_registry = match value("needs_registry") {
        "true" => true,
        "false" => false,
        _ => return Err("generated face has an invalid needs_registry value".to_owned()),
    };
    let registry_name = if value("registry_name").is_empty() {
        value("module")
    } else {
        value("registry_name")
    };
    let preset = if value("preset").is_empty() {
        "NoPreset"
    } else {
        value("preset")
    };
    let parts = if value("parts").is_empty() {
        "NoParts"
    } else {
        value("parts")
    };
    let handle = if value("handle").is_empty() {
        kind.clone()
    } else {
        value("handle").to_owned()
    };
    let name_zh = if value("name_zh").is_empty() {
        kind.clone()
    } else {
        value("name_zh").to_owned()
    };
    let name_en = if value("name_en").is_empty() {
        kind.clone()
    } else {
        value("name_en").to_owned()
    };
    let registry_rule_path = if value("registry_rule_path").is_empty() {
        rule_path_for_source(&source)
    } else {
        value("registry_rule_path").to_owned()
    };
    let exports = split_csv_owned(value("exports"));
    let handle_traits = split_csv_owned(value("handle_traits"));
    let part_traits = split_csv_owned(value("part_traits"));
    let registration_rule = parse_registration_rule_owned(value("registration_rule"))?;
    let admission = parse_admission_owned(value("admission"))?;
    let namespace = if value("namespace").is_empty() {
        default_namespace.to_owned()
    } else {
        value("namespace").to_owned()
    };
    Ok(RegistrationSnapshot {
        namespace: namespace.clone(),
        id: NodeId::from_namespaced_path(&namespace, &source, &kind),
        parent,
        kind: kind.clone(),
        preset: preset.to_owned(),
        parts: parts.to_owned(),
        params: if value("params").is_empty() {
            kind.clone()
        } else {
            value("params").to_owned()
        },
        // The derived `handle`, not the raw key: the same value answers this field
        // and `source.function` below, and a face that does not store `handle`
        // (the file form never does — nothing renders it) must still carry the
        // function name its compiled form carries (`function: stringify!($kind)`).
        // Reading the raw key in one place and the derived value in the other is
        // how every authored face ended up with `function=""` (audit `N-4`).
        // 这里用的是**派生的** `handle`，而不是原始键：同一个值同时回答本字段与下面的
        // `source.function`，而一个不存 `handle` 的注册面（文件形式从不存——没有任何东西渲染它）
        // 仍必须带着它编译形式所带的函数名（`function: stringify!($kind)`）。一处读原始键、
        // 另一处读派生值，正是所有创作面 `function=""` 的来路（审计 `N-4`）。
        handle: handle.clone(),
        stable_name: (!value("stable_name").is_empty()).then_some(value("stable_name").to_owned()),
        name: OwnedLocalizedText {
            zh: name_zh,
            en: name_en,
        },
        summary: OwnedLocalizedText {
            zh: value("summary_zh").to_owned(),
            en: value("summary_en").to_owned(),
        },
        exports,
        needs_registry,
        registry_name: registry_name.to_owned(),
        getting_from_other_registry: (!value("getting_from_other_registry").is_empty())
            .then_some(value("getting_from_other_registry").to_owned()),
        registry_rule_path: registry_rule_path.to_owned(),
        registry_rule: registration_rule,
        admission,
        requires: try_parse_requirements_owned(value("requires"))
            .map_err(|error| error.to_string())?,
        provides: split_csv_owned(value("provides")),
        contract: OwnedObjectContract {
            required_parts: split_csv_owned(value("required_parts")),
            provided_parts: split_csv_owned(value("provided_parts")),
        },
        flow: parse_flow_value(value("flow"))?.unwrap_or_else(OwnedFlowContract::none),
        flow_provider: (!value("flow_provider").is_empty())
            .then_some(value("flow_provider").to_owned()),
        handle_traits,
        part_traits,
        runtime_checks: RuntimeCheckSpec::parse_list(value("runtime_checks"))?,
        plugin: None,
        source: OwnedSourceLocation {
            file: source,
            line: declaration_line,
            column: 1,
            // The derived `handle`, so this field and the `handle` field above are
            // one value, not two derivations (audit `N-4`).
            // 派生的 `handle`，因此本字段与上面的 `handle` 字段是同一个值，而不是两份推导
            // （审计 `N-4`）。
            function: handle,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One derivation: both spellings of the input answer `source.function` with
    /// the same value the `handle` field carries.
    /// 只有一份推导：两种输入拼法都让 `source.function` 与 `handle` 字段同值。
    ///
    /// Red before `N-4`: with `handle` absent, `source.function` was `""` while the
    /// `handle` field said `kind` — the same value derived twice, differently.
    /// `N-4` 之前为红：`handle` 不存时 `source.function` 是 `""`，而 `handle` 字段说的是
    /// `kind`——同一个值被推导了两次、结果还不同。
    #[test]
    fn the_handle_field_and_the_source_function_are_one_value() {
        let cases = [
            ("an omitted handle", None, "Widget"),
            ("a declared handle", Some("CustomHandle"), "CustomHandle"),
        ];
        for (label, declared, expected) in cases {
            let mut values = BTreeMap::new();
            for (key, value) in [
                ("source", "widget/widget.rs"),
                ("kind", "Widget"),
                ("namespace", "probe"),
                ("needs_registry", "false"),
                ("registration_rule", "ANY"),
                ("admission", "ANY"),
            ] {
                values.insert(key.to_owned(), value.to_owned());
            }
            values.insert(
                "parent_node".to_owned(),
                NodeId::from_namespaced_path("probe", "root.rs", "root").to_string(),
            );
            if let Some(declared) = declared {
                values.insert("handle".to_owned(), declared.to_owned());
            }
            let snapshot =
                snapshot_from_values(&values, "probe").expect("a face with required keys converts");
            assert_eq!(snapshot.handle, expected, "{label}: the handle field");
            assert_eq!(
                snapshot.source.function, expected,
                "{label}: `source.function` is the derived handle, not a second reading of the key"
            );
        }
    }

    /// A face's field values, with the identity keys the snapshot needs.
    /// 一个注册面的字段取值，含快照需要的身份键。
    fn face_values(source: &str, kind: &str) -> BTreeMap<String, String> {
        let mut values = BTreeMap::new();
        for (key, value) in [
            ("source", source),
            ("kind", kind),
            ("namespace", "probe"),
            ("needs_registry", "false"),
            ("registration_rule", "ANY"),
            ("admission", "ANY"),
        ] {
            values.insert(key.to_owned(), value.to_owned());
        }
        values.insert(
            "parent_node".to_owned(),
            NodeId::from_namespaced_path("probe", "root.rs", "root").to_string(),
        );
        values
    }

    /// The rule-location field names a position, not the existence of a file.
    /// 规则位置字段名的是一个位置，而不是某份文件的存在。
    ///
    /// Audit `t75` §6.1 read the field's documentation as "the path that *produced* the rule"
    /// while this derivation fills the canonical location beside the face whether or not a file is
    /// there. The documentation now says "would be read from"; this pin holds the value to it, and
    /// the derivation touches no filesystem at all, so no state of the disk can change it — the
    /// same `source` yields the same location (audit `t75` §6.1).
    /// 审计 `t75` §6.1 读到字段文档说的是"**产出**该规则的路径"，而本派生填的是注册面旁那个规范
    /// 位置，无论那里有没有文件。文档现在写的是"**会被读取**的位置"；本钉子把取值钉在文档上，而该
    /// 派生完全不触碰文件系统，因此磁盘状态无法改变它——同一个 `source` 得到同一个位置
    /// （审计 `t75` §6.1）。
    #[test]
    fn the_rule_path_field_names_the_location_the_rule_would_be_read_from() {
        let values = face_values("control/object/slider/slider.rs", "Slider");
        let snapshot = snapshot_from_values(&values, "probe").expect("a readable face");
        assert_eq!(
            snapshot.registry_rule_path,
            rule_path_for_source("control/object/slider/slider.rs")
        );
        assert_eq!(
            snapshot.registry_rule_path,
            "src/control/object/slider/registry_rule/registry_rule.rs"
        );
        // The same input answers the same location again: the value is a function of the face's
        // `source` and of nothing else, so no state of the disk can reach it.
        // 同一输入再次给出同一位置：取值只是注册面 `source` 的函数，磁盘状态无从影响它。
        let again = snapshot_from_values(&values, "probe").expect("a readable face");
        assert_eq!(again.registry_rule_path, snapshot.registry_rule_path);
        // A declared location wins over the derived one.
        // 显式声明的位置优先于派生位置。
        let mut declared = values.clone();
        declared.insert(
            "registry_rule_path".to_owned(),
            "src/elsewhere/registry_rule/registry_rule.rs".to_owned(),
        );
        let declared = snapshot_from_values(&declared, "probe").expect("a readable face");
        assert_eq!(
            declared.registry_rule_path,
            "src/elsewhere/registry_rule/registry_rule.rs"
        );
    }
}
