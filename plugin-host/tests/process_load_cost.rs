//! What a mismatched process executable costs the host before it is refused.
//! 进程可执行文件与已验证字节不符时，宿主在拒绝它之前付出的代价。
//!
//! `ProcessBackend::load` compares the staged program against the verified bytes
//! before anything runs. The comparison used to be the first use of the file's
//! size, so a file that could not possibly match — a 256 MiB sparse file whose
//! digest was never going to agree — was read into memory in full and only then
//! refused. The reading is what this file measures: a length check that runs
//! first must cost a constant, not the size of the file it refuses.
//! `ProcessBackend::load` 会在任何东西运行之前，把暂存程序与已验证字节比对。那次比对过去是
//! 第一次用到文件尺寸，因此一个不可能匹配的文件——摘要永远不会一致的 256 MiB 稀疏文件——会被
//! 整份读进内存后才被拒绝。本文件测的就是这次读取：先跑的长度检查必须是常数代价，而不是它所
//! 拒绝的那个文件的尺寸。
//!
//! Why the counter: the cost in question is bytes allocated, and RSS is the wrong
//! instrument — the kernel may hand back unused pages, and the reading may be
//! attributed to the page cache instead of the process. A counting global
//! allocator measures exactly the quantity in question.
//! 为什么用计数器：要测的量就是分配的字节数，而 RSS 是错误的仪器——内核可能根本不把未用页面
//! 交给进程，读取还可能被记在页缓存头上。计数式全局分配器测量的正是这个量。
//!
//! Why one test in its own binary: the counter is process-global, so a sibling
//! test allocating on another thread would be attributed to this measurement. One
//! test in one file is the whole isolation mechanism, which is why this file
//! duplicates the small artifact fixture instead of borrowing another's.
//! 为什么单独一个二进制里只放一个测试：计数器是进程全局的，另一个线程上的兄弟测试一旦分配就会
//! 被算进本测量。一个文件一个测试就是全部隔离手段，这也是本文件复刻那小块产物夹具、而不借用
//! 别处的原因。
#![cfg(feature = "process-tools")]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use nichlink_plugin_host::{ProcessBackend, ProcessProgram};
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

/// A verified artifact over `bytes`, shaped like the other backend tests'.
/// 覆盖 `bytes` 的已验证产物，形状与后端其他测试一致。
fn artifact(bytes: Vec<u8>) -> nichlink_run_method::VerifiedPluginArtifact {
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

/// A wrong-length executable is refused without being read.
/// 长度不符的可执行文件会被拒绝，且不会被读取。
#[test]
fn a_wrong_length_executable_is_refused_without_reading_it() {
    // Large enough that reading it would dominate the measurement, and created
    // sparse so the test itself writes nothing.
    // 大到读取它会主导整个测量，同时以稀疏方式创建，使测试本身不写任何数据。
    const REFUSED_BYTES: u64 = 256 * 1024 * 1024;
    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("plugin");
    let file = std::fs::File::create(&path).expect("create the oversized program");
    file.set_len(REFUSED_BYTES)
        .expect("give it a sparse length");
    drop(file);

    let artifact = artifact(b"an honest process plugin".to_vec());
    let backend = ProcessBackend::default();
    // Reset the high-water mark to what is live right now, so the reading covers
    // the load alone: the artifact and the file were built above.
    // 把高水位重置为此刻的存活量，使读数只覆盖加载这一步：产物与文件都在上面构造完毕。
    let baseline = LIVE_BYTES.load(Ordering::SeqCst);
    PEAK_BYTES.store(baseline, Ordering::SeqCst);
    let outcome = backend.load(artifact, ProcessProgram::new(&path));
    let peak = PEAK_BYTES.load(Ordering::SeqCst).saturating_sub(baseline);

    let error = match outcome {
        Ok(_) => panic!("a program of the wrong length must not load"),
        Err(error) => error,
    };
    assert!(
        error.to_string().contains("differs"),
        "the refusal must name the mismatch, not arrive as an I/O failure: {error}"
    );
    println!("refusing a {REFUSED_BYTES}-byte mismatch cost {peak} bytes peak");
    assert!(
        peak < 1024 * 1024,
        "the refusal allocated {peak} bytes; the length check has to run before the read, so the \
         cost must not follow the size of the file it refuses"
    );
}
