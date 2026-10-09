//! The item vocabulary is the kernel's, not a surface's prefix list.
//! 条目词表归内核所有，而不是执行面自己拼的前缀词表。
//!
//! Studio used to carry a thirteen-prefix word list of its own, which missed
//! qualified declarations and turned `let` bindings into symbols (audit `STU-S-06`).
//! This file is the outside-of-the-crate pin for the one implementation that
//! replaced it: it reaches `item_symbols` through `xirang_kernel::source`, the path a
//! surface has.
//! Studio 曾自己带一份 13 条前缀的词表，它漏掉带可见性的声明、又把 `let` 绑定当成符号（审计
//! `STU-S-06`）。本文件是取代它的那份唯一实现的 crate 外钉子：它经 `xirang_kernel::source` 取
//! `item_symbols`，也就是执行面拥有的那条路径。

use xirang_kernel::source::item_symbols;

/// The shapes the old word list got wrong, each written the way a host writes it.
/// 旧词表弄错的那些形状，各按宿主真实写法书写。
const SOURCE: &str = "\
pub(crate) async fn load(value: usize) {}\n\
impl Widget {\n\
    unsafe fn render(&self) {}\n\
}\n\
trait Sink {\n\
    fn name(&self) -> &str;\n\
}\n\
pub(crate) struct Panel;\n\
pub(super) enum Mode {\n\
    A,\n\
}\n\
pub union Bits {\n\
    word: u32,\n\
}\n\
pub trait Marker {}\n\
pub type Alias = usize;\n\
pub const LIMIT: usize = 3;\n\
static TABLES: [u8; 1] = [0];\n\
mod nested;\n\
fn outer(value: usize) {\n\
    let text = \"load\";\n\
    let pointer: fn(u32) -> u32 = add;\n\
}\n";

/// The names `item_symbols` reports, optionally only the functions.
/// `item_symbols` 报告的名字，可选只要函数。
fn names(is_function: Option<bool>) -> Vec<String> {
    item_symbols(SOURCE)
        .into_iter()
        .filter(|item| is_function.is_none_or(|wanted| item.is_function == wanted))
        .map(|item| item.name)
        .collect()
}

/// A qualified function and an `unsafe` method are functions, and the bodyless `fn`
/// a trait declares is one too.
/// 带可见性的函数与 `unsafe` 方法都是函数，trait 里那条没有函数体的 `fn` 也是。
#[test]
fn qualified_functions_are_functions() {
    let functions = names(Some(true));
    for expected in ["load", "render", "name", "outer"] {
        assert!(
            functions.iter().any(|name| name == expected),
            "`{expected}` must be an item: {functions:?}"
        );
    }
    let load = item_symbols(SOURCE)
        .into_iter()
        .find(|item| item.name == "load")
        .expect("the qualified function is listed");
    assert!(
        load.signature.contains("pub(crate) async fn load"),
        "the signature is the declaration as written: {}",
        load.signature
    );
    assert_eq!(load.line, 1, "the first line declares it");
}

/// The declarations the old list could not spell — a visibility qualifier in front
/// of a named declaration — are items, and each is listed once.
/// 旧词表拼不出来的那些声明——具名声明前有可见性限定——都是条目，且各只列一次。
#[test]
fn qualified_declarations_are_items() {
    let items = names(Some(false));
    for expected in [
        "Panel", "Mode", "Bits", "Marker", "Alias", "LIMIT", "TABLES", "nested",
    ] {
        assert_eq!(
            items.iter().filter(|name| *name == expected).count(),
            1,
            "`{expected}` must be listed exactly once: {items:?}"
        );
    }
    let panel = item_symbols(SOURCE)
        .into_iter()
        .find(|item| item.name == "Panel")
        .expect("the qualified struct is listed");
    assert_eq!(
        panel.signature, "pub(crate) struct Panel;",
        "the signature is the declaration line"
    );
}

/// A `let` binding is not an item — not even when its type is written as `fn(u32)`.
/// `let` 绑定不是条目——即使它的类型写成 `fn(u32)` 也不是。
#[test]
fn a_let_binding_is_not_an_item() {
    let items = names(None);
    for binding in ["text", "pointer"] {
        assert!(
            !items.iter().any(|name| name == binding),
            "`{binding}` is a binding, not an item: {items:?}"
        );
    }
}

/// A declaration keyword inside a comment or a string literal is prose.
/// 注释或字符串字面量里的声明关键字是散文。
#[test]
fn keywords_in_comments_and_strings_are_not_items() {
    let source = "\
// struct Ghost;\n\
/* enum Phantom { A } */\n\
fn outer() {\n\
    let quoted = \"enum Quoted { B }\";\n\
}\n";
    let items = item_symbols(source);
    let reported = items
        .iter()
        .map(|item| item.name.clone())
        .collect::<Vec<_>>();
    assert_eq!(reported, ["outer"], "{reported:?}");
}

/// Source order: a list shows the items the way a reader meets them.
/// 源码顺序：列表按读者遇到条目的顺序展示。
#[test]
fn items_are_listed_in_source_order() {
    let lines = item_symbols(SOURCE)
        .into_iter()
        .map(|item| item.line)
        .collect::<Vec<_>>();
    let mut sorted = lines.clone();
    sorted.sort_unstable();
    assert_eq!(lines, sorted, "{lines:?}");
    assert_eq!(
        lines.first().copied(),
        Some(1),
        "the first item is the first line"
    );
}
