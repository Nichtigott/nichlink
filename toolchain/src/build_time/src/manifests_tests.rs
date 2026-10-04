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
    let nodes = crate::build_time::source_walk::discover_root(&src);
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
    let nodes = crate::build_time::source_walk::discover_root(&src);
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
    let nodes = crate::build_time::source_walk::discover_root(&src);
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

/// The record carries the three facts audit `W3-1` added, and a derivation agrees with them.
/// 记录带着审计 `W3-1` 新增的三项事实，而一次推导与它们一致。
///
/// The acceptance is an **对账**: the columns are only worth publishing if a reader that derives
/// the same facts by reading the sources gets the same answers. So the pin recomputes each column
/// from the file with the same kernel functions a reader would use, and compares — a record that
/// disagreed with the derivation would be worse than no record, because the readers would trust it.
/// 验收是一次**对账**：这些列只在"自己读源码推导出同样事实的读者得到同样答案"时才值得发布。因此钉子用
/// 读者会用的同一批内核函数从文件重算每一列并比对——与推导不一致的记录比没有记录更糟，因为读者会信它。
#[test]
fn the_record_carries_the_source_hash_fields_and_calls_a_derivation_agrees_with() {
    let (_root, src) = package(
        "record-facts",
        "pub fn paint(&self) -> i32 { helper() }\nfn helper() -> i32 { 7 }\n",
    );
    let nodes = crate::build_time::source_walk::discover_root(&src);
    let out = src.join("out");
    fs::create_dir_all(&out).expect("out dir");
    super::write_pruning_manifest(&src, &nodes, &out).expect("the manifest writes");
    let record = fs::read_to_string(out.join("pruning_manifest.tsv")).expect("the record");
    let header = record.lines().next().unwrap_or_default();
    assert_eq!(
        header,
        "# node\tsource\tsymbol\tpath\tkind\tregistry_name\tparent\tsource_hash\tfields\tcalls",
        "the columns are the record's contract"
    );

    let text = fs::read_to_string(src.join("dial/dial.rs")).expect("the face source");
    let expected_hash = nichlink_kernel::sha256_hex(text.as_bytes());
    let mut expected_calls: Vec<String> = nichlink_kernel::source::function_symbols(&text)
        .into_iter()
        .flat_map(|function| nichlink_kernel::source::direct_calls(&function.body, &function.name))
        .collect();
    expected_calls.sort();
    expected_calls.dedup();

    let row = record
        .lines()
        .find(|line| line.contains("dial/dial.rs"))
        .expect("a row for the face");
    let columns = row.split('\t').collect::<Vec<_>>();
    assert_eq!(columns.len(), 10, "ten columns: {row}");
    assert_eq!(
        columns[7], expected_hash,
        "the source hash is the file's own sha256: {row}"
    );
    assert!(
        columns[9].contains("helper"),
        "the calls column names what the file calls: {row}"
    );
    assert_eq!(
        columns[9],
        expected_calls.join(","),
        "and it is the kernel's own direct-call rule, sorted: {row}"
    );
    // The fingerprint is a hash, so the **check** that it is well-formed is its length and its
    // stability across two runs — a fingerprint that moved between identical builds would make
    // every reader fall back to deriving, silently.
    // 指纹是散列，因此对它的**检查**是长度与"两次运行之间稳定"——在两次相同构建之间会变的指纹，会让每个
    // 读者静默地回落到推导。
    assert_eq!(
        columns[8].len(),
        64,
        "the field fingerprint is a sha256: {row}"
    );
    let before = columns[8].to_owned();
    super::write_pruning_manifest(&src, &nodes, &out).expect("the manifest writes again");
    let again = fs::read_to_string(out.join("pruning_manifest.tsv")).expect("the record");
    assert!(
        again.contains(&before),
        "and the same sources produce the same fingerprint"
    );
}

/// Every column the writer spells is one the reader reads back (audit `W3-1`).
/// 写入方拼出的每一列，读取方都读得回来（审计 `W3-1`）。
///
/// The rule this pins is the repo's own: a text contract has **one** renderer and **one** parser,
/// and a round trip is what proves they agree. Without it, three new columns would be write-only —
/// the writer would publish facts no reader could reach, which is the "default surface lags the
/// capability surface" defect in its other direction.
/// 这条钉子守的是本仓自己的规矩：一份文本契约**一份**渲染方与**一份**解析方，而回环是证明两者一致的
/// 东西。没有它，三个新列就是"只写不读"——写入方发布的事实没有任何读者够得着，正是"承诺面落后于能力面"
/// 那个缺陷的另一个方向。
#[test]
fn every_published_column_reads_back() {
    let (root, src) = package(
        "round-trip",
        "pub fn paint(&self) -> i32 { helper() }\nfn helper() -> i32 { 7 }\n",
    );
    let out = root.join("out");
    fs::create_dir_all(&out).expect("out directory");
    let nodes = crate::build_time::source_walk::discover_root(&src);
    crate::build_time::manifests::write_pruning_manifest(&src, &nodes, &out)
        .expect("the manifest writes");

    let rows = crate::build_time::read_pruning_manifest(&out).expect("the record reads");
    let row = rows
        .iter()
        .find(|row| row.source == "dial/dial.rs")
        .expect("a row for the face");
    assert_eq!(
        row.kind.as_deref(),
        Some("Dial"),
        "the declared kind reads back: {row:?}"
    );
    let hash = row
        .source_hash
        .as_deref()
        .expect("the source hash reads back");
    assert_eq!(hash.len(), 64, "and it is a sha256: {hash}");
    let text = fs::read_to_string(src.join("dial/dial.rs")).expect("the source");
    assert_eq!(
        hash,
        nichlink_kernel::sha256_hex(text.as_bytes()),
        "and it is the hash of the bytes the build was looking at"
    );
    assert_eq!(
        row.fields.as_deref().map(str::len),
        Some(64),
        "the field fingerprint reads back: {row:?}"
    );
    assert_eq!(
        row.calls.as_deref(),
        Some("helper"),
        "the direct calls read back: {row:?}"
    );

    // A record from before these columns still parses: the older three-column form leaves every
    // optional column `None` rather than failing, because a reader that would rather derive than
    // trust a published value has to survive an old record.
    // 早于这些列的记录照样解析：较早的三列形式让每个可选列为 `None` 而不是失败，因为"宁可推导也不采信
    // 已发布值"的读者必须能活过一份旧记录。
    fs::write(
        out.join("pruning_manifest.tsv"),
        "# node\tsource\tsymbol\nbdb4427ce81c9bc51e56bee7667fd2be\tcontrol/control.rs\t-\n",
    )
    .expect("an old record");
    let old = crate::build_time::read_pruning_manifest(&out).expect("an old record reads");
    assert_eq!(old.len(), 1, "the old form still parses: {old:?}");
    assert!(
        old[0].source_hash.is_none() && old[0].calls.is_none(),
        "and its absent columns are `None`, not an error: {old:?}"
    );
    let _ = fs::remove_dir_all(&root);
}
