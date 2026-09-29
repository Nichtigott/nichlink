//! Canonical bytes for the registration half of a plugin signing payload.
//! 插件签名载荷中注册声明一半的规范化字节。
//!
//! A signature over a manifest and its bytes says nothing about the
//! `RegistrationInfo` that travels beside them, so a tampered `parent` or
//! `flow` still verified as `Signature` — and could even pass a slot contract
//! check the honest artifact failed. Every field a registration claims is
//! written here in a fixed order with a length prefix, and
//! [`PluginManifest::signing_payload`](super::PluginManifest::signing_payload)
//! appends the result, so one signature covers both halves.
//! 只覆盖 manifest 与插件字节的签名，对同行携带的 `RegistrationInfo` 什么都没说：被篡改的
//! `parent` 或 `flow` 仍会验证为 `Signature`，甚至能通过诚实工件通不过的槽位合同检查。这里按
//! 固定顺序、带长度前缀写下注册声明声称的每个字段，由
//! [`PluginManifest::signing_payload`](super::PluginManifest::signing_payload) 追加，两半因此被
//! 同一个签名覆盖。
//!
//! Adding a field to `RegistrationInfo` means adding it here too: a field the
//! encoder does not name is a field the signature does not protect.
//! 给 `RegistrationInfo` 增加字段就意味着在这里也增加：编码器不写出的字段，就是签名不保护的字段。

use crate::registry_core::declaration::{
    Admission, ObjectContract, PluginManifest, PluginMode, PluginSource, RegistrationInfo,
    RegistrationRule, SourceLocation,
};

use super::FlowContract;

/// Append one length-prefixed field.
/// 追加一个带长度前缀的字段。
pub(super) fn field(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&(value.len() as u64).to_le_bytes());
    out.extend_from_slice(value.as_bytes());
}

/// Append an optional field, keeping `None` distinct from an empty string.
/// 追加可选字段，并让 `None` 与空字符串保持可区分。
fn optional(out: &mut Vec<u8>, value: Option<&str>) {
    match value {
        Some(value) => {
            out.push(1);
            field(out, value);
        }
        None => out.push(0),
    }
}

/// Append a length-prefixed list of strings, so two short neighbours cannot be
/// read as one long field.
/// 追加带长度前缀的字符串列表，两个相邻的短字段不会被读成一个长字段。
fn strings(out: &mut Vec<u8>, values: &[&str]) {
    out.extend_from_slice(&(values.len() as u64).to_le_bytes());
    for value in values {
        field(out, value);
    }
}

/// Append every field one manifest claims, except the signature itself: a
/// signature cannot cover the bytes that carry it.
/// 追加 manifest 声称的每个字段，签名自身除外：签名无法覆盖承载它的那段字节。
pub(super) fn manifest(out: &mut Vec<u8>, manifest: PluginManifest) {
    field(out, manifest.name);
    field(out, manifest.crate_name);
    field(out, manifest.version);
    field(out, manifest.framework.0);
    field(
        out,
        match manifest.source {
            PluginSource::Official => "official",
            PluginSource::User => "user",
        },
    );
    field(
        out,
        match manifest.mode {
            PluginMode::Extension => "extension",
            PluginMode::Replacement => "replacement",
        },
    );
    field(out, manifest.checksum);
    optional(out, manifest.public_key_fingerprint);
    optional(out, manifest.revocation_list);
}

/// Append the structural rule a face's registry imposes on its own members.
/// 追加注册面对自身成员施加的结构规范。
fn rule(out: &mut Vec<u8>, rule: RegistrationRule) {
    optional(out, rule.required_preset);
    strings(out, rule.required_parts);
    strings(out, rule.required_exports);
    strings(out, rule.required_handle_traits);
    strings(out, rule.required_part_traits);
}

/// Append the external-dependency gate.
/// 追加外部依赖门禁。
fn admission(out: &mut Vec<u8>, admission: Admission) {
    strings(out, admission.allowed_paths);
    strings(out, admission.denied_paths);
}

/// Append the construction contract's two part lists.
/// 追加构造合同的两份 part 列表。
fn object_contract(out: &mut Vec<u8>, contract: ObjectContract) {
    strings(out, contract.required_parts);
    strings(out, contract.provided_parts);
}

/// Append the automatically comparable input/output contract.
/// 追加可自动比较的输入/输出合同。
fn flow(out: &mut Vec<u8>, flow: FlowContract) {
    field(out, flow.id.0);
    out.extend_from_slice(&flow.version.to_le_bytes());
    field(out, flow.input);
    field(out, flow.output);
}

/// Append the declaration site, which is what a reader uses to find the origin.
/// 追加声明位置，读者据此找到来源。
fn source(out: &mut Vec<u8>, source: SourceLocation) {
    field(out, source.file);
    out.extend_from_slice(&source.line.to_le_bytes());
    out.extend_from_slice(&source.column.to_le_bytes());
    field(out, source.function);
}

/// Append everything a plugin registration claims.
/// 追加插件注册声明声称的一切。
pub(super) fn registration(info: &RegistrationInfo, out: &mut Vec<u8>) {
    field(out, info.namespace);
    out.extend_from_slice(&info.id.into_bytes());
    out.extend_from_slice(&info.parent.into_bytes());
    field(out, info.kind);
    field(out, info.preset);
    field(out, info.parts);
    field(out, info.params);
    field(out, info.handle);
    optional(out, info.stable_name);
    field(out, info.name.zh);
    field(out, info.name.en);
    field(out, info.summary.zh);
    field(out, info.summary.en);
    strings(out, info.exports);
    out.push(u8::from(info.needs_registry));
    field(out, info.registry_name);
    optional(out, info.getting_from_other_registry);
    field(out, info.registry_rule_path);
    rule(out, info.registry_rule);
    admission(out, info.admission);
    out.extend_from_slice(&(info.requires.len() as u64).to_le_bytes());
    for requirement in info.requires {
        field(out, requirement.capability);
        field(out, requirement.provider);
    }
    strings(out, info.provides);
    object_contract(out, info.contract);
    flow(out, info.flow);
    optional(out, info.flow_provider);
    strings(out, info.handle_traits);
    strings(out, info.part_traits);
    out.extend_from_slice(&(info.runtime_checks.len() as u64).to_le_bytes());
    for check in info.runtime_checks {
        field(out, &check.expression());
    }
    match info.plugin {
        Some(manifest_of_face) => {
            out.push(1);
            manifest(out, manifest_of_face);
        }
        None => out.push(0),
    }
    source(out, info.source);
}

#[cfg(test)]
mod tests {
    use crate::registry_core::declaration::{
        Admission, ContractId, LocalizedText, ObjectContract, PluginManifest, PluginMode,
        PluginSource, RegistrationInfo, RegistrationRule, SourceLocation,
    };
    use crate::{FlowContract, FrameworkId, NodeId};

    /// One registration with every field set, so an encoder that forgets a
    /// field is compared against something that carries one.
    /// 一份每个字段都有取值的注册声明，编码器漏掉字段时才有东西可比。
    fn full_registration() -> RegistrationInfo {
        RegistrationInfo {
            namespace: "signing-test",
            id: NodeId::from_path("signing.rs", "Signing"),
            parent: NodeId::from_path("parent.rs", "Parent"),
            kind: "Signing",
            preset: "SigningPreset",
            parts: "SigningParts",
            params: "SigningParams",
            handle: "SigningHandle",
            stable_name: Some("signing"),
            name: LocalizedText {
                zh: "签名",
                en: "Signing",
            },
            summary: LocalizedText {
                zh: "摘要",
                en: "Summary",
            },
            exports: &["one"],
            needs_registry: true,
            registry_name: "signing-registry",
            getting_from_other_registry: Some("other-registry"),
            registry_rule_path: "rule.rs",
            registry_rule: RegistrationRule::ANY.require_preset("SigningPreset"),
            admission: Admission::allow_paths(&["crate::allowed"]),
            requires: &[],
            provides: &["capability"],
            contract: ObjectContract {
                required_parts: &["head"],
                provided_parts: &["head", "tail"],
            },
            flow: FlowContract::new(ContractId::new("signing.v1"), 3, "Input", "Output"),
            flow_provider: Some("FlowProvider"),
            handle_traits: &["SigningHandleTrait"],
            part_traits: &["SigningPartsTrait"],
            runtime_checks: &[crate::NON_EMPTY_TEXT],
            plugin: Some(PluginManifest {
                name: "official.signing",
                crate_name: "official_signing",
                version: "1.0.0",
                framework: FrameworkId::new("nichlink.test"),
                source: PluginSource::Official,
                mode: PluginMode::Extension,
                checksum: "sha256:aa",
                signature: Some("ignored-by-the-payload"),
                public_key_fingerprint: Some("ff"),
                revocation_list: Some("official-1"),
            }),
            source: SourceLocation {
                file: "signing.rs",
                line: 7,
                column: 11,
                function: "full_registration",
            },
        }
    }

    fn encoded(info: &RegistrationInfo) -> Vec<u8> {
        let mut out = Vec::new();
        super::registration(info, &mut out);
        out
    }

    /// The payload is a function of the registration: encrypting a different
    /// value into any field changes it.
    /// 载荷是注册声明的函数：任何字段换成别的取值都会改变它。
    #[test]
    fn every_compared_field_changes_the_registration_bytes() {
        let honest = encoded(&full_registration());
        assert!(!honest.is_empty());

        // A named alias keeps the closure pair readable and the lint quiet.
        // 具名别名让闭包对更易读，也让 lint 闭嘴。
        type Mutation = (&'static str, fn(&mut RegistrationInfo));
        let mutations: [Mutation; 5] = [
            ("parent", |info| {
                info.parent = NodeId::from_path("other.rs", "Other");
            }),
            ("flow", |info| {
                info.flow = FlowContract::new(ContractId::new("hijacked.v1"), 1, "A", "B");
            }),
            ("kind", |info| info.kind = "Other"),
            ("requires", |info| {
                info.requires = &[crate::registry_core::declaration::RequirementSpec {
                    capability: "cap",
                    provider: "obj",
                }];
            }),
            ("plugin", |info| info.plugin = None),
        ];
        for (name, mutate) in mutations {
            let mut info = full_registration();
            mutate(&mut info);
            assert_ne!(
                encoded(&info),
                honest,
                "a change to `{name}` must change the signed registration bytes"
            );
        }
    }

    /// The signature field of the embedded manifest is not covered: a signature
    /// cannot be part of the bytes it signs.
    /// 内嵌 manifest 的签名字段不被覆盖：签名不可能属于它所签的那段字节。
    #[test]
    fn the_signature_itself_is_not_part_of_the_payload() {
        let honest = encoded(&full_registration());
        let mut resigned = full_registration();
        let mut manifest = resigned.plugin.expect("manifest");
        manifest.signature = Some("another-signature");
        resigned.plugin = Some(manifest);
        assert_eq!(encoded(&resigned), honest);
    }
}
