//! Fault-matrix integration tests for the plugin host adapters.
//! 插件宿主适配器的故障矩阵集成测试。

// The imports exist for the feature-gated modules below (which re-import them
// through `use super::*`), so they are gated with the same condition; otherwise
// `-D warnings` refuses them as unused when no adapter feature is on.
// 这些导入供下面按特性门控的模块使用（它们经 `use super::*` 再导入），因此用同一个条件
// 门控；否则在没有适配器特性时 `-D warnings` 会把它们当作未使用而拒绝。
#[cfg(any(feature = "wasm", feature = "process-tools"))]
use nichlink_run_method::{
    Admission, FlowContract, FrameworkId, LocalizedText, NodeId, ObjectContract, PluginArtifact,
    PluginManifest, PluginMode, PluginSource, PluginTrustPolicy, RegistrationInfo,
    RegistrationRule, RuntimeCheckSpec, SourceLocation, sha256_hex,
};

// Only the feature-gated test modules below build a plugin artifact, so without
// either adapter feature this helper is dead — which `-D dead-code` refuses.
// 只有下面按特性门控的测试模块会构造插件产物，因此在两个适配器特性都关闭时这个辅助函数
// 是死代码，而 `-D dead-code` 会拒绝它。
#[cfg(any(feature = "wasm", feature = "process-tools"))]
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
    use nichlink_plugin_host::{
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

    /// The table ceiling is wired to the store, not only declared: with it at
    /// zero even a one-element table cannot activate.
    /// 表的上限是真的接到存储上的，而不只是声明：把它设为零时，连只有一个元素的表也无法
    /// 激活。
    #[test]
    fn the_table_element_limit_is_enforced() {
        let wat = r#"(module
          (memory (export "memory") 1)
          (data (i32.const 0) "ok")
          (table 1 funcref)
          (func (export "nichlink_health") (param i32 i32) (result i64) (i64.const 2))
          (func (export "nichlink_probe") (param i32 i32) (result i64) (i64.const 2))
        )"#;
        let bytes = wat::parse_str(wat).expect("valid WAT");
        let artifact = artifact(bytes, PluginMode::Extension);
        let table = WasmPluginTable::with_backend(
            Box::leak(Box::new([slot()])),
            WasmBackend::new(WasmLimits {
                table_elements: 0,
                ..WasmLimits::default()
            }),
        )
        .unwrap();
        // Registration is lazy; activation is the first call.
        // 注册是惰性的；激活发生在第一次调用。
        table
            .install("test", ValidationChannel::Local, artifact)
            .expect("registration does not activate");
        let error = table
            .call("test", "probe", &[])
            .expect_err("a table past the ceiling must not activate");
        assert!(
            error.to_string().contains("table") || error.to_string().contains("limit"),
            "{error}"
        );
    }

    /// A hostile module cannot buy host memory through a table: the memory
    /// ceiling does not bound a table, which is a separate eagerly-instantiated
    /// array of function references.
    /// 敌对模块无法通过表买到宿主内存：内存上限并不约束表，而表是一块独立的、即时实例化的
    /// 函数引用数组。
    #[test]
    fn a_huge_table_is_refused() {
        let wat = r#"(module
          (memory (export "memory") 1)
          (data (i32.const 0) "ok")
          (table 100000000 funcref)
          (func (export "nichlink_health") (param i32 i32) (result i64) (i64.const 2))
          (func (export "nichlink_probe") (param i32 i32) (result i64) (i64.const 2))
        )"#;
        let bytes = wat::parse_str(wat).expect("valid WAT");
        let artifact = artifact(bytes, PluginMode::Extension);
        let table = WasmPluginTable::with_backend(
            Box::leak(Box::new([slot()])),
            WasmBackend::new(WasmLimits::default()),
        )
        .unwrap();
        table
            .install("test", ValidationChannel::Local, artifact)
            .expect("registration does not activate");
        let error = table
            .call("test", "probe", &[])
            .expect_err("a hundred million table entries must not be allocated");
        assert!(
            error.to_string().contains("table") || error.to_string().contains("limit"),
            "{error}"
        );
    }

    /// Compilation happens before any runtime limit can apply, so the artifact
    /// ceiling is checked by hand and reported as a limit rather than as a broken
    /// module.
    /// 编译发生在任何运行期限制生效之前，因此工件上限是手工检查的，并且报成"超限"而不是
    /// "模块损坏"。
    #[test]
    fn an_oversized_artifact_is_refused_before_compilation() {
        let bytes = wat::parse_str(ECHO).expect("valid WAT");
        let ceiling = bytes.len() - 1;
        let artifact = artifact(bytes, PluginMode::Extension);
        let table = WasmPluginTable::with_backend(
            Box::leak(Box::new([slot()])),
            WasmBackend::new(WasmLimits {
                max_module_bytes: ceiling,
                ..WasmLimits::default()
            }),
        )
        .unwrap();
        table
            .install("test", ValidationChannel::Local, artifact)
            .expect("registration does not activate");
        let error = table
            .call("test", "echo", &[])
            .expect_err("an artifact past the ceiling must not be compiled");
        assert!(error.to_string().contains("artifact is"), "{error}");
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
    use nichlink_plugin_host::{PluginInstance, ProcessBackend, ProcessLimits, ProcessProgram};
    use std::path::{Path, PathBuf};
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

    /// A child that is still running is a `Timeout`; a child that exited
    /// non-zero is a `Process` failure with its stderr.
    /// 仍在运行的子进程是 `Timeout`；非零退出的子进程是带 stderr 的 `Process` 失败。
    ///
    /// The deadline is 250 ms rather than a few milliseconds on purpose: the
    /// assertion is "still running at the deadline", and a deadline tight enough
    /// to race the scheduler makes this test fail under a loaded machine for a
    /// reason that has nothing to do with the behaviour under test. The message
    /// prints the outcome so a failure under load is diagnosable rather than a
    /// bare assertion.
    /// 超时有意取 250 ms 而不是几毫秒：断言的是"到期时它仍在运行"，而紧到与调度器赛跑的超时
    /// 会让本测试在机器繁忙时因为与被测行为无关的原因失败。消息里打印实际结果，因此负载下的
    /// 失败是可诊断的，而不是一个光秃秃的断言。
    #[test]
    fn timeout_and_crash_are_distinct_failures() {
        let (dir, path) = executable("#!/bin/sh\nsleep 2\n");
        let bytes = std::fs::read(&path).unwrap();
        let plugin = ProcessBackend::new(ProcessLimits {
            timeout: Duration::from_millis(250),
            ..ProcessLimits::default()
        })
        .load(
            artifact(bytes, PluginMode::Extension),
            ProcessProgram::new(&path),
        )
        .unwrap();
        let outcome = plugin.call("run", &[]);
        assert!(
            matches!(outcome, Err(nichlink_plugin_host::HostError::Timeout)),
            "a child that is still running must report Timeout, got {outcome:?}"
        );
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
            Err(nichlink_plugin_host::HostError::Process(_))
        ));
        drop(dir);
    }

    /// Write one plugin script and return its path.
    /// 写一个插件脚本并返回其路径。
    fn script(directory: &Path, name: &str, body: &str) -> PathBuf {
        let path = directory.join(format!("{name}.sh"));
        std::fs::write(&path, body).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        path
    }

    /// Encode one response frame: the length prefix the host reads, then the body.
    /// 编码一个响应帧：宿主会读取的长度前缀，然后负载。
    fn framed(body: &[u8]) -> Vec<u8> {
        let mut frame = Vec::with_capacity(4 + body.len());
        frame.extend_from_slice(&(body.len() as u32).to_le_bytes());
        frame.extend_from_slice(body);
        frame
    }

    /// Stage `bytes` as a plugin executable and load it with `limits`.
    /// 把 `bytes` 暂存为插件可执行文件，并以 `limits` 加载它。
    fn load(path: &Path, limits: ProcessLimits) -> impl PluginInstance {
        let bytes = std::fs::read(path).unwrap();
        ProcessBackend::new(limits)
            .load(
                artifact(bytes, PluginMode::Extension),
                ProcessProgram::new(path),
            )
            .unwrap()
    }

    /// Build a plugin that emits the prepared `frame` on stdout without reading stdin.
    /// 构造一个插件：不读 stdin，把预先准备的 `frame` 写到 stdout。
    fn emitter(directory: &Path, name: &str, frame: &[u8]) -> PathBuf {
        let payload = directory.join(format!("{name}.frame"));
        std::fs::write(&payload, frame).unwrap();
        script(
            directory,
            name,
            &format!("#!/bin/sh\nexec cat {}\n", payload.display()),
        )
    }

    /// A response larger than a pipe buffer is an ordinary response, not a timeout.
    /// 大于一个管道缓冲的响应是正常响应，而不是超时。
    ///
    /// This is the regression pin for the measured boundary: frames of 65_532 bytes
    /// succeeded and 65_537 bytes reported `Timeout` before the pipes were drained.
    /// 这是实测边界的回归钉子：在管道被排空之前，65_532 字节的帧成功而 65_537 字节的帧
    /// 报 `Timeout`。
    #[test]
    fn output_larger_than_the_pipe_buffer_is_returned() {
        for body in [65_532_usize, 65_533, 65_536, 65_537, 200 * 1024] {
            let directory = tempfile::tempdir().unwrap();
            let payload = vec![b'x'; body];
            let plugin = emitter(directory.path(), "emit", &framed(&payload));
            let loaded = load(&plugin, ProcessLimits::default());
            assert_eq!(
                loaded.call("run", &[]).unwrap().len(),
                body,
                "a {body}-byte response is inside the declared 1 MiB limit"
            );
        }
    }

    /// A child that keeps writing after its answer must still deliver it.
    /// Reading only the first frame left the child blocked on a full pipe, so the
    /// host waited for an exit that could not come and killed it at the deadline,
    /// reporting `Timeout` and discarding an answer it already had.
    /// 已经给出答案却继续写入的子进程仍必须交付那个答案。只读第一帧会让子进程阻塞在满管道
    /// 上，于是宿主去等一个不可能到来的退出、在超时点杀掉它，报出 `Timeout` 并丢掉它其实
    /// 已经拿到的答案。
    #[test]
    fn a_child_that_writes_past_its_answer_still_delivers_it() {
        let directory = tempfile::tempdir().unwrap();
        let payload = b"answer";
        let frame = directory.path().join("answer.frame");
        std::fs::write(&frame, framed(payload)).unwrap();
        let plugin = script(
            directory.path(),
            "chatty",
            &format!(
                "#!/bin/sh\ncat {}\nhead -c 200000 /dev/zero\n",
                frame.display()
            ),
        );
        let loaded = load(&plugin, ProcessLimits::default());
        assert_eq!(loaded.call("run", &[]).unwrap(), payload);
    }

    /// The declared output cap is refused as `Limit`, and the refusal survives the
    /// child, instead of degrading into a misleading `Timeout`.
    /// 声明的输出上限以 `Limit` 拒绝，且该拒绝不会被降级成误导性的 `Timeout`。
    #[test]
    fn output_over_the_declared_limit_is_a_limit_failure() {
        let directory = tempfile::tempdir().unwrap();
        let plugin = emitter(directory.path(), "emit", &framed(&vec![b'x'; 4096]));
        let loaded = load(
            &plugin,
            ProcessLimits {
                max_output_bytes: 1024,
                ..ProcessLimits::default()
            },
        );
        assert!(matches!(
            loaded.call("run", &[]),
            Err(nichlink_plugin_host::HostError::Limit(_))
        ));
    }

    /// The deadline must bound the input write, not only the child's run.
    /// 超时必须约束输入写入，而不只是子进程的运行。
    ///
    /// Measured before the fix: a 1 MiB input (exactly `max_input_bytes`) to a child
    /// that never reads stdin blocked for the child's whole lifetime (5 s) and then
    /// reported a broken pipe, so the 2 s deadline never applied at all.
    /// 修复前实测：1 MiB 输入（正好等于 `max_input_bytes`）写给从不读 stdin 的子进程，
    /// 会阻塞整个子进程生存期（5 秒），随后报 broken pipe——2 秒超时根本没有生效。
    #[test]
    fn a_child_that_never_reads_stdin_still_hits_the_deadline() {
        let directory = tempfile::tempdir().unwrap();
        let plugin = script(directory.path(), "sleepy", "#!/bin/sh\nsleep 5\n");
        let loaded = load(
            &plugin,
            ProcessLimits {
                timeout: Duration::from_millis(200),
                ..ProcessLimits::default()
            },
        );
        let start = std::time::Instant::now();
        let outcome = loaded.call("run", &vec![7_u8; 1024 * 1024]);
        assert!(
            matches!(outcome, Err(nichlink_plugin_host::HostError::Timeout)),
            "expected the deadline to win, got {outcome:?}"
        );
        assert!(
            start.elapsed() < Duration::from_secs(2),
            "the deadline must bound the input write, took {:?}",
            start.elapsed()
        );
    }

    /// An input larger than a pipe buffer still reaches a child that reads it.
    /// 大于一个管道缓冲的输入仍然能到达读取它的子进程。
    #[test]
    fn input_larger_than_the_pipe_buffer_reaches_a_reading_child() {
        let directory = tempfile::tempdir().unwrap();
        let payload = directory.path().join("ok.frame");
        std::fs::write(&payload, framed(b"ok")).unwrap();
        let plugin = script(
            directory.path(),
            "drain",
            &format!(
                "#!/bin/sh\ncat > /dev/null\nexec cat {}\n",
                payload.display()
            ),
        );
        let loaded = load(&plugin, ProcessLimits::default());
        assert_eq!(loaded.call("run", &vec![7_u8; 1024 * 1024]).unwrap(), b"ok");
    }

    /// A child that floods stderr is drained too, so it cannot block the call.
    /// 向 stderr 灌大量数据的子进程同样被排空，因此不会阻塞调用。
    #[test]
    fn stderr_larger_than_the_pipe_buffer_does_not_block() {
        let directory = tempfile::tempdir().unwrap();
        let noise = directory.path().join("noise.bin");
        std::fs::write(&noise, vec![b'e'; 200 * 1024]).unwrap();
        let plugin = script(
            directory.path(),
            "noisy",
            &format!("#!/bin/sh\ncat {} >&2\nexit 7\n", noise.display()),
        );
        let loaded = load(
            &plugin,
            ProcessLimits {
                timeout: Duration::from_secs(3),
                ..ProcessLimits::default()
            },
        );
        let start = std::time::Instant::now();
        let outcome = loaded.call("run", &[]);
        assert!(
            matches!(outcome, Err(nichlink_plugin_host::HostError::Process(_))),
            "expected the child's failure, got {outcome:?}"
        );
        assert!(
            start.elapsed() < Duration::from_secs(2),
            "stderr must be drained, took {:?}",
            start.elapsed()
        );
    }
}
