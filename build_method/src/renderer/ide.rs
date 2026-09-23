//! IDE-only shadow declarations for faces rust-analyzer cannot reach.
//! 为 rust-analyzer 无法抵达的注册面生成仅供 IDE 的影子声明。
//!
//! `rustc` honours `#[path]` at any nesting depth, while rust-analyzer applies
//! it only at the top level of a file or an expansion. Every nested face
//! therefore also gets a crate-root declaration that only the IDE sees, gated
//! so each tool reads exactly one binding.
//! `rustc` 在任意嵌套深度都遵守 `#[path]`，而 rust-analyzer 只在文件或展开的顶层
//! 应用它。因此每个嵌套注册面还要多一条只有 IDE 才看到的 crate 根声明，并经门控
//! 使两种工具各自只读到一个同名绑定。

use std::fmt::Write as _;

use crate::Node;

/// One face file that rust-analyzer cannot reach through the real tree.
/// 一个 rust-analyzer 无法通过真实树抵达的注册面文件。
///
/// `rustc` honours `#[path]` at any nesting depth; `rust-analyzer` applies it
/// only when the `mod` declaration sits at the top level of a file or an
/// expansion. Inside the generated inline modules it drops the attribute, so
/// the face file never joins the crate and the author loses completion,
/// go-to-definition and hover. Each such face therefore also gets a crate-root
/// declaration that only the IDE sees.
/// `rustc` 在任意嵌套深度都遵守 `#[path]`；`rust-analyzer` 只在声明位于文件或
/// 展开的顶层时才应用它。在内联模块里它会丢弃该属性，面文件因此从未进入 crate，
/// 作者也就失去补全、跳转与悬停。所以每个这样的面还要有一条只有 IDE 才看到的
/// crate 根声明。
pub(super) struct IdeShadow {
    /// Crate-root module name carrying the same file for the IDE.
    /// 为 IDE 载入同一文件的 crate 根模块名。
    pub(super) module: String,
    /// Absolute, separator-normalized face path.
    /// 绝对且分隔符归一化的注册面路径。
    file: String,
    /// `cfg` attributes mirrored from the real declaration.
    /// 与真实声明一致的 `cfg` 属性。
    cfg: String,
    /// Whether the real declaration is `pub mod`, so the IDE view matches it.
    /// 真实声明是否为 `pub mod`，以便 IDE 视角与之一致。
    pub(super) public: bool,
}

/// Emit the IDE-only declarations collected while walking the tree.
/// 发射遍历模块树时收集到的、仅供 IDE 的声明。
///
/// These sit at the top level of the generated file, which is the only place
/// `rust-analyzer` applies `#[path]`. `rustc` never reads them: the real
/// declarations carry `cfg(not(rust_analyzer))` and these carry
/// `cfg(rust_analyzer)`, so exactly one of the two exists for either tool. Ids,
/// module paths and diagnostics are therefore untouched for `rustc`.
/// 它们位于生成文件的顶层——这是 `rust-analyzer` 唯一应用 `#[path]` 的位置。
/// `rustc` 永不读取它们：真实声明带 `cfg(not(rust_analyzer))`，这些带
/// `cfg(rust_analyzer)`，两种工具各自只看到其中一份。因此 `rustc` 的 id、模块
/// 路径与诊断完全不变。
pub(super) fn render_ide_shadows(output: &mut String, shadows: &[IdeShadow]) {
    if shadows.is_empty() {
        return;
    }
    output.push_str(
        "\n// IDE-only declarations: rust-analyzer ignores `#[path]` inside an inline module.\n// 仅供 IDE 的声明：rust-analyzer 会忽略内联模块里的 `#[path]`。\n",
    );
    for shadow in shadows {
        output.push_str(&shadow.cfg);
        output.push_str("#[allow(unexpected_cfgs)]\n#[cfg(rust_analyzer)]\n");
        output.push_str("#[doc(hidden)]\n");
        writeln!(output, "#[path = {:?}]", shadow.file).unwrap();
        if shadow.public {
            writeln!(output, "pub mod {};", shadow.module).unwrap();
        } else {
            writeln!(output, "mod {};", shadow.module).unwrap();
        }
    }
}

/// Build the crate-root IDE view of a face when the real declaration is nested.
/// 当真实声明处于嵌套位置时，构造该注册面的 crate 根 IDE 视角。
pub(super) fn ide_shadow(
    enabled: bool,
    declaration_depth: usize,
    chain: &str,
    absolute: &str,
    cfg: &str,
    public: bool,
) -> Option<IdeShadow> {
    (enabled && declaration_depth > 0).then(|| IdeShadow {
        module: format!("__nichlink_ra_{chain}"),
        file: absolute.to_owned(),
        cfg: cfg.to_owned(),
        public,
    })
}

/// Emit one `#[path]` face declaration, plus the IDE-only view when needed.
/// 发射一条 `#[path]` 注册面声明，需要时附上仅供 IDE 的视角。
///
/// The real declaration is gated with `cfg(not(rust_analyzer))` whenever a
/// shadow exists, so each tool sees exactly one binding for the name and
/// rust-analyzer never reports a duplicate definition.
/// 只要存在影子声明，真实声明就由 `cfg(not(rust_analyzer))` 把关，于是两种工具
/// 各自只看到一个同名绑定，rust-analyzer 也不会报重复定义。
pub(super) fn emit_face_declaration(
    output: &mut String,
    node: &Node,
    absolute: &str,
    declaration_depth: usize,
    shadow: Option<IdeShadow>,
    ide_shadows: &mut Vec<IdeShadow>,
) {
    let indent = "    ".repeat(declaration_depth);
    if shadow.is_some() {
        writeln!(output, "{indent}#[cfg(not(rust_analyzer))]").unwrap();
    }
    writeln!(output, "{indent}#[path = {absolute:?}]").unwrap();
    if node.children.is_empty() {
        writeln!(output, "{indent}pub mod {};", node.name).unwrap();
    } else {
        writeln!(output, "{indent}mod {};", node.name).unwrap();
    }
    if let Some(shadow) = shadow {
        writeln!(output, "{indent}#[cfg(rust_analyzer)]").unwrap();
        // The alias carries the real declaration's visibility, so a `pub mod`
        // face stays public to the rest of the workspace in the IDE too.
        // 别名沿用真实声明的可见性，因此 `pub mod` 的注册面在 IDE 里对其余
        // workspace 成员仍然是公开的。
        let visibility = if shadow.public { "pub" } else { "pub(crate)" };
        writeln!(
            output,
            "{indent}{visibility} use crate::{} as {};",
            shadow.module, node.name
        )
        .unwrap();
        ide_shadows.push(shadow);
    }
    if !node.children.is_empty() {
        writeln!(output, "{indent}pub use {}::*;", node.name).unwrap();
    }
}
