use nichlink_run_method::{
    Admission, FlowContract, FrameworkId, LocalizedText, NodeId, ObjectContract, PluginArtifact,
    PluginManifest, PluginMode, PluginSource, PluginTrustPolicy, RegistrationInfo,
    RegistrationRule, RuntimeCheckSpec, SourceLocation, sha256_hex,
};

fn artifact(bytes: Vec<u8>, mode: PluginMode) -> nichlink_run_method::VerifiedPluginArtifact {
    let checksum = Box::leak(sha256_hex(&bytes).into_boxed_str());
    let registration = RegistrationInfo {
        namespace: "plugin-test",
        id: NodeId::from_path("plugin.rs", "plugin"),
        parent: nichlink_run_method::ROOT_NODE_ID,
        kind: "Plugin",
        preset: "",
        parts: "",
        params: "",
        handle: "PluginHandle",
        stable_name: None,
        name: LocalizedText {
            zh: "插件",
            en: "Plugin",
        },
        summary: LocalizedText { zh: "", en: "" },
        exports: &[],
        needs_registry: false,
        registry_name: "plugin",
        getting_from_other_registry: None,
        registry_rule_path: "<test>",
        registry_rule: RegistrationRule::ANY,
        admission: Admission::ANY,
        requires: &[],
        provides: &[],
        contract: ObjectContract {
            required_parts: &[],
            provided_parts: &[],
            expected_output: "",
            actual_output: "",
        },
        flow: FlowContract::NONE,
        flow_provider: None,
        handle_traits: &[],
        part_traits: &[],
        runtime_checks: &[] as &[RuntimeCheckSpec],
        plugin: Some(PluginManifest {
            name: "plugin-test",
            crate_name: "plugin_test",
            version: "1.0.0",
            framework: FrameworkId::new("nichlink.test"),
            source: PluginSource::User,
            mode,
            checksum,
            signature: None,
            public_key_fingerprint: None,
            revocation_list: None,
        }),
        source: SourceLocation {
            file: "plugin.rs",
            line: 1,
            column: 1,
            function: "plugin",
        },
    };
    PluginArtifact {
        registration,
        bytes,
        key_fingerprint: None,
    }
    .verify_artifact(PluginTrustPolicy::open())
    .expect("test artifact digest must verify")
}

#[cfg(feature = "wasm")]
mod wasm_faults {
    use super::*;
    use nichlink_host::{
        ValidationChannel, WasmBackend, WasmLimits, WasmPluginSlot, WasmPluginTable,
    };

    fn slot() -> WasmPluginSlot {
        WasmPluginSlot::new(
            "test",
            FrameworkId::new("nichlink.test"),
            PluginMode::Extension,
            FlowContract::NONE,
            &[ValidationChannel::Local],
        )
    }

    fn table(wat: &str, limits: WasmLimits) -> WasmPluginTable {
        let bytes = wat::parse_str(wat).expect("valid WAT");
        let artifact = artifact(bytes, PluginMode::Extension);
        let table =
            WasmPluginTable::with_backend(Box::leak(Box::new([slot()])), WasmBackend::new(limits))
                .unwrap();
        table
            .install("test", ValidationChannel::Local, artifact)
            .unwrap();
        table
    }

    const ECHO: &str = r#"
        (module
          (memory (export "memory") 1)
          (data (i32.const 0) "ok")
          (func (export "nichlink_health") (param i32 i32) (result i64) (i64.const 2))
          (func (export "nichlink_echo") (param i32 i32) (result i64)
            (i64.extend_i32_u (local.get 1)))
        )
    "#;

    #[test]
    fn lazy_activation_and_generation_are_observable() {
        let table = table(ECHO, WasmLimits::default());
        assert!(!table.is_loaded("test").unwrap());
        assert_eq!(table.call("test", "echo", b"abc").unwrap(), b"abc");
        assert!(table.is_loaded("test").unwrap());
        assert_eq!(table.generation("test").unwrap(), Some(1));
    }

    #[test]
    fn incompatible_abi_is_rejected_before_instance_is_published() {
        let wat = r#"(module
          (memory (export "memory") 1)
          (func (export "nichlink_abi_version") (result i32) (i32.const 99))
          (func (export "nichlink_health") (param i32 i32) (result i64) (i64.const 2)))"#;
        let bytes = wat::parse_str(wat).expect("valid WAT");
        let error = match WasmBackend::default().load(artifact(bytes, PluginMode::Extension)) {
            Ok(_) => panic!("ABI mismatch must reject the instance"),
            Err(error) => error,
        };
        let message = error.to_string();
        assert!(
            message.contains("ABI") && message.contains("incompatible"),
            "{message}"
        );
    }

    #[test]
    fn fuel_exhaustion_is_reported() {
        let wat = r#"(module
          (memory (export "memory") 1)
          (data (i32.const 0) "ok")
          (func (export "nichlink_health") (param i32 i32) (result i64) (i64.const 2))
          (func $spin (param i32 i32) (result i64)
            (call $spin (local.get 0) (local.get 1)))
          (export "nichlink_spin" (func $spin))
        )"#;
        let table = table(
            wat,
            WasmLimits {
                fuel_per_call: 100,
                ..WasmLimits::default()
            },
        );
        let error = table.call("test", "spin", &[]).unwrap_err().to_string();
        assert!(
            error.contains("fuel") || error.contains("out of fuel"),
            "{error}"
        );
    }

    #[test]
    fn output_limit_is_enforced_before_memory_read() {
        let wat = r#"(module
          (memory (export "memory") 1)
          (data (i32.const 0) "ok")
          (func (export "nichlink_health") (param i32 i32) (result i64) (i64.const 2))
          (func (export "nichlink_large") (param i32 i32) (result i64) (i64.const 4294967396)
          )
        )"#;
        let table = table(
            wat,
            WasmLimits {
                max_output_bytes: 8,
                ..WasmLimits::default()
            },
        );
        let error = table.call("test", "large", &[]).unwrap_err().to_string();
        assert!(
            error.contains("output") && error.contains("limit"),
            "{error}"
        );
    }

    #[test]
    fn linear_memory_growth_is_capped() {
        let wat = r#"(module
          (memory (export "memory") 1)
          (data (i32.const 0) "ok")
          (func (export "nichlink_health") (param i32 i32) (result i64) (i64.const 2))
          (func (export "nichlink_grow") (param i32 i32) (result i64)
            (drop (memory.grow (i32.const 1000))) (i64.const 2))
        )"#;
        let table = table(
            wat,
            WasmLimits {
                memory_bytes: 64 * 1024,
                ..WasmLimits::default()
            },
        );
        let error = table.call("test", "grow", &[]).unwrap_err().to_string();
        assert!(
            error.contains("memory") || error.contains("trap") || error.contains("limited"),
            "{error}"
        );
    }
}

#[cfg(feature = "process-tools")]
mod process_faults {
    use super::*;
    use nichlink_host::{PluginInstance, ProcessBackend, ProcessLimits, ProcessProgram};
    use std::time::Duration;

    fn executable(script: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("plugin.sh");
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(script.as_bytes()).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        (dir, path)
    }

    #[test]
    fn timeout_and_crash_are_distinct_failures() {
        let (dir, path) = executable("#!/bin/sh\nsleep 2\n");
        let bytes = std::fs::read(&path).unwrap();
        let plugin = ProcessBackend::new(ProcessLimits {
            timeout: Duration::from_millis(20),
            ..ProcessLimits::default()
        })
        .load(
            artifact(bytes, PluginMode::Extension),
            ProcessProgram::new(&path),
        )
        .unwrap();
        assert!(matches!(
            plugin.call("run", &[]),
            Err(nichlink_host::HostError::Timeout)
        ));
        drop(dir);

        let (dir, path) = executable("#!/bin/sh\nexit 7\n");
        let bytes = std::fs::read(&path).unwrap();
        let plugin = ProcessBackend::default()
            .load(
                artifact(bytes, PluginMode::Extension),
                ProcessProgram::new(&path),
            )
            .unwrap();
        assert!(matches!(
            plugin.call("run", &[]),
            Err(nichlink_host::HostError::Process(_))
        ));
        drop(dir);
    }
}
