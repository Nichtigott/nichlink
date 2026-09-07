//! Repeatable registration scale audit for release qualification.

use std::time::Instant;

use nichlink::{
    FrameworkId, NodeId, OwnedAdmission, OwnedFlowContract, OwnedLocalizedText,
    OwnedObjectContract, OwnedRegistrationRule, OwnedSourceLocation, RegistrationSnapshot,
    Registry, root_node_id,
};

fn snapshot(namespace: &str, index: usize, parent: NodeId) -> RegistrationSnapshot {
    let kind = format!("Face{index}");
    RegistrationSnapshot {
        namespace: namespace.to_owned(),
        id: NodeId::from_namespaced_path(namespace, &format!("scale/{index}.rs"), &kind),
        parent,
        kind: kind.clone(),
        preset: "default".to_owned(),
        parts: String::new(),
        params: String::new(),
        handle: kind.clone(),
        stable_name: None,
        name: OwnedLocalizedText {
            zh: kind.clone(),
            en: kind.clone(),
        },
        summary: OwnedLocalizedText {
            zh: String::new(),
            en: String::new(),
        },
        exports: Vec::new(),
        needs_registry: false,
        registry_name: format!("face-{index}"),
        getting_from_other_registry: None,
        registry_rule_path: "<scale-audit>".to_owned(),
        registry_rule: OwnedRegistrationRule {
            required_preset: None,
            required_parts: Vec::new(),
            required_exports: Vec::new(),
            required_handle_traits: Vec::new(),
            required_part_traits: Vec::new(),
        },
        admission: OwnedAdmission {
            allowed_paths: Vec::new(),
            denied_paths: Vec::new(),
        },
        requires: Vec::new(),
        provides: Vec::new(),
        contract: OwnedObjectContract {
            required_parts: Vec::new(),
            provided_parts: Vec::new(),
            expected_output: String::new(),
            actual_output: String::new(),
        },
        flow: OwnedFlowContract::none(),
        flow_provider: None,
        handle_traits: Vec::new(),
        part_traits: Vec::new(),
        runtime_checks: Vec::new(),
        plugin: None,
        source: OwnedSourceLocation {
            file: format!("scale/{index}.rs"),
            line: 1,
            column: 1,
            function: kind,
        },
    }
}

fn main() {
    let sizes = std::env::args()
        .skip(1)
        .map(|value| value.parse::<usize>().expect("size must be an integer"))
        .collect::<Vec<_>>();
    let sizes = if sizes.is_empty() {
        vec![10_000, 100_000]
    } else {
        sizes
    };
    println!("nodes\tregister_ms\tindex_ms\tentries\tpages\tstatic_face_bytes");
    for size in sizes {
        let namespace = format!("scale-{size}");
        let root = Registry::root_for_namespace(FrameworkId::new("nichlink.scale"), &namespace);
        let parent = root_node_id(&namespace);
        let submissions = (0..size)
            .map(|index| snapshot(&namespace, index, parent))
            .collect::<Vec<_>>();
        let mut registry = root;
        let register_start = Instant::now();
        registry
            .register_snapshot_batch(submissions)
            .expect("generated scale batch must register");
        let register_ms = register_start.elapsed().as_millis();
        let index_start = Instant::now();
        let index = registry.index();
        let index_ms = index_start.elapsed().as_millis();
        let stats = registry.storage_stats();
        println!(
            "{size}\t{register_ms}\t{index_ms}\t{}\t{}\t{}",
            index.len(),
            stats.pages,
            size * std::mem::size_of::<nichlink::StaticFace>()
        );
        let extra = snapshot(&namespace, size, parent);
        registry
            .register_snapshot_batch([extra])
            .expect("incremental transaction must register");
        assert_eq!(registry.index().len(), size + 2);
    }
}
