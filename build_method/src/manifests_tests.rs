//! Tests for the audit manifests: which functions a file declares, and which
//! symbol the release prunes.
//! 审计清单的测试：一个文件声明了哪些函数，以及发布期修剪的是哪个符号。

use std::fs;
use std::path::PathBuf;

/// A throwaway package with one face file, so a manifest writer has a face to key
/// its rows on. `body` is appended after the face invocation, because one file may
/// hold only one registration face.
/// 一个含单个注册面文件的一次性包，因此清单写入方有面可以给行做键。`body` 追加在面调用之后，
/// 因为一个文件只能承载一个注册面。
fn package(label: &str, body: &str) -> (PathBuf, PathBuf) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "nichlink-manifests-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    let src = root.join("src");
    fs::create_dir_all(src.join("dial")).expect("face directory");
    fs::write(
        src.join("dial/dial.rs"),
        format!("crate::root_object! {{\n    kind: Dial,\n}}\n{body}"),
    )
    .expect("face file");
    (root, src)
}

/// The mask the kernel scanner applies is the point of this column: `fn ` inside a
/// comment or a doc example is not a declaration, and the line scanner this file
/// used to carry listed both.
/// 内核扫描器所做的屏蔽正是这一列的意义：注释或文档示例里的 `fn ` 不是声明，而本文件过去带的那套
/// 按行扫描器把两者都列了出来。
#[test]
fn a_commented_out_function_is_not_a_declaration() {
    let (root, src) = package(
        "masked",
        "pub fn real() {}\n\
         // fn ghost() {}\n\
         /* fn in_a_block_comment() {} */\n\
         /// A doc example: fn documented() {}\n\
         pub fn after_doc() {}\n\
         pub fn with_string() { let text = \"fn in_a_string() {}\"; let _ = text; }\n",
    );
    let nodes = crate::source_walk::discover_root(&src);
    let out = root.join("out");
    fs::create_dir_all(&out).expect("out directory");
    super::write_function_manifest(&src, &nodes, &out).expect("the manifest writes");
    let text = fs::read_to_string(out.join("function_manifest.tsv")).expect("rows");
    assert!(text.contains("dial::real"), "{text}");
    assert!(text.contains("dial::after_doc"), "{text}");
    assert!(text.contains("dial::with_string"), "{text}");
    for ghost in ["ghost", "in_a_block_comment", "documented", "in_a_string"] {
        assert!(
            !text.contains(ghost),
            "`{ghost}` is not a declaration: the kernel scanner masks what is not Rust: {text}"
        );
    }
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_method_carries_no_impl_owner_the_scanner_cannot_know() {
    let (root, src) = package("owner", "impl Dial {\n    pub fn method(&self) {}\n}\n");
    let nodes = crate::source_walk::discover_root(&src);
    let out = root.join("out");
    fs::create_dir_all(&out).expect("out directory");
    super::write_function_manifest(&src, &nodes, &out).expect("the manifest writes");
    let text = fs::read_to_string(out.join("function_manifest.tsv")).expect("rows");
    assert!(text.contains("dial::method"), "{text}");
    assert!(
        !text.contains("Dial::method"),
        "the impl owner was a guess, and it is gone: {text}"
    );
    let _ = fs::remove_dir_all(&root);
}

/// A tracked symbol comes from the facts that name it: a module item keeps its
/// module path, and `optional_pruning_probe` — a method on the face's own type —
/// starts at that type's name, which is the face's `kind`. The row scanner this
/// replaces returned the literal `Button::optional_pruning_probe` for every face.
/// 被跟踪的符号来自命名它的事实：模块条目保留模块路径，而 `optional_pruning_probe`——面自己类型上
/// 的方法——从那个类型的名字开始，也就是面的 `kind`。它替换掉的按行扫描器对每个面都返回字面量
/// `Button::optional_pruning_probe`。
#[test]
fn a_pruning_probe_symbol_is_named_after_the_face_kind() {
    let (root, src) = package(
        "pruning",
        "impl Dial {\n    pub fn optional_pruning_probe(&self) {}\n}\n\n\
         pub static PRUNING_TABLE: &[&str] = &[];\n",
    );
    let nodes = crate::source_walk::discover_root(&src);
    let out = root.join("out");
    fs::create_dir_all(&out).expect("out directory");
    super::write_pruning_manifest(&src, &nodes, &out).expect("the manifest writes");
    let text = fs::read_to_string(out.join("pruning_manifest.tsv")).expect("rows");
    assert!(text.contains("Dial::optional_pruning_probe"), "{text}");
    assert!(
        !text.contains("Button::optional_pruning_probe"),
        "the symbol must come from this face's kind, not from a literal: {text}"
    );
    assert!(
        text.contains("dial::PRUNING_TABLE"),
        "a module item keeps its module path: {text}"
    );
    let _ = fs::remove_dir_all(&root);
}
