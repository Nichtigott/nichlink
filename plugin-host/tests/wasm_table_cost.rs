//! What a declared Wasm table costs the host, measured rather than assumed.
//! 声明的 Wasm 表在宿主一侧的开销：实测，而不是假设。

// The measurement is about the Wasm backend, so the whole binary is gated on the
// feature that provides it. Without the gate, `cargo check --workspace
// --no-default-features --all-targets` would try to compile this file against a
// plugin host that has no `WasmBackend` at all.
// 本测量针对 Wasm 后端，因此整个二进制门控在提供它的特性上。没有这道门控，
// `cargo check --workspace --no-default-features --all-targets` 会拿一个根本没有
// `WasmBackend` 的插件宿主来编译本文件。
#![cfg(feature = "wasm")]

// `M3` in `docs/audit-production-readiness.md` fixed an unbounded table by adding
// `WasmLimits::table_elements`, and `U4` recorded that the *magnitude* was never
// measured: the ceiling was verified to refuse, not verified to matter. This file
// supplies the missing number.
//
// Why the counter: the host-side cost of `(table N funcref)` is a number of bytes
// the engine allocates, and reading it out of the engine is not possible — wasmi
// exposes no table-size query. A counting global allocator measures exactly the
// quantity in question, with no dependence on RSS granularity, page cache, or
// what the rest of the machine is doing.
//
// Why one test in its own binary: the counter is process-global, so a sibling
// test allocating on another thread would be attributed to this measurement. One
// test in one file is the whole isolation mechanism, which is also why this file
// duplicates the small artifact fixture instead of sharing `fault_matrix`'s.
//
// `docs/audit-production-readiness.md` 里的 `M3` 通过新增 `WasmLimits::table_elements`
// 修掉了无上限的表，而 `U4` 记下的是**量级**从未被测：只验证了上限会拒绝，没验证它值多少。
// 本文件补上这个数字。
//
// 为什么用计数器：`(table N funcref)` 在宿主一侧的开销就是引擎分配的字节数，而这个数无法从
// 引擎读出来——wasmi 没有暴露任何查询表大小的接口。计数式全局分配器测量的正是这个量，且不依赖
// RSS 粒度、页缓存或机器上其他东西在做什么。
//
// 为什么单独一个二进制里只放一个测试：计数器是进程全局的，另一个线程上的兄弟测试一旦分配就会
// 被算进本测量。一个文件一个测试就是全部隔离手段，这也是本文件复刻那小块产物夹具、而不与
// `fault_matrix` 共用的原因。

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use nichlink_plugin_host::{
    ValidationChannel, WasmBackend, WasmLimits, WasmPluginSlot, WasmPluginTable,
};
use nichlink_run_method::{
    Admission, FlowContract, FrameworkId, LocalizedText, NodeId, ObjectContract, PluginArtifact,
    PluginManifest, PluginMode, PluginSource, PluginTrustPolicy, RegistrationInfo,
    RegistrationRule, RuntimeCheckSpec, SourceLocation, sha256_hex,
};

/// Bytes currently allocated through the counting allocator.
/// 经计数分配器当前已分配的字节数。
static LIVE_BYTES: AtomicUsize = AtomicUsize::new(0);

/// High-water mark of [`LIVE_BYTES`] since the last reset.
/// 自上次重置以来 [`LIVE_BYTES`] 的高水位。
static PEAK_BYTES: AtomicUsize = AtomicUsize::new(0);

/// A `System` allocator that keeps the two counters above in step.
/// 一个让上面两个计数器保持同步的 `System` 分配器。
struct Counting;

impl Counting {
    /// Add one successful allocation to both counters.
    /// 把一次成功的分配计入两个计数器。
    fn record(size: usize) {
        let live = LIVE_BYTES.fetch_add(size, Ordering::Relaxed) + size;
        PEAK_BYTES.fetch_max(live, Ordering::Relaxed);
    }

    /// Remove one freed or moved allocation from the live total.
    /// 从存活总量中扣除一次被释放或移动的分配。
    fn release(size: usize) {
        LIVE_BYTES.fetch_sub(size, Ordering::Relaxed);
    }
}

// SAFETY: every method forwards to `System` unchanged; the counters only observe
// the layout sizes it was already given, and no allocation decision depends on
// them. `realloc` counts the new size and releases the old one, so a `Vec` that
// grows is attributed to the peak rather than to its final size.
// 安全性：每个方法都原样转发给 `System`；计数器只观察它本来就收到的布局大小，没有任何分配
// 决定依赖它们。`realloc` 计入新尺寸并释放旧尺寸，因此一个增长中的 `Vec` 计入峰值而不是
// 最终尺寸。
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            Self::record(layout.size());
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            Self::record(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        Self::release(layout.size());
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let moved = unsafe { System.realloc(pointer, layout, new_size) };
        if !moved.is_null() {
            Self::release(layout.size());
            Self::record(new_size);
        }
        moved
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// One slot definition, matching the shape the other backend tests use.
/// 一个槽定义，与后端其他测试使用的形状一致。
fn slot() -> WasmPluginSlot {
    WasmPluginSlot::new(
        "test",
        FrameworkId::new("nichlink.test"),
        PluginMode::Extension,
        FlowContract::NONE,
        &[ValidationChannel::Local],
    )
}

/// A verified artifact whose module declares a table of `elements` entries.
/// 一个已验证产物，其模块声明一张有 `elements` 个条目的表。
fn artifact_with_table(elements: usize) -> nichlink_run_method::VerifiedPluginArtifact {
    let wat = format!(
        r#"(module
          (memory (export "memory") 1)
          (data (i32.const 0) "ok")
          (table {elements} funcref)
          (func (export "nichlink_health") (param i32 i32) (result i64) (i64.const 2))
          (func (export "nichlink_probe") (param i32 i32) (result i64) (i64.const 2))
        )"#
    );
    let bytes = wat::parse_str(&wat).expect("the generated module is valid WAT");
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
            mode: PluginMode::Extension,
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
    .expect("the test artifact's digest must verify")
}

/// Activate one module declaring `elements` table entries under a backend whose
/// ceiling is `limit`, and report the peak bytes allocated while doing it.
/// 在一个上限为 `limit` 的后端下激活一个声明 `elements` 个表条目的模块，并报告这段时间内
/// 分配峰值的字节数。
fn measure(elements: usize, limit: usize) -> (usize, Result<(), String>) {
    let artifact = artifact_with_table(elements);
    let table = WasmPluginTable::with_backend(
        Box::leak(Box::new([slot()])),
        WasmBackend::new(WasmLimits {
            table_elements: limit,
            ..WasmLimits::default()
        }),
    )
    .expect("the slot definition is valid");
    table
        .install("test", ValidationChannel::Local, artifact)
        .expect("install only queues the artifact");
    // Reset the high-water mark to what is live right now, so the reading covers
    // activation alone: the WAT text and the parsed module were built above.
    // 把高水位重置为此刻的存活量，使读数只覆盖激活这一步：WAT 文本与解析后的模块都在上面
    // 构造完毕。
    let baseline = LIVE_BYTES.load(Ordering::SeqCst);
    PEAK_BYTES.store(baseline, Ordering::SeqCst);
    let outcome = table.call("test", "probe", &[]).map(|_| ());
    let peak = PEAK_BYTES.load(Ordering::SeqCst).saturating_sub(baseline);
    (peak, outcome.map_err(|error| error.to_string()))
}

/// One measured point, so the two halves of the measurement are named.
/// 一个实测点，使测量的两半各有名字。
#[test]
fn a_declared_table_costs_the_host_by_the_element_and_the_ceiling_refuses_it_first() {
    // A million entries is large enough that per-element cost is not dominated by
    // fixed overhead, and small enough that measuring it does not disturb the
    // machine. The ceiling is raised to match, because the point here is the cost
    // of a table the host agreed to.
    // 一百万条目足够大，使每元素开销不被固定开销淹没；又足够小，不打扰整台机器。上限同步抬高，
    // 因为这里要测的是宿主同意了的那张表的开销。
    const MEASURED: usize = 1_048_576;
    let (allowed_peak, outcome) = measure(MEASURED, MEASURED);
    outcome.expect("a table at the ceiling must instantiate");
    let per_element = allowed_peak / MEASURED;
    println!(
        "table of {MEASURED} elements: {allowed_peak} bytes peak, {per_element} bytes/element"
    );
    assert!(
        (1..=64).contains(&per_element),
        "a function reference costs a pointer or two, not {per_element} bytes: the \
         measurement's own assumption is wrong and the extrapolation below would be too"
    );

    // The same declaration under the shipped ceiling is refused. The measured
    // peak is what settles U4's exploitability half: the limiter denies the
    // allocation *before* the table exists, so the refusal costs less than one
    // table the host did agree to.
    // 同样的声明在出厂上限下被拒绝。实测峰值正是 U4 可利用性那一半的答案：限制器在表存在
    // **之前**就拒绝了这次分配，因此拒绝的代价小于宿主确实同意的一张表。
    let (refused_peak, outcome) = measure(100_000_000, MEASURED);
    let error = outcome.expect_err("a hundred million entries must not be honoured");
    assert!(
        error.contains("table") || error.contains("limit"),
        "the refusal must name the ceiling, not arrive as a broken module: {error}"
    );
    println!("hundred-million-entry table refused after {refused_peak} bytes peak");
    assert!(
        refused_peak < allowed_peak,
        "refusing {refused_peak} bytes must cost less than the {allowed_peak} bytes a table at \
         the ceiling costs; otherwise the ceiling is paid for and then checked"
    );

    // What the ceiling prevents, as arithmetic on the measured rate rather than a
    // second guess: the table a hostile module would have bought.
    // 上限阻止了什么——这是在实测速率上做的算术，而不是第二次猜测：敌对模块本来会买到的那张表。
    let unrefused = 100_000_000 * per_element;
    println!(
        "without the ceiling, that module would have cost about {} MiB",
        unrefused / (1024 * 1024)
    );
    assert!(
        unrefused > 100 * 1024 * 1024,
        "the ceiling has to be preventing hundreds of megabytes to be worth its own existence"
    );
}
