//! Allocation counts for the release read path, measured instead of argued.
//! 发布读路径的分配计数：实测，而不是论证。

// `README.md`'s cost table makes three claims this file settles:
// `README.md` 的成本表有三条断言由本文件定案：
//
// - "Read-only built-in topology | Static `StaticFace` slice | No startup
//   allocation": every read of the built-in plan must allocate nothing at all.
// - "只读内置拓扑 | 静态 `StaticFace` 切片 | 启动时零分配"：对内置计划的每次读取都必须
//   完全不分配。
// - "Build-declared static graft | Static selector slice inside `StaticPlan` |
//   Reading the declaration allocates nothing": same, for `grafts()`.
// - "构建期声明的静态 graft | `StaticPlan` 内的静态选择器切片 | 读该声明不分配"：同上，
//   针对 `grafts()`。
// - "`overlay_static` allocates no plan but still performs contract, admission,
//   and connector validation once": the honest reading, recorded in
//   `docs/audit-graft-vs-readme.md` as C17, is "allocates no *plan*" rather than
//   "allocates nothing" — so this file measures the static overlay against the
//   dynamic one over the same cut rather than asserting zero.
// - "`overlay_static` 不分配计划，但仍做一次合同/接纳/连接器校验"：诚实的读法（记在
//   `docs/audit-graft-vs-readme.md` 的 C17）是"不分配**计划**"，而不是"什么都不分配"——
//   因此本文件不假设零，而是在同一个切口上把静态 overlay 与动态 overlay 对比测量。
//
// Why a counting global allocator rather than a profiler: the claim is a count of
// heap allocations on one code path, and `dhat` or `valgrind` are not installed
// here. A counting allocator measures exactly that quantity and needs no external
// tool. It is process-global, which is why this file holds only two tests: they
// run in parallel with each other in the same binary, so each one brackets its
// measurement and asserts on it immediately, and the second test's assertion is a
// comparison of two measurements taken under the same conditions.
// 为什么用计数式全局分配器而不是 profiler：这条断言就是某条代码路径上堆分配的次数，而本环境
// 没有装 `dhat` 或 `valgrind`。计数分配器测量的正是这个量，且不需要外部工具。它是进程全局的，
// 因此本文件只放两条测试：它们在同一二进制里彼此并行，所以每条都用自己的括号夹住测量并立刻断言，
// 第二条断言的还是同条件下两次测量的对比。

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use control_button::{FRAMEWORK, base_registry, builtin_static_plan};
use nichlink_run_method::registry_core::{GraftPlan, Registry};

/// Number of successful allocations since the last reset.
/// 自上次重置以来成功分配的次数。
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

/// Bytes of those allocations, summed.
/// 这些分配的字节数之和。
static BYTES: AtomicUsize = AtomicUsize::new(0);

/// A `System` allocator that counts successful allocations.
/// 一个统计成功分配次数的 `System` 分配器。
struct Counting;

// SAFETY: every method forwards to `System` unchanged and only observes the
// layout size it was given; no allocation decision depends on the counters.
// `dealloc` and `realloc` are not counted, because the question is how many
// allocations a path performs, not how many bytes it holds at the end. A
// `realloc` counts as one more allocation, which is what a growing `Vec` does.
// 安全性：每个方法都原样转发给 `System`，只观察它收到的布局大小；没有任何分配决定依赖这些
// 计数器。`dealloc` 与 `realloc` 不计数，因为问题是一条路径做了多少次分配，而不是它最后占多少
// 字节。`realloc` 计作一次新的分配，增长中的 `Vec` 正是如此。
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let moved = unsafe { System.realloc(pointer, layout, new_size) };
        if !moved.is_null() {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            BYTES.fetch_add(new_size, Ordering::Relaxed);
        }
        moved
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// Allocations and bytes performed while `body` runs, as a pair.
/// `body` 运行期间发生的分配次数与字节数，成对给出。
fn measure<T>(body: impl FnOnce() -> T) -> (usize, usize, T) {
    let allocations = ALLOCATIONS.load(Ordering::SeqCst);
    let bytes = BYTES.load(Ordering::SeqCst);
    let value = body();
    (
        ALLOCATIONS.load(Ordering::SeqCst) - allocations,
        BYTES.load(Ordering::SeqCst) - bytes,
        value,
    )
}

/// The read path the cost table calls "no startup allocation": the built-in plan
/// itself, its two slices, and the two queries the table names, `find` and
/// `children_of`.
/// 成本表称为"启动时零分配"的读路径：内置计划本身、它的两个切片，以及表中点名的两个查询
/// `find` 与 `children_of`。
fn assert_the_static_read_path_allocates_nothing() {
    let plan = builtin_static_plan();
    // A hit and a miss for `find`, and a query for a parent that has children:
    // an implementation that allocated only on one branch would still be caught.
    // `find` 命中与落空各一次、以及对一个有子级的父级的查询：只在某个分支上分配的实现同样会被
    // 抓到。
    let first = plan
        .faces()
        .first()
        .expect("the example declares faces")
        .id();
    let missing = nichlink_run_method::registry_core::NodeId::from_path("<absent>", "Absent");

    let (allocations, bytes, observed) = measure(|| {
        let plan = builtin_static_plan();
        let mut seen = 0usize;
        for face in plan.faces() {
            seen += usize::from(face.owns_registry());
            let _ = face.parent();
        }
        for cut in plan.grafts() {
            let _ = cut.cut();
            let _ = cut.full();
        }
        let hit = plan.find(first).is_some();
        let miss = plan.find(missing).is_none();
        let children = plan.children_of(first).count();
        (plan.len(), plan.is_empty(), seen, hit, miss, children)
    });
    println!("static read path: {allocations} allocations, {bytes} bytes; observed {observed:?}");
    assert_eq!(
        allocations, 0,
        "reading the built-in plan allocated {bytes} bytes over {allocations} allocations"
    );
    assert_eq!(bytes, 0, "no allocation may be counted either way");
    assert!(!observed.1, "the example's plan is not empty");
    assert!(observed.3, "a face from the plan must be found in it");
    assert!(observed.4, "an absent id must not be found");
}

/// One test, not two: the counter is process-global, and libtest runs a file's
/// tests on separate threads, so a second test allocating while the first asserts
/// "0 allocations" would make that assertion fail for a reason that has nothing
/// to do with the code under test. Both phases therefore run in one test body,
/// sequentially, and the file holds exactly one `#[test]`.
/// 只放一条测试，不放两条：计数器是进程全局的，而 libtest 会把同一文件里的测试放在不同线程上，
/// 因此第二条测试一旦在第一条断言"0 次分配"时分配，那条断言就会因为与被测代码无关的原因失败。
/// 两个阶段因此顺序跑在同一个测试体里，本文件恰好只有一条 `#[test]`。
#[test]
fn the_release_read_path_allocations_are_what_the_cost_table_claims() {
    assert_the_static_read_path_allocates_nothing();
    overlay_phases();
}

/// Where the overlay's allocations go, measured in phases so the claim can be
/// read off numbers instead of inferred from the code.
///
/// The static path is not expected to allocate *nothing*: it clones the base tree
/// and builds the set of visited cuts, which the dynamic path does too. What it
/// skips is the `GraftPlan` and its selector strings. The phases below separate
/// the shared cost (an overlay with no cuts) from the per-cut cost of each
/// spelling, and measure building the plan on its own.
/// overlay 的分配花在哪里——分阶段测量，使这条断言可以从数字读出来，而不是从代码推断。
///
/// 静态路径并不被期望"什么都不分配"：它会克隆基树并建立已访问切口的集合，动态路径也一样。
/// 它跳过的是 `GraftPlan` 及其选择器字符串。下面几个阶段把共享开销（不带任何切口的 overlay）
/// 与两种写法各自的按切口开销分开，并单独测量构造计划的代价。
fn overlay_phases() {
    let base = base_registry();
    let external = control_button_graft::external_registry();
    let static_cuts = builtin_static_plan().grafts();
    assert_eq!(
        static_cuts.len(),
        2,
        "the example hands over two slots; this comparison assumes that"
    );
    let one_static = &static_cuts[..1];
    let one_dynamic = || GraftPlan::new(FRAMEWORK).cut("root/control/button", "button_fast");
    let two_dynamic = || {
        GraftPlan::new(FRAMEWORK)
            .cut("root/control/button", "button_fast")
            .cut("root/control/slider", "slider_fast")
    };

    // The floor: an overlay with no cuts still clones the base tree and builds
    // its bookkeeping, so neither spelling can be zero.
    // 下限：不带任何切口的 overlay 仍会克隆基树并建立簿记，因此两种写法都不可能为零。
    let (empty_allocations, empty_bytes, empty) = measure(|| {
        base.overlay_static(
            &[] as &[nichlink_run_method::registry_core::StaticGraftCut],
            &external,
        )
        .expect("an overlay with no cuts is the base tree")
    });
    // Like for like: one cut each, then the example's whole plan each. Comparing
    // two cuts against one would have made the static path look 30 allocations
    // worse for no reason other than the second cut.
    // 同口径：各一个切口，然后各用示例的整份计划。拿两个切口对一个切口，会让静态路径看起来
    // 白白差 30 次分配，而差别只来自那第二个切口。
    let (one_static_allocations, one_static_bytes, _) = measure(|| {
        base.overlay_static(one_static, &external)
            .expect("static overlay")
    });
    let (one_dynamic_allocations, one_dynamic_bytes, _) = measure(|| {
        base.overlay(&one_dynamic(), &external)
            .expect("dynamic overlay")
    });
    let (two_static_allocations, two_static_bytes, statically) = measure(|| {
        base.overlay_static(static_cuts, &external)
            .expect("static overlay")
    });
    let (two_dynamic_allocations, two_dynamic_bytes, dynamically) = measure(|| {
        base.overlay(&two_dynamic(), &external)
            .expect("dynamic overlay")
    });
    let (plan_allocations, plan_bytes, _) = measure(one_dynamic);

    println!(
        "overlay allocations/bytes: empty {empty_allocations}/{empty_bytes}; \
         one cut static {one_static_allocations}/{one_static_bytes}, dynamic \
         {one_dynamic_allocations}/{one_dynamic_bytes}; two cuts static \
         {two_static_allocations}/{two_static_bytes}, dynamic {two_dynamic_allocations}/\
         {two_dynamic_bytes}; one-cut plan alone {plan_allocations}/{plan_bytes}"
    );

    // Both spellings publish the same effective tree, so this compares two ways
    // of writing one operation rather than two different results.
    // 两种写法发布同一棵有效树，因此这是同一个操作的两种写法之间的对比，而不是两个不同结果。
    assert_eq!(kind_at(&statically), "ButtonFast");
    assert_eq!(kind_at(&dynamically), "ButtonFast");
    assert_eq!(
        kind_at(&empty),
        "Button",
        "an empty overlay changes nothing"
    );
    assert_eq!(
        kind_at(&base),
        "Button",
        "the base tree must be untouched by either overlay"
    );

    // What the cost table claims about `overlay_static` is that it allocates no
    // plan. That plan is measurably not free, and the static path never builds it.
    // 成本表关于 `overlay_static` 的断言是它不分配计划。实测表明那份计划并非免费，而静态路径
    // 从不构造它。
    assert!(
        plan_allocations > 0 && plan_bytes > 0,
        "the plan the static path skips must itself cost allocations, or the sentence is empty"
    );
    // And the static path is not the cheaper one per cut, which is worth knowing
    // before reading "skips the plan" as "cheaper". The numbers are printed above
    // and recorded in `docs/performance-baseline.md`.
    // 而按切口算，静态路径并不是更便宜的那一个——在把"跳过计划"读成"更便宜"之前，这一点值得
    // 知道。数字打印在上面，并记入 `docs/performance-baseline.md`。
    assert!(
        one_static_allocations > empty_allocations && one_dynamic_allocations > empty_allocations,
        "one cut must cost more than no cut on both paths"
    );
    // Like for like, the static spelling is the cheaper one, so the claim holds
    // for the stronger reading too. An earlier version of this comparison used
    // the example's two static cuts against a one-cut dynamic plan and concluded
    // the opposite; the numbers above are what caught that.
    // 同口径下静态写法更便宜，因此这条断言在更强的读法下也成立。本对比的早先版本拿示例的两个
    // 静态切口去对一个切口的动态计划，得出了相反结论；是上面的数字抓出了这一点。
    assert!(
        one_static_allocations < one_dynamic_allocations && one_static_bytes < one_dynamic_bytes,
        "one cut: static {one_static_allocations}/{one_static_bytes} against dynamic \
         {one_dynamic_allocations}/{one_dynamic_bytes}"
    );
    assert!(
        two_static_allocations < two_dynamic_allocations && two_static_bytes < two_dynamic_bytes,
        "two cuts: static {two_static_allocations}/{two_static_bytes} against dynamic \
         {two_dynamic_allocations}/{two_dynamic_bytes}"
    );
    println!(
        "per cut: static {}/{}, dynamic {}/{}",
        one_static_allocations - empty_allocations,
        one_static_bytes - empty_bytes,
        one_dynamic_allocations - empty_allocations,
        one_dynamic_bytes - empty_bytes
    );
}

/// The kind at one logical path, for comparing two effective trees.
/// 某个逻辑路径上的 kind，用于比较两棵有效树。
fn kind_at(registry: &Registry) -> String {
    registry
        .depth_first()
        .iter()
        .find(|info| registry.path_for(info.id).as_deref() == Some("root/control/button"))
        .map(|info| info.kind.clone())
        .expect("the example always has this face")
}
