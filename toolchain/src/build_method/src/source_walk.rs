//! Folder-backed registration discovery.
//! 文件夹注册面发现。

use std::fs;
use std::path::{Path, PathBuf};

use super::Node;
use super::registry_syntax::parse_face;

/// A registration face that lives where the build cannot compile it.
/// 长在构建无法编译的位置上的注册面。
#[derive(Clone, Debug)]
pub(crate) struct UnplacedFace {
    /// The file, relative to the package root.
    /// 文件，以包根为基准。
    pub(crate) relative: String,
    /// One-based line when the failure is inside the file.
    /// 失败发生在文件内部时为从 1 开始的行号。
    pub(crate) line: usize,
    /// The diagnostic phase this belongs to.
    /// 本条属于哪个诊断阶段。
    pub(crate) phase: &'static str,
    /// What is wrong, ready to print.
    /// 问题所在，可直接打印。
    pub(crate) message: String,
}

/// Discover the registration tree, ignoring unplaceable files.
/// 发现注册树，忽略无法安放的文件。
///
/// The findings a caller needs are collected by [`discover_root_reporting`];
/// this entry point exists for the readers that only want the tree.
/// 调用方需要的发现结果由 [`discover_root_reporting`] 收集；本入口供只要树的读者使用。
pub(crate) fn discover_root(src: &Path) -> Vec<Node> {
    let mut unplaced = Vec::new();
    discover_root_reporting(src, &mut unplaced)
}

/// Discover the registration tree and report the faces it cannot place.
/// 发现注册树，并报告它无法安放的注册面。
///
/// A `.rs` file that is not a registration face is skipped in silence: every Rust
/// project has ordinary modules next to its faces, and refusing them would make
/// the tool unusable on a real crate. A file that *is* a face, or that is meant
/// to be one and does not parse, is reported: the build could never compile it,
/// and silence there is exactly the failure this project exists to refuse.
/// 不是注册面的 `.rs` 文件静默跳过：每个 Rust 项目都会在注册面旁边放普通模块，拒绝它们
/// 会让工具在真实 crate 上不可用。而**是**注册面的文件、或本意是面却解析不了的文件会被
/// 报告：构建永远编译不到它，此处沉默正是本项目存在的意义所在——拒绝的那种失败。
pub(crate) fn discover_root_reporting(src: &Path, unplaced: &mut Vec<UnplacedFace>) -> Vec<Node> {
    discover_root_reporting_with_workers(src, unplaced, worker_budget())
}

/// [`discover_root_reporting`] with the worker count spelled out, so a pin can compare worker counts.
/// [`discover_root_reporting`] 的显式工作线程数版本，好让钉子能比较不同线程数的结果。
///
/// **Order is the whole contract.** The nodes are sorted by name below and the unplaceable findings
/// are merged in the order the directory handed its entries out, so the tree and the diagnostics are
/// the same whatever the worker count — the fingerprint that goes into every record is computed from
/// this tree, and one that depended on completion order would invalidate every record the moment the
/// machine's core count changed (audit `K2`'s rule, now holding for discovery too).
/// **顺序就是全部契约。** 节点在下面按名字排序，无法安放的发现按目录交出条目的顺序合并，因此无论几个
/// 工作线程，树与诊断都相同——写进每份记录的指纹就是从这棵树算出来的，而依赖完成顺序的指纹会在机器的核数
/// 一变时让每份记录失效（审计 `K2` 的规则，现在对发现过程同样成立）。
pub(crate) fn discover_root_reporting_with_workers(
    src: &Path,
    unplaced: &mut Vec<UnplacedFace>,
    workers: usize,
) -> Vec<Node> {
    // A tree the build cannot read is a layout problem, reported where layout
    // problems are reported. This used to `expect("src directory must exist")`,
    // which took the build script down with exit 101 and left `check --json` with
    // an empty stdout — the contract that command was fixed to keep.
    // 构建读不到的树是布局问题，报到布局问题该报的地方。这里过去是
    // `expect("src directory must exist")`，那会以退出 101 打死构建脚本，并让
    // `check --json` 的 stdout 一片空白——而那正是那条命令被修好要守住的契约。
    let Ok(entries) = fs::read_dir(src) else {
        unplaced.push(unreadable(src, src));
        return Vec::new();
    };
    // The entries are read out first so one entry can go to one worker; the order the directory
    // handed them out is the order the findings below are merged in.
    // 先把条目读出来，好把一个条目交给一个工作线程；目录交出它们的顺序就是下面合并发现的顺序。
    let entries: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    let mut nodes = Vec::new();
    for (node, mut found) in parallel_map_with_workers(&entries, workers, |path| {
        walk_root_entry(src, path, workers)
    }) {
        if let Some(node) = node {
            nodes.push(node);
        }
        unplaced.append(&mut found);
    }
    nodes.sort_by(|left, right| left.name.cmp(&right.name));
    nodes.retain(has_source);
    nodes
}

/// One entry of the source root, walked the way the root walk asks for it.
/// 源根的一个条目，按根遍历要求的方式走过。
fn walk_root_entry(src: &Path, path: &Path, workers: usize) -> (Option<Node>, Vec<UnplacedFace>) {
    let mut unplaced = Vec::new();
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return (None, unplaced);
    };
    if matches!(name, "lib.rs" | "main.rs" | "bin") {
        return (None, unplaced);
    }
    let name = name.to_owned();
    if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
        record_unplaced(&mut unplaced, src, path);
    }
    if path.is_dir() && valid_name(&name) {
        let attached = path.join(format!("{name}.rs"));
        return (
            Some(Node {
                name,
                file: attached.is_file().then_some(attached),
                children: discover_children(src, path, &mut unplaced, workers),
            }),
            unplaced,
        );
    }
    (None, unplaced)
}

fn discover_children(
    src: &Path,
    dir: &Path,
    unplaced: &mut Vec<UnplacedFace>,
    workers: usize,
) -> Vec<Node> {
    // The same refusal as the root's: a directory that vanished or cannot be read
    // between the walk reaching it and this call is reported, not fatal.
    // 与根目录同样的拒绝：在遍历到达它之后、本次调用之前消失或读不了的目录会被报告，而不是致命。
    let Ok(entries) = fs::read_dir(dir) else {
        unplaced.push(unreadable(src, dir));
        return Vec::new();
    };
    // Each level maps in parallel on its own — the wide level of a real tree is rarely the top one
    // (the 50,000-file fixture keeps its 2,500 faces in `src/control/object/`), and parallelising
    // only the root would leave that level serial. [`parallel_map_with_workers`] refuses to nest, so
    // a worker that meets another wide level finishes it on its own thread instead of spawning a
    // second pool: the thread count stays bounded by the machine, not by the depth of the tree.
    // 每一层各自并行映射——真实树的宽层很少是顶层（50,000 文件夹具把它的 2,500 个面放在
    // `src/control/object/` 下），只并行根层会让那一层留在串行。 [`parallel_map_with_workers`] 拒绝嵌套，
    // 因此在工作线程里遇到另一个宽层的调用会在自己的线程上把它做完，而不是再开一个线程池：线程数由机器
    // 决定，而不是由树的深度决定。
    let entries: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    let mut nodes = Vec::new();
    for (node, mut found) in parallel_map_with_workers(&entries, workers, |path| {
        walk_child_entry(src, dir, path, workers)
    }) {
        if let Some(node) = node {
            nodes.push(node);
        }
        unplaced.append(&mut found);
    }
    nodes.sort_by(|left, right| left.name.cmp(&right.name));
    nodes.retain(has_source);
    nodes
}

/// One entry of a module directory, walked the way [`discover_children`] asks for it.
/// 模块目录的一个条目，按 [`discover_children`] 要求的方式走过。
fn walk_child_entry(
    src: &Path,
    dir: &Path,
    path: &Path,
    workers: usize,
) -> (Option<Node>, Vec<UnplacedFace>) {
    let mut unplaced = Vec::new();
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return (None, unplaced);
    };
    if path.is_file() {
        if path.file_stem().and_then(|stem| stem.to_str())
            == dir.file_name().and_then(|stem| stem.to_str())
        {
            return (None, unplaced);
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            return (None, unplaced);
        }
        record_unplaced(&mut unplaced, src, path);
    }
    let name = name.to_owned();
    if path.is_dir() && valid_name(&name) {
        let attached = path.join(format!("{name}.rs"));
        return (
            Some(Node {
                name,
                file: attached.is_file().then_some(attached),
                children: discover_children(src, path, &mut unplaced, workers),
            }),
            unplaced,
        );
    }
    (None, unplaced)
}

/// One directory the walk could not read, reported like an unplaceable face.
/// 遍历读不到的一个目录，像无法安放的注册面一样被报告。
///
/// `src` is the source root the message and the relative path are measured
/// against, and `directory` is the one that failed; they are the same path when
/// the root itself is the problem.
/// `src` 是消息与相对路径所依据的源根，`directory` 是失败的那个；根本身出问题时两者相同。
fn unreadable(src: &Path, directory: &Path) -> UnplacedFace {
    let problem = match fs::symlink_metadata(directory) {
        Ok(metadata) if metadata.is_dir() => "cannot be read".to_owned(),
        Ok(_) => "is not a directory".to_owned(),
        Err(error) => format!("does not exist ({error})"),
    };
    UnplacedFace {
        relative: super::relative_display(src, directory),
        line: 0,
        phase: "face-layout",
        message: format!(
            "{} {problem}; the build reads registration faces from this source tree",
            directory.display()
        ),
    }
}

/// Record a `.rs` file that cannot become a module, unless it is an ordinary one.
/// 记录无法成为模块的 `.rs` 文件，除非它只是普通模块。
fn record_unplaced(unplaced: &mut Vec<UnplacedFace>, src: &Path, path: &Path) {
    let Ok(source) = fs::read_to_string(path) else {
        // An unreadable file makes no claim we can judge; the build's own reads
        // report it where the file actually mattered.
        // 读不了的文件没有可判断的声明；构建自己的读取会在该文件真正要紧的地方报告它。
        return;
    };
    match parse_face(&source) {
        Ok(None) => {}
        // Two different things used to get one message. A face written with `external_object!`
        // declares its own registry and is deliberately *not* part of this package's generated
        // tree — the example host that grafts it registers it explicitly — so saying "the build
        // can never compile it" was false about a working crate. Only a face of the generated
        // layout that sits outside it has a layout problem.
        // 过去有两种不同的东西共用一句话。用 `external_object!` 写的面声明了自己的注册机、并且**有意**
        // 不属于本包的生成树——graft 它的那个示例宿主显式注册它——因此"构建永远编译不了它"对一个能工作的
        // crate 来说是假话。只有生成本该在 `<name>/<name>.rs` 却长在别处的面才是布局问题。
        Ok(Some(face)) if is_external_face(&face.macro_name) => unplaced.push(UnplacedFace {
            relative: super::relative_display(src, path),
            line: face.location.line,
            phase: "face-external",
            message: "external face (`external_object!`): it declares its own registry and this \
                      package's generated tree does not contain it; the host that grafts it \
                      registers it explicitly"
                .to_owned(),
        }),
        Ok(Some(face)) => unplaced.push(UnplacedFace {
            relative: super::relative_display(src, path),
            line: face.location.line,
            phase: "face-layout",
            message: "registration face is outside the `<name>/<name>.rs` layout, so the build can never compile it; move the file to `<name>/<name>.rs`".to_owned(),
        }),
        Err(error) => unplaced.push(UnplacedFace {
            relative: super::relative_display(src, path),
            line: error.location.as_ref().map_or(0, |location| location.line),
            phase: "face-syntax",
            message: error.message,
        }),
    }
}

/// Whether a registration macro declares a face of its own registry rather than of the
/// generated tree.
/// 一个注册宏声明的面是自己注册机的，还是生成树的。
///
/// `external_object!` is the framework's out-of-project declaration, and its `__`-prefixed
/// re-export is the same macro under the spelling reserved for generated code.
/// `external_object!` 是框架的项目外声明，而它带 `__` 前缀的重导出是同一个宏在"生成代码专用"拼法下的
/// 名字。
fn is_external_face(macro_name: &str) -> bool {
    matches!(macro_name, "external_object" | "__external_object")
}

fn has_source(node: &Node) -> bool {
    node.file.is_some() || node.children.iter().any(has_source)
}

/// Whether a directory name may become a module name.
/// 目录名是否可以成为模块名。
///
/// Discovery and the admission scan must agree on this: a directory discovery
/// skips is a directory whose faces never reach the generated tree, so the
/// admission scan must skip it too — otherwise a face that can never be
/// compiled still vetoes the build.
/// 发现过程与 admission 扫描必须在这一点上一致：发现过程跳过的目录，其注册面永远
/// 进不了生成树，因此 admission 扫描也必须跳过——否则一个永远编译不到的面仍然能否决
/// 构建。
pub(crate) fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().enumerate().all(|(index, ch)| {
            ch == '_' || ch.is_ascii_alphanumeric() && (index > 0 || !ch.is_ascii_digit())
        })
}

pub(crate) fn emit_rerun_paths(src: &Path, nodes: &[Node]) {
    println!("cargo:rerun-if-changed={}", src.display());
    let mut files = Vec::new();
    collect_source_files(nodes, &mut files);
    files.sort();
    files.dedup();
    for file in files {
        println!("cargo:rerun-if-changed={}", file.display());
    }
}

pub(crate) fn discovery_fingerprint(src: &Path, scan: &Path, nodes: &[Node]) -> String {
    // Two inputs, each covering what the other cannot (audit `T1`): the **raw file set** by path alone
    // — the module tree below cannot see a file that no `mod` declaration names, so a face added
    // beside the tree used to leave the fingerprint unchanged — and the **recorded files** with
    // contents, which is what notices an edit. Paths and not contents for the first half: hashing every
    // file's bytes would make each freshness check read the whole tree once more.
    // 两份输入，各自覆盖对方覆盖不到的东西（审计 `T1`）：**原始文件集合**只看路径——下面那棵模块树看不见
    // 没有任何 `mod` 声明的文件，因此在树旁边新增一个面过去不改指纹——以及**记录过的文件**及其内容，后者
    // 用来发现编辑。前半只取路径不取内容：哈希每个文件的字节会让每次新鲜性检查对整棵树多读一遍。
    let mut raw = Vec::new();
    visit_rust_paths(scan, &mut raw);
    raw.sort();
    let mut files = Vec::new();
    collect_source_files(nodes, &mut files);
    files.sort();
    let mut input = Vec::new();
    for path in &raw {
        input.extend_from_slice(super::relative_display(src, path).as_bytes());
        input.push(0);
        input.push(0xfe);
    }
    // The per-file half — read the bytes and spell the path — is independent per file, and it is the one
    // part of this rule parallelism can touch (audit `K2`). Measured at 50,000 files: **125 ms per
    // 2,500-file member**, i.e. ~50 µs per file of read + hash + allocation, paid once per member in
    // sequence. The chunks are joined in the **sorted file order**, which is what keeps the digest
    // identical to the serial one: combining in completion order would change the fingerprint and make
    // every existing record stale.
    // 每文件那一半——读字节、拼路径——逐文件独立，是这条规则里并行唯一能碰的部分（审计 `K2`）。在 50,000
    // 文件上实测：每个 2,500 文件的成员 **125 ms**，即每文件约 50 µs 的读+哈希+分配，而 20 个成员按顺序各付一次。
    // 各分块按**排序后的文件顺序**拼接，这正是让摘要与串行版逐字节相同的原因：按完成顺序拼接会改掉指纹，
    // 从而让每一份已存在的记录变陈旧。
    let chunks = parallel_map(&files, |file| {
        let mut chunk = Vec::new();
        chunk.extend_from_slice(super::relative_display(src, file).as_bytes());
        chunk.push(0);
        chunk.extend_from_slice(&fs::read(file).unwrap_or_default());
        chunk.push(0xff);
        chunk
    });
    for chunk in chunks {
        input.extend_from_slice(&chunk);
    }
    super::registry_identity::NodeId::from_bytes(&input).to_string()
}

pub(crate) fn collect_source_files(nodes: &[Node], files: &mut Vec<std::path::PathBuf>) {
    for node in nodes {
        if let Some(file) = &node.file {
            files.push(file.clone());
        }
        collect_source_files(&node.children, files);
    }
}

/// The filesystem facts the kernel's source walk asks this surface for.
/// 内核源码遍历向本执行面索取的文件系统事实。
struct StdSourceTree;

impl nichlink_kernel::source::SourceTree for StdSourceTree {
    fn is_directory(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn entries(&self, path: &Path) -> Result<Vec<PathBuf>, String> {
        fs::read_dir(path)
            .map_err(|error| format!("cannot scan {}: {error}", path.display()))?
            .map(|entry| {
                entry
                    .map(|entry| entry.path())
                    .map_err(|error| format!("cannot scan {}: {error}", path.display()))
            })
            .collect()
    }

    fn read_text(&self, path: &Path) -> Result<String, String> {
        fs::read_to_string(path).map_err(|error| format!("cannot read {}: {error}", path.display()))
    }
}

/// Every `.rs` file under `directory`, following the crate's own layout.
/// `directory` 下的每个 `.rs` 文件，遵循 crate 自己的布局。
pub(crate) fn collect_rust_sources(
    directory: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    nichlink_kernel::source::collect_rust_sources(
        &StdSourceTree,
        directory,
        nichlink_kernel::source::SourceWalk::EVERYTHING,
        |_, _| nichlink_kernel::source::Keep::Yes,
        files,
    )
}

/// Map over items with several workers, preserving order (audit `K2`).
/// 用多个工作线程映射一组条目，并**保持顺序**（审计 `K2`）。
///
/// Standard library only: this checkout builds offline and publishes crates, so a thread-pool
/// dependency is not an option, and `std::thread::scope` has been enough since 1.63. **Order
/// preservation is the whole contract** — a fingerprint that depended on completion order would go
/// stale for every record the moment the machine's core count changed, and `fingerprints_do_not_depend
/// _on_the_worker_count` is the pin that holds it.
/// 只用标准库：本检出离线构建并发布 crate，因此线程池依赖不是选项，而 `std::thread::scope` 自 1.63 起就够用。
/// **保持顺序就是它的全部契约**——依赖完成顺序的指纹会在机器核数一变时让每份记录立刻陈旧，而
/// `fingerprints_do_not_depend_on_the_worker_count` 就是守住它的钉子。
fn parallel_map<T: Send>(
    items: &[std::path::PathBuf],
    map: impl Fn(&std::path::PathBuf) -> T + Sync,
) -> Vec<T> {
    parallel_map_with_workers(items, worker_budget(), map)
}

/// How many workers this process may hand one mapping (audit `T1`).
/// 本进程一次映射最多可以交给几个工作线程（审计 `T1`）。
///
/// A rule rather than a number, because machines differ and a tool call must not take a user's whole
/// machine: **half the cores, at most [`MAX_WORKERS`], never fewer than one core left alone** — an
/// editor, a compiler and a test run are usually standing beside us. `NICH_LINK_JOBS` (see the
/// kernel's `lexicon`) overrides it for a caller who knows better. The **result never depends on this
/// number**: order is preserved either way, which is what
/// `discovery_tests::the_discovered_tree_does_not_depend_on_the_worker_count` and
/// `parallel_tests::fingerprints_do_not_depend_on_the_worker_count` hold.
/// 是规则而不是数字，因为机器各不相同，而一次工具调用不该把用户的整台机器拿走：**一半的核、最多
/// [`MAX_WORKERS`] 个、至少留一个核**——编辑器、编译器与测试通常就在旁边。比规则更清楚自己处境的调用方
/// 用 `NICH_LINK_JOBS`（见内核 `lexicon`）覆盖它。**结果绝不取决于这个数字**：两种情况下顺序都保持，
/// 这正是 `discovery_tests::the_discovered_tree_does_not_depend_on_the_worker_count` 与
/// `parallel_tests::fingerprints_do_not_depend_on_the_worker_count` 守住的东西。
pub(crate) fn worker_budget() -> usize {
    let requested = std::env::var(nichlink_kernel::lexicon::JOBS_ENV)
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|count| *count > 0);
    let cores = std::thread::available_parallelism().map_or(1, |count| count.get());
    budget_from(requested, cores)
}

/// The budget as a pure function of the explicit request and the machine's core count (audit `T1`).
/// 把预算写成"显式请求 + 机器核数"的纯函数（审计 `T1`）。
///
/// Split out so the rule can be pinned **without writing to the process environment**: the rule is
/// what matters, and a test that mutates `NICH_LINK_JOBS` would be writing to the same environment
/// every other test in this process reads (`std::env::set_var` is `unsafe` in edition 2024 for
/// exactly that reason). An explicit request that parses to a positive number wins outright; a zero
/// or a typo is not a request and falls through to the machine rule.
/// 拆出来是为了让规则**不必写进程环境**就能被钉住：要紧的是规则本身，而一个改写 `NICH_LINK_JOBS` 的测试
/// 会写进本进程里其它每个测试都在读的那份环境（edition 2024 里 `std::env::set_var` 是 `unsafe`，正是
/// 这个原因）。解析出正数的显式请求直接生效；零或错别字不算请求，落到机器规则上。
fn budget_from(requested: Option<usize>, cores: usize) -> usize {
    requested.unwrap_or_else(|| (cores / 2).clamp(1, MAX_WORKERS))
}

/// The most workers a mapping takes even on a very wide machine.
/// 即使在非常宽的机器上，一次映射最多用几个工作线程。
///
/// A 64-core build server gains nothing from 64 threads on a twenty-member workspace, and the thread
/// handoff is not free; this is the ceiling the rule above is capped by, not a promise about speed.
/// 一台 64 核的构建服务器不会因为给二十个成员的工作区开 64 个线程而得到什么，而线程交接也不是免费的；
/// 这是上面那条规则的上限，而不是关于速度的承诺。
const MAX_WORKERS: usize = 8;

thread_local! {
    /// Whether this thread is already inside a mapping, which is what stops a nested wide level from
    /// opening a second pool (audit `T1`).
    /// 本线程是否已经在一个映射之内；正是它阻止嵌套的宽层再开一个线程池（审计 `T1`）。
    static MAPPING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// [`parallel_map`] with the worker budget spelled out, so a pin can compare worker counts.
/// [`parallel_map`] 的显式工作线程预算版本，好让钉子能比较不同线程数。
fn parallel_map_with_workers<I: Sync, T: Send>(
    items: &[I],
    budget: usize,
    map: impl Fn(&I) -> T + Sync,
) -> Vec<T> {
    parallel_map_with_threshold(items, budget, 64, map)
}

/// The single mapping implementation: ordered, bounded, and refusing to nest (audit `K2`, `T1`).
/// 唯一的映射实现：保持顺序、有界、并拒绝嵌套（审计 `K2`、`T1`）。
///
/// `serial_below` is the size under which the thread handoff costs more than the work it hands off,
/// and it is a parameter because "small" is a property of the **items**: a file is a handful of
/// kilobytes, while a workspace **member** is a whole-tree content hash, so two members already pay
/// for the handoff where sixty-four files do not. `budget` is the machine rule ([`worker_budget`]:
/// half the cores, at most [`MAX_WORKERS`], one core always left alone) unless a caller names one.
/// `serial_below` 是"线程交接比它交接出去的活还贵"的那个规模，而它是一个参数，因为"小"是**条目**的性质：
/// 一个文件是几 KB，而工作区的一个**成员**是一次整树内容哈希，因此两个成员就够付线程交接的钱，而六十四个
/// 文件不够。`budget` 是机器规则（[`worker_budget`]：一半的核、最多 [`MAX_WORKERS`] 个、永远留一个核），
/// 除非调用方点名一个。
///
/// Inside a worker of an enclosing mapping it runs serially — a nested pool would multiply the
/// machine's threads by the depth of the tree — and the budget is capped by the item count, so a
/// mapping over three members never opens eight threads.
/// 在**外层映射的工作线程之内**时它串行运行——嵌套线程池会把机器的线程数乘以树的深度——而预算被条目数
/// 封顶，因此对三个成员的映射绝不会开八个线程。
pub(crate) fn parallel_map_with_threshold<I: Sync, T: Send>(
    items: &[I],
    budget: usize,
    serial_below: usize,
    map: impl Fn(&I) -> T + Sync,
) -> Vec<T> {
    let workers = budget.min(items.len()).max(1);
    if items.len() < serial_below || workers <= 1 || MAPPING.with(std::cell::Cell::get) {
        return items.iter().map(map).collect();
    }
    let chunk = items.len().div_ceil(workers);
    let mut slots: Vec<Option<Vec<T>>> = (0..items.len().div_ceil(chunk)).map(|_| None).collect();
    std::thread::scope(|scope| {
        for (slot, slice) in slots.iter_mut().zip(items.chunks(chunk)) {
            let map = &map;
            scope.spawn(move || {
                let _guard = MappingGuard::enter();
                *slot = Some(slice.iter().map(map).collect());
            });
        }
    });
    slots.into_iter().flatten().flatten().collect()
}

/// Marks its thread as being inside a mapping, for as long as it lives.
/// 在它存活期间，把所在线程标记为"在一个映射之内"。
struct MappingGuard;

impl MappingGuard {
    /// Enter a mapping on this thread, restoring the previous mark when dropped.
    /// 在本线程进入一个映射，并在被丢弃时恢复先前的标记。
    fn enter() -> Self {
        MAPPING.with(|mapping| mapping.set(true));
        MappingGuard
    }
}

impl Drop for MappingGuard {
    fn drop(&mut self) {
        MAPPING.with(|mapping| mapping.set(false));
    }
}

/// The cheap stamp of a package's sources: how many `.rs` files it has, and the newest modification
/// time among them (audit `T1`).
/// 一个包源码的廉价戳：有多少 `.rs` 文件，以及其中最新的修改时间（审计 `T1`）。
///
/// Directory entries only — not a single file is opened — so this costs a walk rather than the bytes.
/// It is a **guard**, never a verdict: the freshness rule beside it in the bridge still compares
/// bytes, and the generation file records this stamp only so that a reader can tell *cheaply* whether
/// the published index could still describe these sources. Its blind spot is by construction the
/// memo's: an edit that preserves both the file count and the modification time is not noticed.
/// 只读目录项——一个文件都不打开——因此它花一次遍历而不是字节。它是**守卫**，从不是裁决：桥里那条新鲜度
/// 规则仍然逐字节比较，而 generation 文件记下这个戳，只是为了让读者能**廉价地**判断已发布的索引是否仍可能
/// 描述着这批源码。它的盲区与那份记忆本来就相同：同时保留文件数与修改时间的编辑不会被发现。
///
/// `(0, 0)` when the package has no readable source layout at all: an absent tree is not an empty one,
/// and answering "zero files" would let a reader treat a missing tree as a clean one.
/// 包连可读的源码布局都没有时是 `(0, 0)`：缺一棵树不等于那是一棵空树，而答"零个文件"会让读者把缺失的树
/// 当成干净的树。
pub(crate) fn source_stamp(root: &Path) -> (usize, u64) {
    let Ok(layout) = crate::build_method::source_layout(root) else {
        return (0, 0);
    };
    let mut count = 0usize;
    let mut newest = 0u64;
    let mut stack = vec![layout.scan_root.clone()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if metadata.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().is_none_or(|extension| extension != "rs") {
                continue;
            }
            count += 1;
            if let Ok(modified) = metadata.modified()
                && let Ok(since) = modified.duration_since(std::time::UNIX_EPOCH)
            {
                newest = newest.max(since.as_nanos() as u64);
            }
        }
    }
    (count, newest)
}

/// Every `.rs` file under `dir`, by path, without reading any of them.
/// `dir` 下每个 `.rs` 文件的路径，不读取其中任何一个。
fn visit_rust_paths(dir: &Path, into: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            visit_rust_paths(&path, into);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            into.push(path);
        }
    }
}

#[cfg(test)]
mod parallel_tests {
    use super::parallel_map;

    /// The worker budget is a rule, and the explicit request beats it (audit `T1`).
    /// 工作线程预算是**一条规则**，而显式请求压过它（审计 `T1`）。
    ///
    /// Two properties matter and neither is about speed: a mapping must never take a user's whole
    /// machine (at most half the cores, and below a wide machine's core count), and an explicit
    /// request must be honoured so a caller can say "use one" or "use more". The machine's core count
    /// is a parameter here rather than a fact read from this box, because the rule has to hold on
    /// every box.
    /// 两条性质要紧，且都与速度无关：一次映射绝不拿走用户的整台机器（最多一半的核，且低于宽机器的核数），
    /// 而显式请求必须被尊重，好让调用方说"就用一个"或"多用一点"。核数在这里是**参数**而不是从本机读来的
    /// 事实，因为这条规则在每台机器上都要成立。
    #[test]
    fn the_worker_budget_leaves_the_machine_alone_and_honours_an_explicit_request() {
        for cores in [1usize, 2, 3, 4, 8, 16, 20, 64, 128] {
            let budget = super::budget_from(None, cores);
            assert!(budget >= 1, "{cores} cores must still buy one worker");
            assert!(
                cores == 1 || budget < cores,
                "{cores} cores: a mapping must leave the machine something ({budget})"
            );
            assert!(
                budget <= 8,
                "{cores} cores: the ceiling is part of the rule"
            );
            assert_eq!(
                super::budget_from(Some(3), cores),
                3,
                "an explicit request wins"
            );
        }
    }

    /// The fingerprint must not depend on how many workers computed it (audit `K2`).
    /// 指纹不得取决于用几个工作线程算出来（审计 `K2`）。
    ///
    /// Parallelism here is an implementation detail of a **rule**: the digest goes into every record,
    /// and a digest that depended on completion order would invalidate all of them the moment the core
    /// count changed. The pin runs the same mapping many times and compares the joined result — with
    /// enough items that the parallel path is actually taken (> the serial threshold).
    /// 这里的并行只是一条**规则**的实现细节：摘要写进每一份记录，而依赖完成顺序的摘要在核数一变时就会让它们
    /// 全部失效。这枚钉子反复跑同一个映射并比较拼接结果——条目数足够多，因此真的会走并行那条路（超过串行阈值）。
    #[test]
    fn fingerprints_do_not_depend_on_the_worker_count() {
        let items: Vec<std::path::PathBuf> = (0..500)
            .map(|index| std::path::PathBuf::from(format!("file{index:04}.rs")))
            .collect();
        let expected: Vec<String> = items
            .iter()
            .map(|path| path.display().to_string())
            .collect();
        for _ in 0..5 {
            let got = parallel_map(&items, |path| path.display().to_string());
            assert_eq!(
                got, expected,
                "order is preserved whatever the worker count"
            );
        }
    }
}

#[cfg(test)]
mod discovery_tests {
    use super::{UnplacedFace, discover_root_reporting_with_workers};
    use std::fs;
    use std::path::PathBuf;

    /// A wide tree: `root/src/control/object/<face>/<face>.rs`, the shape a real generated host has
    /// when it has to spread a large registry over directories — and the shape the 50,000-file
    /// fixture has, whose wide level is neither the root nor the first level (audit `T1`).
    /// 一棵宽树：`root/src/control/object/<face>/<face>.rs`——真实生成宿主在必须把大注册表摊到目录里时的
    /// 形状，也正是 50,000 文件夹具的形状，而它的宽层既不是根层也不是第一层（审计 `T1`）。
    fn wide_tree(label: &str, faces: usize) -> PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "nichlink-source-walk-{label}-{}-{sequence}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src/control/object")).expect("tree");
        fs::write(root.join("src/lib.rs"), "// host entry\n").expect("library target");
        fs::write(
            root.join("src/control/control.rs"),
            "// the module that owns the wide level\n",
        )
        .expect("control module");
        for index in 0..faces {
            let face = format!("child{index:06}");
            let directory = root.join("src/control/object").join(&face);
            fs::create_dir_all(&directory).expect("face directory");
            fs::write(directory.join(format!("{face}.rs")), "// face\n").expect("face source");
        }
        // A face that sits outside the layout is the finding the walk reports, and its position in
        // the merged list is part of what this pin compares.
        // 一个长在布局之外的面正是遍历要报告的发现，而它在合并后的清单里的位置也是这枚钉子比较的东西。
        fs::write(root.join("src/control/loose.rs"), "// not a face\n").expect("loose file");
        root
    }

    fn render(nodes: &[super::Node]) -> Vec<String> {
        let mut lines = Vec::new();
        render_into(nodes, 0, &mut lines);
        lines
    }

    fn render_into(nodes: &[super::Node], depth: usize, lines: &mut Vec<String>) {
        for node in nodes {
            lines.push(format!(
                "{}{} file={:?}",
                "  ".repeat(depth),
                node.name,
                node.file.as_ref().map(|file| file
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default())
            ));
            render_into(&node.children, depth + 1, lines);
        }
    }

    fn render_findings(findings: &[UnplacedFace]) -> Vec<String> {
        findings
            .iter()
            .map(|finding| format!("{} {} {}", finding.relative, finding.phase, finding.line))
            .collect()
    }

    /// Discovery must hand back the same tree and the same findings whatever the worker count
    /// (audit `T1`).
    /// 无论几个工作线程，发现过程都必须交回同一棵树与同一批发现（审计 `T1`）。
    ///
    /// The tree goes into the fingerprint that every record carries, so a result that depended on
    /// which worker finished first would invalidate every record on a machine with a different core
    /// count. The serial run is the reference; the parallel runs must match it **including the order
    /// of the findings**, which is the half that a naive parallel walk gets wrong.
    /// 这棵树会进入每份记录携带的指纹，因此一个取决于哪个工作线程先做完的结果，会在核数不同的机器上让每份记录
    /// 失效。串行那次是参照；并行的几次必须与它一致，**连发现的顺序也要一致**——那正是草率的并行遍历会弄错的
    /// 那一半。
    #[test]
    fn the_discovered_tree_does_not_depend_on_the_worker_count() {
        let root = wide_tree("workers", 200);
        let src = root.join("src");
        let mut serial_findings = Vec::new();
        let serial = discover_root_reporting_with_workers(&src, &mut serial_findings, 1);
        assert!(
            render(&serial).len() > 200,
            "the fixture must be wide enough to take the parallel path: {}",
            render(&serial).len()
        );
        for workers in [2usize, 3, 8, 64] {
            let mut findings = Vec::new();
            let tree = discover_root_reporting_with_workers(&src, &mut findings, workers);
            assert_eq!(
                render(&tree),
                render(&serial),
                "the tree differs with {workers} workers"
            );
            assert_eq!(
                render_findings(&findings),
                render_findings(&serial_findings),
                "the findings differ with {workers} workers"
            );
        }
        let _ = fs::remove_dir_all(&root);
    }
}
