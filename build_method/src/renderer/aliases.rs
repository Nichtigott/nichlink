//! Visible object aliases and the crate paths generated code spells.
//! 可见的对象别名，以及生成代码写出的 crate 路径。
//!
//! The aliases keep the registry hierarchy visible in the source while
//! forwarding to the one hidden implementation; the two path helpers are the
//! only place the runtime crate name is spelled for generated code.
//! 别名让注册机层级在源码里保持可见，同时转发到唯一的隐藏实现；两个路径辅助函数
//! 是生成代码写出运行期 crate 名的唯一位置。

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use nichlink::lexicon;

use crate::{Node, relative_display};

/// The absolute path of the runtime crate's registry surface, as generated code
/// must spell it.
/// 生成代码必须写出的运行期 crate 注册面绝对路径。
///
/// The crate name itself is a kernel contract, so a host that addresses it
/// differently has one place to change instead of every emitted line.
/// crate 名本身是内核契约：寻址方式若变，改一处而不是每一条发射出来的行。
pub(super) fn registry_path() -> String {
    format!("::{}::registry_core", lexicon::RUN_METHOD_CRATE)
}

/// The absolute path of the runtime crate, as generated code addresses the
/// macros that live at its root.
/// 生成代码寻址位于运行期 crate 根部的宏时使用的绝对路径。
pub(super) fn run_method_path() -> String {
    format!("::{}", lexicon::RUN_METHOD_CRATE)
}

/// Emit one visible declaration macro for every folder-level registry name.
/// 为文件夹级注册机生成对应的可见声明宏。
///
/// The aliases keep the hierarchy in the source (`test1_object!` instead of
/// one anonymous primitive) while forwarding to the one core
/// implementation. Explicit `parent:` fields remain the source of truth.
/// 别名让源码直接表达层级（如 `test1_object!`），同时仍只转发到唯一的
/// 隐藏实现；真正的父子关系仍以显式 `parent:` 字段为准。
///
/// The doc comment is the vocabulary an editor shows on hover, because a macro
/// body cannot be read as a field list. The matcher stays parenthesised: a
/// brace-delimited one makes rust-analyzer drop completion at an empty value
/// position such as `kind: |`, which is exactly where a field value is typed.
/// 文档注释是编辑器悬停时展示的词表——宏体本身读不成字段列表。匹配器保持括号定界：
/// 改成花括号会让 rust-analyzer 在 `kind: |` 这类空值位置失去补全，而那正是要填
/// 字段值的地方。
///
/// The delimiter an editor *inserts* is a separate matter, and rust-analyzer
/// decides it from the macro's own declaration: without a hint it inserts
/// `name!($0)` — the `()` the author did not ask for — while
/// `#[rust_analyzer::macro_style(braces)]` makes it insert `name!{$0}`, which is
/// the form every face is written in. rustc accepts the tool attribute without a
/// warning, so the hint costs nothing at build time.
/// 编辑器**插入**哪种定界符是另一件事，由 rust-analyzer 依据宏自身的声明决定：没有提示
/// 时它插入 `name!($0)`——作者并不想要的那个括号；而
/// `#[rust_analyzer::macro_style(braces)]` 让它插入 `name!{$0}`，也就是每个注册面实际
/// 使用的形式。rustc 接受这个工具属性且不报警告，因此这条提示不花构建期代价。
pub(super) fn render_object_aliases(output: &mut String, src: &Path, nodes: &[Node]) {
    let mut names = BTreeSet::from(["root".to_owned()]);
    collect_object_aliases(src, nodes, &mut names);
    let vocabulary = "/// `kind` is the handle-marker type this file declares (`pub struct <Kind>;`), so\n/// write that type first and reference it here: an editor cannot complete a name\n/// the author has not written yet, and `kind` is captured as an identifier\n/// rather than an expression, which is also why value completion does not fire\n/// there. Every other field completes normally.\n/// `kind` 就是本文件声明的那个 handle 标记类型（`pub struct <Kind>;`）：先写出该类型，\n/// 再在这里引用它。编辑器无法补全一个作者还没写下的名字，而且 `kind` 是以标识符而非\n/// 表达式捕获的——这也是它的值位不会弹候选的原因。其余字段的值都能正常补全。\n/// Declare a registration face: `kind` first, then any of `preset`, `parts`,\n/// `name`, `summary`, `params`, `exports`, `handle`, `stable_name`,\n/// `needs_registry`, `registry_name`, `parent`, `getting_from_other_registry`,\n/// `registry_rule_path`, `registry_rule`, `admission`, `handle_traits`,\n/// `handle_contracts`, `part_traits`, `part_contracts`, `requires`,\n/// `provides`, `expected_output`, `actual_output`, `flow`, `flow_provider`,\n/// `plugin`, `runtime_checks` — in that order, each one optional.\n/// 声明一个注册面：先写 `kind`，其后可依次使用 `preset`、`parts`、`name`、\n/// `summary`、`params`、`exports`、`handle`、`stable_name`、`needs_registry`、\n/// `registry_name`、`parent`、`getting_from_other_registry`、\n/// `registry_rule_path`、`registry_rule`、`admission`、`handle_traits`、\n/// `handle_contracts`、`part_traits`、`part_contracts`、`requires`、\n/// `provides`、`expected_output`、`actual_output`、`flow`、`flow_provider`、\n/// `plugin`、`runtime_checks`——顺序如上，每一项都可省略。";
    let run_method = run_method_path();
    for name in names {
        writeln!(
            output,
            "{vocabulary}\n#[doc(hidden)]\n#[allow(unused_macros)]\n#[rust_analyzer::macro_style(braces)]\nmacro_rules! {name}_object {{\n    ($($tokens:tt)*) => {{\n        {run_method}::__nichlink_object! {{ $($tokens)* }}\n        #[cfg(rust_analyzer)]\n        {run_method}::face_fields_mirror! {{ $($tokens)* }}\n    }}\n}}\n#[allow(unused_imports)]\npub(crate) use {name}_object;\n"
        )
        .unwrap();
    }
}

fn collect_object_aliases(src: &Path, nodes: &[Node], names: &mut BTreeSet<String>) {
    for node in nodes {
        let owns_registry = node.file.as_ref().is_some_and(|file| {
            fs::read_to_string(file)
                .ok()
                .and_then(|source| {
                    crate::parsed_face(&source, &relative_display(src, file))
                        .and_then(|face| face.boolean("needs_registry"))
                })
                .unwrap_or(false)
        });
        if owns_registry {
            names.insert(node.name.clone());
        }
        collect_object_aliases(src, &node.children, names);
    }
}

#[cfg(test)]
mod tests {
    use super::{render_object_aliases, run_method_path};
    use crate::Node;
    use crate::renderer::test_support::{temporary_directory, write_registry};
    use std::fs;

    #[test]
    fn aliases_cover_each_registry_level() {
        let root = temporary_directory("object-aliases");
        let workspace = root.join("workspace/workspace.rs");
        let panel = root.join("workspace/object/panel/panel.rs");
        let control = root.join("control/control.rs");
        write_registry(
            &workspace,
            "root_object",
            "Workspace",
            "crate::ROOT_NODE_ID",
        );
        write_registry(
            &panel,
            "workspace_object",
            "Panel",
            "crate::workspace::NODE_ID",
        );
        write_registry(&control, "root_object", "Control", "crate::ROOT_NODE_ID");
        let nodes = vec![
            Node {
                name: "control".to_owned(),
                file: Some(control),
                children: Vec::new(),
            },
            Node {
                name: "workspace".to_owned(),
                file: Some(workspace),
                children: vec![Node {
                    name: "panel".to_owned(),
                    file: Some(panel),
                    children: Vec::new(),
                }],
            },
        ];

        let mut output = String::new();
        render_object_aliases(&mut output, &root, &nodes);

        // Editors cannot read a macro's token tree, so every generated alias
        // also hands the author's tokens to `face_fields_mirror!`, which is what
        // turns them into a field list an editor can complete. rustc never
        // expands that call: it carries `cfg(rust_analyzer)`.
        // 编辑器读不了宏的 token 树，因此每个生成的别名还把作者的 token 交给
        // `face_fields_mirror!`，由它变成编辑器能补全的字段列表。rustc 从不展开这次
        // 调用：它带 `cfg(rust_analyzer)`。
        assert!(
            output.contains(&format!(
                "#[cfg(rust_analyzer)]\n        {}::face_fields_mirror! {{ $($tokens)* }}",
                run_method_path()
            )),
            "aliases must expose the field vocabulary to an editor: {output}"
        );
        assert!(output.contains("macro_rules! root_object"));
        assert!(output.contains("macro_rules! workspace_object"));
        assert!(output.contains("macro_rules! panel_object"));
        assert!(output.contains("macro_rules! control_object"));
        assert!(output.contains(&format!("{}::__nichlink_object!", run_method_path())));
        // An editor inserts the call with round brackets, so the alias matcher
        // must be the delimiter-agnostic token tree and forward it verbatim: a
        // `{ … }` matcher would reject `root_object!( … )` outright.
        // 编辑器插入调用时用圆括号，因此别名的匹配器必须是与分隔符无关的 token 树，
        // 并原样转发它：`{ … }` 匹配器会直接拒绝 `root_object!( … )`。
        assert!(
            output.contains("macro_rules! root_object {\n    ($($tokens:tt)*) => {"),
            "the alias must accept any delimiter: {output}"
        );
        // An editor inserts the delimiter the macro asks for, and every face is
        // written with braces: without this hint rust-analyzer inserts `name!()`.
        // 编辑器插入的是宏自己要求的定界符，而每个注册面都写成花括号：没有这条提示时
        // rust-analyzer 会插入 `name!()`。
        assert!(
            output.contains("#[rust_analyzer::macro_style(braces)]\nmacro_rules! root_object"),
            "the alias must ask for brace completion: {output}"
        );
        assert!(output.contains("#[allow(unused_macros)]"));
        assert!(output.contains("#[allow(unused_imports)]"));
        fs::remove_dir_all(root).expect("temporary fixture cleanup");
    }

    #[test]
    fn empty_tree_does_not_emit_a_legacy_control_alias() {
        let root = temporary_directory("empty-object-aliases");
        let mut output = String::new();

        render_object_aliases(&mut output, &root, &[]);

        assert!(output.contains("macro_rules! root_object"));
        assert!(!output.contains("macro_rules! control_object"));
        fs::remove_dir_all(root).expect("temporary fixture cleanup");
    }
}
