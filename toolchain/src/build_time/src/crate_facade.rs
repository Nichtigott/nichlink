//! Planning the **facade**: the crate that compiles the cross-crate half (audit `M7`, §M7.33).
//! 规划 **facade**：那个编译跨 crate 那一半的 crate（审计 `M7`，§M7.33）。
//!
//! A partition hands subtrees to ghost crates, and once a subtree leaves, the host's generated tree can
//! no longer name it: `assert_static_registration(control::REGISTRATION.registry_rule,
//! control::object::button::REGISTRATION)` and `StaticGraftCut::from_ids(crate::control::object::
//! button::NODE_ID, …)` both point at a module the host does not compile any more. The facade is the
//! crate that sees **both ends**: it depends on the host, on every ghost, and on every crate the graft
//! declarations name, and it carries the cut table plus the contract assertions with every
//! `crate::<module>` rewritten to the crate that compiles that module.
//! 一次拆分把子树交给幽灵 crate，而子树一旦离开，宿主的生成树就再也点不了它的名：
//! `assert_static_registration(control::REGISTRATION.registry_rule, control::object::button::REGISTRATION)`
//! 与 `StaticGraftCut::from_ids(crate::control::object::button::NODE_ID, …)` 都指向一个宿主不再编译的模块。
//! facade 就是那个**两端都看得见**的 crate：它依赖宿主、每个幽灵、以及 graft 声明点名的每个 crate，并携带
//! 切口表与契约断言——其中每个 `crate::<模块>` 都改写成编译该模块的那个 crate。
//!
//! It is planned here and **written** by the authoring CLI, exactly like a ghost: the planner returns
//! bytes, and nothing here touches the filesystem.
//! 它在这里被规划、由创作 CLI **写下**，与幽灵完全一样：规划器返回字节，这里不碰文件系统。

use std::fs;
use std::path::{Path, PathBuf};

use nichlink_kernel::lexicon;

use super::crate_plan::{GENERATED_MARKER, PlannedCrate, relative_walk, toml_section, toml_value};

/// A facade, planned but not written.
/// 一个已规划但尚未写下的 facade。
#[derive(Debug)]
pub(crate) struct PlannedFacade {
    /// The package name, `<package_prefix>-facade`.
    /// 包名，`<package_prefix>-facade`。
    pub(crate) package: String,
    /// Where it goes: the host's sibling, so the host's relative dependency paths resolve identically.
    /// 落在哪里：宿主的同级目录，这样宿主的相对依赖路径解析结果完全相同。
    pub(crate) directory: PathBuf,
    /// The crates the facade is planned to see: the host and each ghost. The implementation crates the
    /// graft declarations name arrive with the host's copied dependency table, which is why they are not
    /// listed here.
    /// facade 被规划为看得见的 crate：宿主与每个幽灵。graft 声明点名的实现 crate 随照抄来的宿主依赖表一起
    /// 到来，因此不在这里列出。
    pub(crate) dependencies: Vec<String>,
    /// `src/lib.rs`: the host's namespace constant, then the generated plan.
    /// `src/lib.rs`：宿主的命名空间常量，然后是生成的计划。
    pub(crate) lib_rs: String,
    /// `build.rs`: run the host's manifest through the pipeline in **facade** mode.
    /// `build.rs`：让宿主的清单以 **facade** 模式跑一遍管线。
    pub(crate) build_rs: String,
    /// `Cargo.toml`: the host's dependencies, copied verbatim, plus the sibling path dependencies.
    /// `Cargo.toml`：宿主的依赖逐字照抄，再加上同级目录的 path 依赖。
    pub(crate) cargo_toml: String,
}

/// Plan the facade for one host, or `None` when there is nothing cross-crate to carry.
/// 为一个宿主规划 facade；没有跨 crate 的东西要承载时返回 `None`。
///
/// `None` is the honest answer for a declaration whose crates are empty: a facade with no ghost to see
/// and no cut to assert is a crate that exists to say nothing. `implementation_crates` are the crate
/// names the **typed** graft expressions begin with; the host's own dependency table already carries
/// their paths, and the facade — being the host's sibling — resolves those paths identically.
/// 对"声明的 crate 为空"来说 `None` 是诚实的答案：一个没有幽灵可看、没有切口可断言的 facade，是为说空话而
/// 存在的 crate。`implementation_crates` 是**类型化** graft 表达式开头的那些 crate 名；宿主自己的依赖表已经
/// 带着它们的路径，而 facade 作为宿主的同级目录，解析那些路径的结果完全相同。
pub(crate) fn plan_facade(
    host_root: &Path,
    package_prefix: &str,
    namespace: &str,
    host_package: &str,
    planned: &[PlannedCrate],
) -> Result<Option<PlannedFacade>, String> {
    if planned.is_empty() {
        return Ok(None);
    }
    let package = format!("{package_prefix}-facade");
    if package == host_package {
        return Err(format!(
            "add_crates: the facade would be called `{package}`, the same as the host package. \
             Rename the host or pick a different `package_prefix`; a crate cannot depend on itself."
        ));
    }
    let directory = host_root.parent().unwrap_or(host_root).join(&package);
    let mut dependencies: Vec<String> = vec![host_package.to_owned()];
    dependencies.extend(planned.iter().map(|planned| planned.package.clone()));
    dependencies.sort();
    dependencies.dedup();
    let lib_rs = facade_lib_rs(namespace);
    let build_rs = facade_build_rs(host_root, namespace);
    let cargo_toml = facade_cargo_toml(host_root, &package, &directory, host_package, planned)?;
    Ok(Some(PlannedFacade {
        package,
        directory,
        dependencies,
        lib_rs,
        build_rs,
        cargo_toml,
    }))
}

/// The facade's `src/lib.rs`.
/// facade 的 `src/lib.rs`。
fn facade_lib_rs(namespace: &str) -> String {
    format!(
        "//! {marker}: the crate that compiles the cross-crate half of a host's registration tree.\n\
         //! Do not edit — the declaration lives in the host's `{declaration}`.\n\
         //! 由 NichLink 生成：编译宿主注册树跨 crate 那一半的 crate。请勿手工修改——声明住在宿主的\n\
         //! `{declaration}` 里。\n\n\
         pub const NICHLINK_NAMESPACE: &str = {namespace:?};\n\n\
         include!(concat!(env!(\"OUT_DIR\"), \"/generated_lib.rs\"));\n",
        marker = GENERATED_MARKER,
        declaration = lexicon::ADD_CRATES_FILE,
    )
}

/// The facade's `build.rs`.
/// facade 的 `build.rs`。
///
/// Same shape as a ghost's, with one difference that is the whole point: the mode is **facade**, so the
/// tree this crate renders carries no modules and no face identities — it carries the cut table and the
/// contract assertions, rewritten to the crates that own each module.
/// 形状与幽灵的相同，只有一处不同而这处正是要点：模式是 **facade**，因此这个 crate 渲染出来的树不发模块、
/// 不发面的身份——它携带切口表与契约断言，并改写成各自模块的属主 crate。
fn facade_build_rs(host_root: &Path, namespace: &str) -> String {
    let manifest = host_root.join("Cargo.toml");
    let src = host_root.join("src");
    let declaration = host_root.join(lexicon::ADD_CRATES_FILE);
    format!(
        "//! {marker}: build the cross-crate half out of the host's sources.\n\
         //! 由 NichLink 生成：从宿主的源码构建跨 crate 那一半。\n\n\
         fn main() {{\n\
         {i}println!(\"cargo:rerun-if-changed={src}\");\n\
         {i}println!(\"cargo:rerun-if-changed={declaration}\");\n\
         {i}let out = std::path::PathBuf::from(std::env::var(\"OUT_DIR\").expect(\"OUT_DIR\"));\n\
         {i}// A build script is single-threaded at this point, and the value is read by the run below.\n\
         {i}// 构建脚本此刻是单线程的，而这个值由下面的那次运行读取。\n\
         {i}unsafe {{ std::env::set_var({env:?}, \"1\") }};\n\
         {i}nichlink_toolchain::build_time::run_for(\n\
         {i}    std::path::Path::new({manifest:?}),\n\
         {i}    &out,\n\
         {i}    {namespace:?},\n\
         {i})\n\
         {i}.expect(\"nichlink\");\n\
         }}\n",
        i = "    ",
        src = src.display(),
        declaration = declaration.display(),
        env = lexicon::SHAPE_FACADE_ENV,
        manifest = manifest.display(),
        marker = GENERATED_MARKER,
    )
}

/// The facade's `Cargo.toml`: the host's dependencies verbatim, plus the sibling path dependencies.
/// facade 的 `Cargo.toml`：宿主的依赖逐字照抄，加上同级目录的 path 依赖。
///
/// The sibling paths are spelled **relative to the facade**, which is why it is planned as the host's
/// sibling: every relative path in the host's own dependency table then resolves to the same directory
/// it does for the host, and a copied table is not a second answer to "what does this source need".
/// 同级路径是**相对 facade** 拼写的，这也正是把它规划成宿主同级目录的原因：宿主自己依赖表里的每个相对路径
/// 于是都解析到与宿主相同的位置，而照抄来的表不是"这份源码需要什么"的第二个答案。
fn facade_cargo_toml(
    host_root: &Path,
    package: &str,
    directory: &Path,
    host_package: &str,
    planned: &[PlannedCrate],
) -> Result<String, String> {
    let manifest = host_root.join("Cargo.toml");
    let text = fs::read_to_string(&manifest).map_err(|error| {
        format!(
            "add_crates: cannot read {} to copy its dependencies: {error}",
            manifest.display()
        )
    })?;
    let version = toml_value(&text, "version").unwrap_or_else(|| "0.1.0".to_owned());
    let edition = toml_value(&text, "edition").unwrap_or_else(|| "2024".to_owned());
    let mut dependencies = toml_section(&text, "dependencies").unwrap_or_default();
    for (name, path) in sibling_dependencies(directory, host_root, host_package, planned) {
        dependencies.push_str(&format!("{name} = {{ path = {path:?} }}\n"));
    }
    let mut output = format!(
        "# {marker}: the dependencies are the host's, copied verbatim, plus the crates it hands work to.\n\
         # 由 NichLink 生成：依赖是宿主的、逐字照抄，再加上它把工作交出去的那些 crate。\n\
         [package]\nname = {package:?}\nversion = {version:?}\nedition = {edition:?}\n\
         publish = false\n\n[dependencies]\n{dependencies}",
        marker = GENERATED_MARKER,
    );
    if let Some(block) = toml_section(&text, "build-dependencies") {
        output.push_str(&format!("\n[build-dependencies]\n{block}"));
    }
    Ok(output)
}

/// `(dependency name, relative path)` for the host and for every ghost.
/// 宿主与每个幽灵的 `(依赖名, 相对路径)`。
fn sibling_dependencies(
    directory: &Path,
    host_root: &Path,
    host_package: &str,
    planned: &[PlannedCrate],
) -> Vec<(String, String)> {
    let mut siblings = vec![(host_package.to_owned(), host_root.to_path_buf())];
    siblings.extend(
        planned
            .iter()
            .map(|planned| (planned.package.clone(), planned.directory.clone())),
    );
    siblings
        .into_iter()
        .filter_map(|(name, target)| {
            relative_walk(directory, &target.join("Cargo.toml")).map(|walk| {
                (
                    name.clone(),
                    walk.trim_end_matches("/Cargo.toml").to_owned(),
                )
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "crate_facade_tests.rs"]
mod crate_facade_tests;
