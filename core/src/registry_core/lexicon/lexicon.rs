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

use std::path::{Path, PathBuf};

/// The generated crate entry the build step writes and the host includes.
/// 构建步骤写盘、宿主 crate include 的生成入口文件名。
pub const GENERATED_LIB_FILE: &str = "generated_lib.rs";

/// The runtime crate's name, as generated code and the face front end address
/// it.
/// 运行期 crate 的名字——生成代码与宏前端这样寻址它。
pub const RUN_METHOD_CRATE: &str = "nichlink_run_method";

/// The face field that marks replaceable plugin surface.
/// 标记可替换插件面的注册面字段。
pub const FACE_FIELD_PLUGIN: &str = "plugin";

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

/// Package-level directory holding NichLink's authoring records.
/// 存放 NichLink 创作记录的包级目录。
pub const NICHLINK_DIR: &str = ".nichlink";

/// Directory name, under `NICHLINK_DIR`, holding external graft plans.
/// `NICHLINK_DIR` 下存放外部 graft 计划的目录名。
pub const EXTERNAL_GRAFT_DIR: &str = "external-grafts";

/// File name of one external graft plan.
/// 单个外部 graft 计划的文件名。
pub const GRAFT_PLAN_FILE: &str = "graft.plan";

/// Directory name, under `NICHLINK_DIR`, holding recorded trace artifacts.
/// `NICHLINK_DIR` 下存放已记录 trace artifact 的目录名。
pub const TRACE_DIR: &str = "traces";

/// File name of one recorded trace artifact.
/// 单个已记录 trace artifact 的文件名。
pub const TRACE_FILE: &str = "nichlink.trace";

/// The environment variable that pins the trace artifact a reader loads.
/// 固定读取方加载哪个 trace artifact 的环境变量。
pub const TRACE_FILE_ENV: &str = "NICH_LINK_TRACE_FILE";

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
mod tests;
