//! Conversion from authored fields to a registration snapshot.
//! 将创作字段转换成注册快照。

use super::*;

impl FaceManifest {
    pub(crate) fn to_snapshot(&self) -> Result<RegistrationSnapshot, String> {
        let value = |key| self.values.get(key).map(String::as_str).unwrap_or("");
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
            authoring_namespace()
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
            handle,
            stable_name: (!value("stable_name").is_empty())
                .then_some(value("stable_name").to_owned()),
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
            requires: parse_requirements_owned(value("requires")),
            provides: split_csv_owned(value("provides")),
            contract: OwnedObjectContract {
                required_parts: split_csv_owned(value("required_parts")),
                provided_parts: split_csv_owned(value("provided_parts")),
                expected_output: value("expected_output").to_owned(),
                actual_output: value("actual_output").to_owned(),
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
                function: value("handle").to_owned(),
            },
        })
    }
}
