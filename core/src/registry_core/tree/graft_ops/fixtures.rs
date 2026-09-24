//! Shared fixtures for the graft-operation tests.
//! 嫁接操作测试共用的 fixture。
//!
//! The overlay tests and the command-dispatch tests build the same kind of
//! minimal registration face, so the builders live here instead of being
//! duplicated per module.
//! 覆盖（overlay）测试与命令派发测试构造的是同一类最小注册面，因此构建器放在这里，
//! 而不是在每个模块里重复一份。

use crate::registry_core::declaration::{
    Admission, FrameworkId, OwnedFlowContract, OwnedLocalizedText, OwnedObjectContract,
    OwnedSourceLocation, RegistrationRule, RegistrationSnapshot,
};
use crate::registry_core::identity::{NodeId, root_node_id};

pub(super) const FRAMEWORK: FrameworkId = FrameworkId::new("graft-test");

pub(super) fn flow(id: &str) -> OwnedFlowContract {
    OwnedFlowContract {
        id: id.to_owned(),
        version: 1,
        input: "LocalCoordinates".to_owned(),
        output: "CanvasFrame".to_owned(),
    }
}

pub(super) fn face(namespace: &str, source: &str, kind: &str, slot: &str) -> RegistrationSnapshot {
    RegistrationSnapshot {
        namespace: namespace.to_owned(),
        id: NodeId::from_namespaced_path(namespace, source, kind),
        parent: root_node_id(namespace),
        kind: kind.to_owned(),
        preset: "NoPreset".to_owned(),
        parts: "NoParts".to_owned(),
        params: kind.to_owned(),
        handle: kind.to_owned(),
        stable_name: None,
        name: OwnedLocalizedText {
            zh: kind.to_owned(),
            en: kind.to_owned(),
        },
        summary: OwnedLocalizedText {
            zh: String::new(),
            en: String::new(),
        },
        exports: Vec::new(),
        needs_registry: false,
        registry_name: slot.to_owned(),
        getting_from_other_registry: None,
        registry_rule_path: "<test>".to_owned(),
        registry_rule: RegistrationRule::ANY.into_owned(),
        admission: Admission::ANY.into_owned(),
        requires: Vec::new(),
        provides: Vec::new(),
        contract: OwnedObjectContract {
            required_parts: Vec::new(),
            provided_parts: Vec::new(),
        },
        flow: flow("render.v1"),
        flow_provider: None,
        handle_traits: Vec::new(),
        part_traits: Vec::new(),
        runtime_checks: Vec::new(),
        plugin: None,
        source: OwnedSourceLocation {
            file: source.to_owned(),
            line: 1,
            column: 1,
            function: kind.to_owned(),
        },
    }
}
