//! Recursive walk over the discovered module tree, one render pass.
//! 对已发现模块树的递归遍历，即一次渲染过程。

use std::fmt::Write as _;
use std::path::Path;

use nichlink::lexicon;

use super::ide::{IdeShadow, emit_face_declaration, ide_shadow};
use crate::{Node, SourceScope, module_feature, node_id, relative_display, source_is_active};

/// Render every node of one tree, returning the IDE shadows collected on the
/// way for the top-level pass to emit.
/// 渲染一棵树的所有节点，并返回途中收集到的 IDE 影子声明，供顶层流程发射。
pub(super) fn render_nodes(
    output: &mut String,
    src: &Path,
    scope: &SourceScope,
    nodes: &[Node],
) -> Vec<IdeShadow> {
    let mut pass = RenderPass {
        src,
        scope,
        ide_shadows: Vec::new(),
    };
    for node in nodes {
        render_node(output, &mut pass, node, 0, false, "");
    }
    pass.ide_shadows
}

/// Mutable state threaded through one render pass.
/// 一次渲染过程中传递的可变状态。
struct RenderPass<'a> {
    src: &'a Path,
    scope: &'a SourceScope,
    ide_shadows: Vec<IdeShadow>,
}

fn render_node(
    output: &mut String,
    pass: &mut RenderPass<'_>,
    node: &Node,
    depth: usize,
    selected_ancestor: bool,
    parent_chain: &str,
) {
    if node
        .file
        .as_ref()
        .is_some_and(|file| relative_display(pass.src, file) == "registry_core/macros/macros.rs")
    {
        return;
    }
    let selected_here = selected_ancestor
        || node_id(pass.src, node).is_some_and(|id| {
            pass.scope
                .roots
                .as_ref()
                .is_some_and(|roots| roots.contains(&id))
        });
    if !pass.scope.includes(pass.src, node, selected_ancestor) {
        return;
    }
    let include_source = source_is_active(pass.src, node, pass.scope, selected_ancestor);
    let indent = "    ".repeat(depth);
    let mut shadow_cfg = String::new();
    if depth == 0 && node.name == "compile_error_demo" {
        writeln!(output, "{indent}#[cfg(feature = \"compile_error_demo\")]").unwrap();
        writeln!(shadow_cfg, "#[cfg(feature = \"compile_error_demo\")]").unwrap();
    }
    if let Some(feature) = module_feature(pass.src, node) {
        writeln!(output, "{indent}#[cfg(feature = {feature:?})]").unwrap();
        writeln!(shadow_cfg, "#[cfg(feature = {feature:?})]").unwrap();
    }
    writeln!(
        output,
        "{indent}#[allow(dead_code, unused_imports, ambiguous_glob_reexports, clippy::items_after_test_module, clippy::module_inception)]"
    )
    .unwrap();
    // A face file is loaded as a real module through `#[path]`, never through
    // `include!`, and never as a copy: that is what preserves a face file's
    // `//!` header, since an `include!` expansion cannot introduce inner
    // attributes.
    // 注册面文件通过 `#[path]` 以真实模块载入，不走 `include!`，也不是副本：
    // 这既保住了面文件的 `//!` 头（`include!` 展开无法引入 inner attribute），
    // 也让编译器直接读取作者正在编辑的那个文件。
    //
    // `#[path]` alone is not enough for the IDE. rust-analyzer only applies the
    // attribute when the declaration is at the top level of a file or an
    // expansion, so every face nested inside the generated inline modules also
    // gets a crate-root shadow declaration guarded by `cfg(rust_analyzer)` while
    // its real declaration is guarded by `cfg(not(rust_analyzer))`. rustc reads
    // only the real one, so ids, module paths and diagnostics are unchanged.
    // 光有 `#[path]` 对 IDE 还不够。rust-analyzer 只在声明位于文件或展开的顶层时
    // 才应用该属性，因此嵌在生成内联模块里的每个注册面还要多一条 crate 根影子
    // 声明（`cfg(rust_analyzer)`），其真实声明则改由 `cfg(not(rust_analyzer))`
    // 把关。rustc 只读真实的那份，id、模块路径与诊断因此不变。
    //
    // The declaration macros derive `source` from `file!()` and the registry
    // name from `module_path!()`, so no per-face constants are injected here.
    // For a leaf face the file is loaded under its own name, which keeps the
    // public module path unchanged. A face that owns child registries must also
    // be their container, so it loads into a same-named child module and
    // re-exports; only that case gains a module-path segment.
    // 声明宏从 `file!()` 推导 `source`、从 `module_path!()` 推导注册机名，因此
    // 这里不再注入逐面常量。叶子面以自身名字载入，公开模块路径保持不变；拥有
    // 子注册机的面必须同时充当容器，于是载入到同名子模块再重导出——只有这种
    // 情况会多出一段模块路径。
    let absolute = node
        .file
        .as_ref()
        .map(|file| file.to_string_lossy().replace('\\', "/"));
    let chain = if parent_chain.is_empty() {
        node.name.clone()
    } else {
        format!("{parent_chain}_{}", node.name)
    };
    if let (true, Some(absolute)) = (include_source, absolute.as_ref())
        && node.children.is_empty()
    {
        // A leaf face is declared at its own indent, so only a nested one needs
        // the IDE to be given a second, crate-root view of the same file.
        // 叶子面声明在自己的缩进处，因此只有嵌套的那些才需要给 IDE 补一份
        // crate 根视角的同一文件。
        emit_face_declaration(
            output,
            node,
            absolute,
            depth,
            ide_shadow(include_source, depth, &chain, absolute, &shadow_cfg, true),
            &mut pass.ide_shadows,
        );
        return;
    }
    // A host that turns on `#![warn(missing_docs)]` sees this generated file as
    // its own source, so inline container submodules need docs too: a leaf face's
    // module takes its docs from the face file's `//!` header, but a container
    // module has no file of its own to take them from.
    // 开启 `#![warn(missing_docs)]` 的宿主会把这份生成文件当成自己的源码，因此内联容器
    // 子模块也需要文档：叶子面的模块文档来自面文件的 `//!` 头，而容器模块没有自己的文件
    // 可依托。
    writeln!(
        output,
        "{indent}/// The `{}` registration subtree this build generated from the folder layout.",
        node.name
    )
    .unwrap();
    writeln!(
        output,
        "{indent}/// 本次构建由文件夹布局生成的 `{}` 注册子树。",
        node.name
    )
    .unwrap();
    writeln!(output, "{indent}pub mod {} {{", node.name).unwrap();
    if let (true, Some(absolute)) = (include_source, absolute.as_ref()) {
        // A container face owns child registries, so its file always loads into
        // a child module one level deeper: it always needs the IDE view.
        // 容器面同时承载子注册机，其文件总是载入到深一层的子模块：它始终需要
        // IDE 视角。
        emit_face_declaration(
            output,
            node,
            absolute,
            depth + 1,
            ide_shadow(
                include_source,
                depth + 1,
                &chain,
                absolute,
                &shadow_cfg,
                false,
            ),
            &mut pass.ide_shadows,
        );
    }
    for child in &node.children {
        render_node(
            output,
            pass,
            child,
            depth + 1,
            selected_ancestor || selected_here,
            &chain,
        );
    }
    if depth == 0 && node.name == lexicon::SCOPE_REGISTRATION_MODULE {
        let inner = "    ".repeat(depth + 1);
        for child in node.children.iter().filter(|child| child.name != "macros") {
            if let Some(feature) = module_feature(pass.src, child) {
                writeln!(output, "{inner}#[cfg(feature = {feature:?})]").unwrap();
            }
            writeln!(output, "{inner}pub use {}::*;", child.name).unwrap();
        }
    }
    writeln!(output, "{indent}}}").unwrap();
}
