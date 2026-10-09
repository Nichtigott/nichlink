//! Text contracts shared by every NichLink surface.
//! 每个 NichLink 执行面共享的文本契约。
//!
//! These strings are contracts, not settings. The build step writes the file
//! the host crate includes, generated code addresses the runtime crate by
//! name, the face front end recognises the field spellings the kernel
//! vocabulary lists, the first-pass scope never prunes the modules that carry
//! the registration machinery, and an external graft plan has exactly one
//! location. Each of them used to be spelled out again at every use site, so a
//! host that renamed a dependency, or an author who wrote a manifest the naive
//! reader could not follow, could disagree with another crate while every
//! compiler stayed quiet.
//! 这些字符串是契约而不是配置。构建步骤写下的文件名正是宿主 crate 要 include 的
//! 那个；生成代码按名字寻址运行期 crate；宏前端识别的字段拼写与内核词表一致；
//! 第一次源码范围修剪永不剪掉承载注册机制的模块；外部 graft 计划的位置只有一处。
//! 它们过去在每个使用点各写一遍，于是重命名依赖的宿主、或写出手写读取器跟不上的
//! manifest 的作者，可能与另一个 crate 产生分歧，而所有编译器都保持沉默。
//!
//! Vocabulary, so one concept has one name to search for (audit `KRN-C-11`): a declaration
//! is a **face**, the thing it becomes in a tree is a **node**, and the last component of a
//! node's path — what a graft cut or a plugin replacement addresses — is its
//! **`registry_name`**. The earlier spellings are aliases, kept here only so a search finds
//! them: *slot*, *slot name*, *branch handle*, *tree slot*, *selector* all meant that path
//! component or the cut that names it, and *object* / *entry* meant the node. New text says
//! face, node, or `registry_name`; the fields keep their published names.
//! 词汇表，使一个概念只有一个可搜索的名字（审计 `KRN-C-11`）：**声明**叫 face，它在树里成为的
//! 东西叫 node，节点路径的最后一段——graft 切口或插件替换所寻址的东西——叫 **`registry_name`**。
//! 早先的拼法只是别名，列在这里仅为让搜索找得到：*slot*、*slot name*、*branch handle*、
//! *tree slot*、*selector* 都指那段路径或命名它的切口，*object* / *entry* 都指节点。新写的文本用
//! face、node 或 `registry_name`；字段保留已发布的旧名。
//!
//! The verb a name starts with says what the function does with its input, so a reader can
//! pick the right call without opening a signature (audit `NAM-33`; the table is the one
//! `docs/audit-2026-09-28/audit-naming-review.md` settled on): `read_*` reads text or bytes
//! from a reader or a file; `load_*` turns a file or artifact on disk into a structure and
//! may fail; `resolve_*` maps an id, path or name to an entity; `find_*` looks something up
//! in a collection and returns a reference or a set — **`get` is not used when the return is
//! not an `Option`**; `collect_*` accumulates into a caller-provided collection and belongs
//! on *public* functions that return that set (a recursive helper behind one is `visit_*` or
//! `walk_*`); `parse_*` turns text into a structure; `render_*` turns a structure back into
//! text; `write_*` writes to disk; `validate_*` checks and returns the reason as an error;
//! `check_*` is for runtime assertions (`RuntimeCheckSpec`). A bare verb is allowed only at
//! an entry position — a crate root, a CLI command's same-named implementation, a trait
//! declaration, or `main` (audit `NAM-32`).
//! 名字开头的动词说明它拿输入做什么，因此读者不必打开签名就能选对调用（审计 `NAM-33`；下表是
//! `docs/audit-2026-09-28/audit-naming-review.md` 定下的那份）：`read_*` 从 reader 或文件读文本
//! 或字节；`load_*` 把磁盘上的文件或工件装载成结构、可失败；`resolve_*` 把 id/路径/名字映射到
//! 实体；`find_*` 在集合里查找并返回引用或集合——**返回值不是 `Option` 时不用 `get`**；
//! `collect_*` 累积进调用方给的集合，只用在**公开**的"返回集合"函数上（它背后的递归 helper 叫
//! `visit_*` 或 `walk_*`）；`parse_*` 把文本变成结构；`render_*` 把结构变回文本；`write_*` 写盘；
//! `validate_*` 检查并把原因作为错误返回；`check_*` 只用于运行期断言（`RuntimeCheckSpec`）。
//! 裸动词只允许出现在入口位——crate 根、CLI 命令的同名实现、trait 声明、或 `main`（审计 `NAM-32`）。

use std::path::{Path, PathBuf};

/// The generated crate entry the build step writes and the host includes.
/// 构建步骤写盘、宿主 crate include 的生成入口文件名。
pub const GENERATED_LIB_FILE: &str = "generated_lib.rs";

/// The runtime crate's name, as generated code and the face front end address
/// it.
/// 运行期 crate 的名字——生成代码与宏前端这样寻址它。
pub const RUN_METHOD_CRATE: &str = "nichlink_toolchain";

/// The face field that marks replaceable plugin surface.
/// 标记可替换插件面的注册面字段。
pub const FACE_FIELD_PLUGIN: &str = "plugin";

/// The build-time graph the pipeline publishes beside the records (audit `M7`, P1.1).
/// 管线在记录旁边发布的构建期图（审计 `M7`，P1.1）。
pub const GRAPH_FILE: &str = "graph_edges.tsv";

/// The readiness record for the published index (audit `M7`, P2.1).
/// 已发布索引的就绪记录（审计 `M7`，P2.1）。
///
/// One file, written **last** and atomically: a reader that finds it can tell whether the index it
/// describes has finished, and a run that died halfway leaves the previous one rather than a
/// half-written claim. This is the "signal" the maintainer asked for — the terminal line and the
/// readers both come from here rather than from a guess about elapsed time.
/// 一个文件，**最后**写、且原子地写：找到它的读者能判断它所描述的索引是否已经完成，而中途死掉的一次运行
/// 留下的是上一次的记录，而不是半份声称。这就是维护者要的"信号"——终端那一行与读取方都从这里来，而不是
/// 从对耗时的猜测来。
pub const GENERATION_FILE: &str = "graph.generation";

/// The first line of the generation record.
/// generation 记录的第一行。
pub const GENERATION_MARKER: &str = "nichlink-build-index";

/// The name of the file that admits **one writer per tree** at a time.
/// 一次只允许**一棵树一个写者**的那份文件的文件名。
///
/// It sits beside the output directory rather than inside it, so the published artifact set stays
/// exactly the set a reader expects, and two runs with different output directories do not contend.
/// 它住在输出目录**旁边**而不是里面，这样已发布的产物集合保持读者预期的那一份，而两个输出目录不同的
/// 运行不会互相争抢。
pub const PUBLISH_LOCK_FILE: &str = ".publishing.lock";

/// The environment variable that tells a run to render the **cross-crate** half: the facade's plan.
/// 告诉一次运行渲染**跨 crate**那一半（facade 的计划）的环境变量。
///
/// A facade compiles no faces of its own; what it carries is the graft cut table and the contract
/// assertions, with every `crate::<module>` rewritten to the crate that compiles that module. The
/// host cannot carry those once a subtree is cut out — its generated tree would name a module it no
/// longer compiles (audit `M7`, P3.2/§M7.33).
/// facade 自己不编译任何面；它携带的是 graft 切口表与契约断言，而其中每一个 `crate::<模块>` 都被改写成
/// **编译该模块的那个 crate**。子树被切出之后宿主无法携带它们——它的生成树会点名一个自己不再编译的模块
/// （审计 `M7`，P3.2/§M7.33）。
pub const SHAPE_FACADE_ENV: &str = "NICH_LINK_SHAPE_FACADE";

/// The environment variable that tells a run to render **only** one crate's subtrees.
/// 告诉一次运行**只**渲染某一个 crate 的子树的环境变量。
///
/// A ghost crate compiles a fragment the host no longer compiles, and its own build script sets this
/// to the claimed subtrees (`control::object,panel`). Every other node is left out, and the nodes
/// **above** a claimed subtree are emitted as empty container modules: mounting an ancestor's face
/// file would register that face a second time, in a second registry (audit `M7`, P3.2).
/// 幽灵 crate 编译的是宿主不再编译的碎片，而它自己的构建脚本把这里设成认领的子树
/// （`control::object,panel`）。其余节点一律不发射，而认领子树**之上**的节点发成空的容器模块：挂载祖先的
/// 面文件会把那个面第二次注册进第二个注册机（审计 `M7`，P3.2）。
pub const SHAPE_ONLY_ENV: &str = "NICH_LINK_SHAPE_ONLY";

/// The environment variable bounding how long a publish waits for that lock, in milliseconds.
/// 限制一次发布为那把锁等待多久的环境变量，单位毫秒。
pub const LOCK_WAIT_ENV: &str = "NICH_LINK_LOCK_WAIT_MS";

/// The key the generation record stamps the **namespace** it was published under.
/// generation 记录盖下"它发布时所用**命名空间**"的那把钥匙。
///
/// Identity is `hash(namespace, source path, name)` and the namespace comes from the environment, so
/// a record read under another namespace holds a different id for the same face — every face would
/// look like it moved. Stamping it lets a reader tell "these records are not mine" from "this face
/// moved", which is the difference between a refusal and a wrong answer.
/// 身份是 `hash(命名空间, 源码路径, 名字)`，而命名空间来自环境，因此在另一个命名空间下读到的记录对同一个面
/// 持有不同的 id——每个面看起来都搬了家。把它盖进记录，读者才能把"这些记录不是我的"与"这个面搬了家"区分开，
/// 而那正是"拒绝"与"错误答案"的差别。
pub const GENERATION_NAMESPACE_KEY: &str = "namespace\t";

/// The host's crate-shape declaration, read at the **package root** (not under `src/`).
/// 宿主的 crate 形状声明，在**包根**读取（不在 `src/` 之下）。
///
/// It sits beside `Cargo.toml` and `build.rs` because it says what the package *is* rather than what
/// it contains, and because `src/` is the registration tree: a declaration filed there would be read
/// by the discovery walk and, once the shape macro names it, mistaken for a face.
/// 它和 `Cargo.toml`、`build.rs` 放在一起，因为它说的是这个包**是什么**而不是它包含什么，也因为 `src/` 是
/// 注册树：把声明放在那里会被发现遍历读到，而且在形状宏认得它之后会被误当成一个注册面。
pub const ADD_CRATES_FILE: &str = "add_crates.rs";

/// The published record of a declared crate shape, beside the build's other artifacts.
/// 已声明的 crate 形状的落盘记录，与构建的其它产物放在一起。
pub const ADD_CRATES_LOCK_FILE: &str = "add-crates.lock";

/// The first line of the crate-shape lock.
/// crate 形状锁的第一行。
pub const ADD_CRATES_MARKER: &str = "nichlink-crate-shape";

/// The front-end marker that selects the collector adapter.
/// 选择 collector 适配层的前端标记。
pub const FACE_FIELD_COLLECTOR: &str = "collector";

/// The environment variable that pins the first-pass source scope.
/// 固定第一次源码范围的环境变量。
pub const SCOPE_ENV: &str = "NICH_LINK_SCOPE";

/// The environment variable that pins the host entry file.
/// 固定宿主入口文件的环境变量。
pub const ENTRY_ENV: &str = "NICH_LINK_ENTRY";

/// The environment variable that asks the build step for a verbose status
/// line.
/// 向构建步骤索取详细状态行的环境变量。
pub const BUILD_VERBOSE_ENV: &str = "NICH_LINK_BUILD_VERBOSE";

/// The environment variable that caps how many workers a walk may use (audit `T1`).
/// 限制一次遍历最多用几个工作线程的环境变量（审计 `T1`）。
///
/// The default is a rule rather than a number — half the machine, at most eight, never fewer than
/// one core left alone — because machines differ and a tool call must not take a user's whole
/// machine. This variable is the explicit override for a caller who knows better than the rule, and
/// it is part of the text contract for the same reason `NICH_LINK_ENTRY` is: two surfaces read it,
/// and a second literal would drift.
/// 默认值是一条规则而不是一个数字——半台机器、最多八个、至少留一个核——因为机器各不相同，而一次工具
/// 调用不该把用户的整台机器拿走。这个变量是给"比规则更清楚自己处境"的调用方的显式覆盖；它属于文本契约
/// 的理由与 `NICH_LINK_ENTRY` 相同：有两处读它，而第二个字面量会漂。
pub const JOBS_ENV: &str = "NICH_LINK_JOBS";

/// The environment variable that pins the package a surface works on.
/// 固定执行面所工作的包的环境变量。
pub const PACKAGE_ROOT_ENV: &str = "NICH_LINK_PACKAGE_ROOT";

/// The environment variable that pins the namespace authored faces land under.
/// 固定创作的注册面所属命名空间的环境变量。
pub const NAMESPACE_ENV: &str = "NICH_LINK_NAMESPACE";

/// The namespace a face lands under when nothing selects one.
/// 没有任何东西选择时，注册面所属的命名空间。
pub const DEFAULT_NAMESPACE: &str = "nichlink.default";

/// The module whose whole subtree carries the registration machinery, and is
/// therefore never pruned by the first-pass scope.
/// 整棵子树都承载注册机制的模块，因此第一次源码范围修剪永不剪掉它。
pub const SCOPE_REGISTRATION_MODULE: &str = "registry_core";

/// Module names the first-pass source scope never prunes.
/// 第一次源码范围永不修剪的模块名。
///
/// The generated tree keeps every registration rule it can reach, so the
/// modules that spell those rules out survive even when the host scope selects
/// a single face.
/// 生成树保留它能到达的每一条注册规则，因此即使宿主范围只选中一个注册面，写出
/// 这些规则的模块也必须存活。
pub const SCOPE_ALWAYS_INCLUDED: &[&str] = &[
    SCOPE_REGISTRATION_MODULE,
    "registry",
    "rules",
    "registry_rule",
    "root_registry",
];

/// The first line every generated registration face file carries.
/// 每个生成的注册面文件所携带的第一行。
///
/// A text contract rather than a runtime detail: it is what tells a rewrite action whether a face is
/// **NichLink's to rewrite** or the operator's to keep (`is_nichlink_owned_source`), and the bridge's
/// repair suggestions have to ask the same question before they hand back a request that would be
/// refused. It lived as a private constant inside the authoring module, which is why the second
/// reader could only have copied the literal.
/// 这是文本契约而不是运行期细节：它决定了重写动作可以把哪个面当作**NichLink 的**来改、哪个是操作者的
/// 要留着（`is_nichlink_owned_source`），而桥的修复建议必须先问同一个问题，才不至于交回一条会被拒绝的
/// 请求。它原先只是创作模块内部的私有常量，这正是第二个读取方只能抄字面量的原因。
pub const GENERATED_MARKER: &str = "// generated-by=NichLink";

/// Package-level directory holding NichLink's authoring records.
/// 存放 NichLink 创作记录的包级目录。
pub const NICHLINK_DIR: &str = ".nichlink";

/// Where a partition puts the crates it generates, **beside** the host rather than in it.
/// 拆分把生成出来的 crate 放在哪里——在宿主**旁边**，而不是里面。
///
/// Visible, not a dot directory: a host root that lists `app-widgets`, `app-gauges` and `app-facade`
/// next to `host/` and `src/` tells a reader nothing about which of them they may edit, and a hidden
/// one tells them nothing at all. `crates/` uses the same word the declaration file does
/// (`add_crates.rs`), and every package inside it opens with `Generated by NichLink … do not edit`.
/// 可见，不是点目录：一个在 `host/`、`src/` 旁边列出 `app-widgets`、`app-gauges`、`app-facade` 的宿主根，
/// 读不出哪一个是他能改的；藏起来的更是什么都读不出。`crates/` 用的是与声明文件（`add_crates.rs`）同一个词，
/// 而它里面每个包的第一行都写着 `Generated by NichLink … do not edit`。
pub const CRATES_DIR: &str = "crates";

/// Directory name, under `NICHLINK_DIR`, holding external graft plans.
/// `NICHLINK_DIR` 下存放外部 graft 计划的目录名。
pub const EXTERNAL_GRAFT_DIR: &str = "external-grafts";

/// Directory name, under `NICHLINK_DIR`, holding the adoption ledger.
/// `NICHLINK_DIR` 下存放采信台账的目录名。
pub const ADOPTION_DIR: &str = "adopted";

/// File name, under `ADOPTION_DIR`, holding the ledger's lines.
/// `ADOPTION_DIR` 下承载台账各行的文件名。
pub const ADOPTION_FILE: &str = "entries";

/// File name of one external graft plan.
/// 单个外部 graft 计划的文件名。
pub const GRAFT_PLAN_FILE: &str = "graft.plan";

/// Directory name, under `NICHLINK_DIR`, holding the move records.
/// `NICHLINK_DIR` 下存放搬动记录的目录名。
///
/// A move is an **identity change** — `NodeId = hash(namespace, source path, name)` — so these records
/// are a compatibility note for the next reader rather than an audit ledger, and only the most recent
/// few are kept (the maintainer's decision on 2026-10-09).
/// 一次搬动就是一次**身份变化**——`NodeId = hash(命名空间, 源码路径, 名字)`——因此这些记录是留给下一个读者的
/// 兼容提示，而不是审计账本，只保留最近几条（维护者 2026-10-09 的决定）。
pub const MOVES_DIR: &str = "moves";

/// File name, under a numbered directory in [`MOVES_DIR`], holding one move record.
/// [`MOVES_DIR`] 下每个编号目录里存放一条搬动记录的文件名。
pub const MOVE_PLAN_FILE: &str = "move.plan";

/// Directory name, under `NICHLINK_DIR`, holding recorded trace artifacts.
/// `NICHLINK_DIR` 下存放已记录 trace artifact 的目录名。
pub const TRACE_DIR: &str = "traces";

/// File name of one recorded trace artifact.
/// 单个已记录 trace artifact 的文件名。
pub const TRACE_FILE: &str = "nichlink.trace";

/// The environment variable that pins the trace artifact a reader loads.
/// 固定读取方加载哪个 trace artifact 的环境变量。
pub const TRACE_FILE_ENV: &str = "NICH_LINK_TRACE_FILE";

/// The environment variable that selects the trace collection mode.
/// 选择 trace 收集模式的环境变量。
///
/// This lived as a bare literal in the recorder, so a rename here would not have
/// been caught by the shared-contract tests the way `TRACE_FILE_ENV` is: hosts are
/// told to set the variable by name, and the scaffold template writes that name
/// into generated code. `the_text_contracts_keep_their_published_values` now pins
/// it next to its sibling, and `0.1.4` is the version move that let a core symbol
/// be consumed by `run_method` again.
/// 它过去是记录器里的裸字面量，因此这里改名不会被共享契约测试像 `TRACE_FILE_ENV` 那样抓到：宿主是
/// 按名字被告知去设置这个变量的，而脚手架模板会把这个名字写进生成的代码。
/// `the_text_contracts_keep_their_published_values` 现在把它钉在它的同类旁边，而 `0.1.4` 正是
/// 让 `run_method` 又能消费一个 core 符号的那次版本移动。
pub const TRACE_MODE_ENV: &str = "NICH_LINK_TRACE";

/// Whether `path` names `prefix` itself or a segment strictly below it.
/// `path` 是 `prefix` 本身，还是位于其下的某个路径段。
///
/// Four sites used to decide this: `Admission::accepts`, the owned
/// `OwnedAdmission::accepts` (through its private `path_matches`), the
/// connector's external-branch test, and `is_registration_path`. They differed
/// in how they spelled the check — `== prefix || strip_prefix(prefix).starts_with('/')`
/// against `starts_with(&format!("{prefix}/"))` against a bare
/// `strip_prefix(prefix)` — so a path such as `ui` vs `ui2` vs `ui/x` could be
/// admitted by one gate and rejected by another with nothing to catch it.
/// The boundary is the *segment*: text that merely begins with the prefix
/// (`ui2`, `registry_core.rs`) is outside, while the prefix itself and anything
/// after a `/` separator is inside. `admission_twins_accept_and_reject_identical_paths`
/// and `the_shared_prefix_check_owns_equality_and_the_directory_boundary` pin it.
///
/// The two families genuinely need different answers at equality, and that is
/// the one point the merge had to keep apart. `Admission`/`OwnedAdmission` and
/// `is_registration_path` treat the prefix *itself* as inside, so this predicate
/// owns the equality-inclusive meaning. The connector's external-branch test used
/// `starts_with(&format!("{owner_path}/"))`, which is false at equality, because
/// `provider_path == owner_path` is the normal ancestor-provider case — a child
/// registry has exactly its owning face's path — and that provider must still
/// face the owner's admission gate. [`path_is_strictly_under`] names that
/// strict form, and `connector::tests::an_ancestor_provider_is_still_gated_by_the_owner_admission`
/// pins it. Callers must pick the one their gate means rather than inherit the
/// shared default silently.
/// 有四处过去各自判定这件事：`Admission::accepts`、owned 的
/// `OwnedAdmission::accepts`（经其私有 `path_matches`）、连接器的外部分支判断，以及
/// `is_registration_path`。它们的写法互不相同——`== prefix ||
/// strip_prefix(prefix).starts_with('/')`、`starts_with(&format!("{prefix}/"))`、
/// 光秃秃的 `strip_prefix(prefix)`——于是 `ui`、`ui2`、`ui/x` 这类路径可能被一道门
/// 放行、被另一道拒绝，而无从察觉。边界在*路径段*：只是开头相同（`ui2`、
/// `registry_core.rs`）算外面；前缀本身以及 `/` 分隔符之后的任何内容算里面。
/// `admission_twins_accept_and_reject_identical_paths` 与
/// `the_shared_prefix_check_owns_equality_and_the_directory_boundary` 钉住它。
///
/// 两个家族在“相等”这一点上确实需要不同答案，这正是合并时必须分开的那一处。
/// `Admission`/`OwnedAdmission` 与 `is_registration_path` 把前缀**本身**算在里面，
/// 因此本谓词拥有“含相等”这一含义。连接器的外部分支判断用的是
/// `starts_with(&format!("{owner_path}/"))`，相等时为假：`provider_path == owner_path`
/// 正是正常的“祖先提供者”情形——子注册机的路径恰好就是拥有它的那个面的路径——而这个
/// 提供者仍须接受拥有者的准入检查。[`path_is_strictly_under`] 命名这种严格形式，
/// `connector::tests::an_ancestor_provider_is_still_gated_by_the_owner_admission` 钉住它。
/// 调用方必须按自己的门禁含义选择，而不是默默继承共享默认值。
pub(crate) fn path_is_under(path: &str, prefix: &str) -> bool {
    path == prefix
        || path
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// Whether `path` names a segment strictly below `prefix`, excluding equality.
/// `path` 是否位于 `prefix` 之下某个路径段，不含相等。
///
/// The connector's external-branch classification is the one caller that needs
/// equality to be *outside*; see [`path_is_under`] for why the two meanings must
/// stay separate. Expressing it here keeps the distinction at one documented
/// place instead of an ad-hoc `!=` the next reader has to re-derive.
/// 连接器的外部分支分类是唯一需要“相等算外面”的调用方；两种含义为何必须分开见
/// [`path_is_under`]。把它表达在这里，区分就集中在一个有文档的地方，而不是留给下一个
/// 读者去重新推导的一句临时 `!=`。
pub(crate) fn path_is_strictly_under(path: &str, prefix: &str) -> bool {
    path != prefix && path_is_under(path, prefix)
}

/// Whether a package-relative source path lives under the registration module.
/// 包内相对源码路径是否位于注册模块之下。
///
/// Both the first-pass scope and the static plan special-case this subtree, and
/// both used to spell the prefix out separately. The shared predicate counts the
/// prefix itself as "under", but the scope exemption list matches the module
/// name `registry_core` by name rather than by path, so this fixed-prefix form
/// subtracts exactly that one case; without it a file literally named
/// `registry_core` would be treated as both a module name and a subtree.
/// 第一次源码范围与静态计划都会特判这棵子树，而两处过去各写一遍这个前缀。共享谓词把
/// 前缀本身也算作"位于其下"，但范围豁免表是按名字而不是按路径匹配模块名
/// `registry_core` 的，因此这个固定前缀形式恰好减去那一种情况；否则一个真正名为
/// `registry_core` 的文件会同时被当作模块名与子树。
pub fn is_registration_path(relative: &str) -> bool {
    path_is_under(relative, SCOPE_REGISTRATION_MODULE) && relative != SCOPE_REGISTRATION_MODULE
}

/// Whether two strings are the same text, usable in a const context.
/// 两个字符串文本是否相同，可在 const 语境中使用。
///
/// One contract cannot be referenced from where it is used: `include!` and
/// `concat!` only accept literals, so the generated entry's file name has to be
/// written out again at that one site. Pinning the two together with a const
/// assertion turns a drift into a compile error instead of a host that includes
/// a file the build step no longer writes.
/// 有一条契约无法在使用处引用：`include!` 与 `concat!` 只接受字面量，因此生成入口
/// 的文件名在那唯一一处只能再写一遍。用 const 断言把两者钉在一起，漂移就变成编译
/// 错误，而不是去 include 一个构建步骤已不再写的文件。
pub const fn same_text(left: &str, right: &str) -> bool {
    let (left, right) = (left.as_bytes(), right.as_bytes());
    if left.len() != right.len() {
        return false;
    }
    let mut index = 0;
    while index < left.len() {
        if left[index] != right[index] {
            return false;
        }
        index += 1;
    }
    true
}

/// Resolve which package a surface is working on, from values only it can read.
/// 从只有执行面才能读取的取值，解析它正在处理哪一个包。
///
/// One rule, in the kernel, even though every input comes from outside: authoring,
/// Studio and the MCP bridge each carried a copy and disagreed about the last
/// resort. A surface gathers the environment and the working directory — the
/// kernel may not, which is why they arrive as parameters — and the decision is
/// pure. Order: an explicit value (a relative one is relative to the working
/// directory, not to the install location); then the working directory, but only
/// when it actually holds a `Cargo.toml`, because treating a non-package as one
/// is how a surface silently indexes the wrong tree; then the caller's own
/// fallback, which stays surface-specific on purpose.
/// 规则只有一条，住在这里（内核），尽管每个输入都来自外部：authoring、Studio 与 MCP 桥
/// 此前各带一份副本，且对最后兜底的选择并不一致。执行面负责采集环境与当前目录——内核不
/// 允许做这件事，这正是它们以参数传入的原因——而决策本身是纯的。顺序：显式取值（相对路径
/// 相对的是当前目录，而不是安装位置）；然后是当前目录，但只有当它真的含 `Cargo.toml` 时，
/// 因为把不是包的目录当成包正是执行面静默索引错误源码树的方式；最后是调用方自己的兜底，
/// 它有意保持与执行面相关。
pub fn resolve_package_root(
    configured: Option<&Path>,
    current_dir: Option<&Path>,
    current_dir_holds_a_package: bool,
    fallback: &Path,
) -> PathBuf {
    if let Some(path) = configured {
        return if path.is_absolute() {
            path.to_path_buf()
        } else {
            current_dir.unwrap_or_else(|| Path::new(".")).join(path)
        };
    }
    if current_dir_holds_a_package && let Some(current) = current_dir {
        return current.to_path_buf();
    }
    fallback.to_path_buf()
}

/// Resolve the namespace authoring lands in, from the configured value.
/// 从配置值解析创作所属的命名空间。
///
/// The default is a constant rather than a literal at each site, because three
/// surfaces used to spell `nichlink.default` out and a rename would have had to
/// find all three.
/// 默认值是一个常量而不是每个使用处的字面量，因为此前有三个执行面把 `nichlink.default`
/// 写了出来，改名就得找齐三处。
pub fn resolve_namespace(configured: Option<&str>) -> &str {
    configured.unwrap_or(DEFAULT_NAMESPACE)
}

#[cfg(test)]
#[path = "lexicon_tests.rs"]
mod lexicon_tests;
