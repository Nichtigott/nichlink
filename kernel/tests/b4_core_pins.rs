//! Pins for the B4 `core/` findings (`docs/audit-2026-09-28/audit-findings.json`).
//! B4 `core/` 条目的钉子（`docs/audit-2026-09-28/audit-findings.json`）。
//!
//! One pin per fixed item, asserted through the crate's public surface so the pin says
//! what a consumer sees, not what a private helper happens to do.
//! 每条被修的条目一枚钉子，都经 crate 的公开面断言，因此钉子说的是消费方看到的东西，而不是某个
//! 私有助手凑巧做了什么。

use std::collections::BTreeMap;

#[cfg(feature = "syntax")]
use nichlink_kernel::authoring::snapshot::snapshot_from_values;
#[cfg(feature = "syntax")]
use nichlink_kernel::declaration::RuntimeCheckSpec;
use nichlink_kernel::plugin::PluginCatalog;

/// The value map `snapshot_from_values` reads, with the fields a face always carries.
/// `snapshot_from_values` 读取的取值表，含注册面总会带着的那些字段。
#[cfg(feature = "syntax")]
fn values(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    let mut values: BTreeMap<String, String> = [
        ("source", "probe.rs"),
        ("kind", "Probe"),
        ("module", "probe"),
        ("parent_node", "00000000000000000000000000000001"),
        ("needs_registry", "false"),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_owned(), value.to_owned()))
    .collect();
    for (key, value) in pairs {
        values.insert((*key).to_owned(), (*value).to_owned());
    }
    values
}

/// `LGC-LG-04`: the lock schema gate has to hold for the whole file, not for the record
/// the parser happens to be looking at. An empty lock that declares an unknown schema, a
/// header written after the records, a header with a typo, and a repeated header are all
/// refusals; only the canonical single header (or no header at all) parses.
/// `LGC-LG-04`：锁的 schema 门禁要对整份文件成立，而不是对解析器正好在看的那条记录成立。声明了
/// 未知 schema 的空锁、写在记录之后的表头、拼错的表头、重复的表头都是拒绝；只有规范的单个表头
/// （或根本没有表头）才解析。
#[test]
fn a_plugin_lock_schema_gate_is_not_line_order_dependent() {
    assert!(
        PluginCatalog::parse("# nichlink-schema=v999\n").is_err(),
        "an empty lock that declares an unknown schema must be refused"
    );
    assert!(
        PluginCatalog::parse("official|f|p|1.0.0|c|sha256:00|extension\n# nichlink-schema=v999\n")
            .is_err(),
        "a header written after the records still gates them"
    );
    assert!(
        PluginCatalog::parse("# nichlink-schema v999\n").is_err(),
        "a misspelled header must not silently disable the gate"
    );
    assert!(
        PluginCatalog::parse("# nichlink-schema=v3\n# nichlink-schema=v3\n").is_err(),
        "a second header is a duplicate, not a silent overwrite"
    );
    assert!(
        PluginCatalog::parse("# nichlink-schema=v3\n").is_ok(),
        "the canonical header parses"
    );
    assert!(
        PluginCatalog::parse("official|f|p|1.0.0|c|sha256:00|extension\n").is_ok(),
        "a lock that declares no schema keeps parsing"
    );
}

/// `LGC-LG-27`: the author's `runtime_checks` edit survives the compile-time/author merge,
/// exactly as the author's `flow` edit does.
/// `LGC-LG-27`：作者改过的 `runtime_checks` 会像作者改过的 `flow` 一样活过"编译期 + 作者快照"
/// 的合并。
#[cfg(feature = "syntax")]
#[test]
fn merging_an_authored_snapshot_keeps_the_authored_runtime_checks() {
    let compiled = snapshot_from_values(&values(&[("runtime_checks", "finite_number")]), "ns")
        .expect("the compiled-side snapshot builds");
    let authored = snapshot_from_values(
        &values(&[("runtime_checks", "coordinates_in_viewport")]),
        "ns",
    )
    .expect("the author-side snapshot builds");
    assert_eq!(
        compiled.runtime_checks,
        vec![RuntimeCheckSpec::FiniteNumber],
        "the fixture's compiled side carries its own checks"
    );
    let merged = compiled.merge_authored(authored);
    assert_eq!(
        merged.runtime_checks,
        vec![RuntimeCheckSpec::CoordinatesInViewport],
        "the author's edit to runtime_checks must not be silently dropped"
    );
}

/// `LGC-LG-28`: the snapshot parser refuses a malformed `requires` entry instead of
/// dropping it and going on — the same verdict the strict validator already gives.
/// `LGC-LG-28`：快照解析器拒绝畸形 `requires` 条目，而不是丢掉它继续走——与严格校验器给出的
/// 结论相同。
#[cfg(feature = "syntax")]
#[test]
fn a_malformed_requires_entry_is_refused_not_dropped() {
    let malformed = values(&[("requires", "a=>b,broken")]);
    let error = snapshot_from_values(&malformed, "ns")
        .expect_err("a `requires` entry without `=>` must be refused, not dropped");
    assert!(
        error.contains("requires"),
        "the refusal names the field: {error}"
    );
    let wellformed = values(&[("requires", "a=>b,c=>d")]);
    assert!(
        snapshot_from_values(&wellformed, "ns").is_ok(),
        "a well-formed `requires` list still parses"
    );
}

/// `LGC-LG-28` (repair): the *strict* entry refuses a malformed `requires` entry, and the
/// published entry keeps the lossy shape callers already compile against — `-> Vec<…>`, dropping
/// the malformed item instead of reporting it. Both are pinned, in that order, because the
/// repair's whole point is that the refusing semantics came from a *new* sibling entry rather
/// than from changing a released signature.
/// `LGC-LG-28`（修复）：**严格**入口拒绝畸形 `requires` 条目，而已发布的入口保留调用方已编译对齐的
/// 有损形状——`-> Vec<…>`，丢掉畸形条目而不是报出它。两条都被钉住，因为这次修复的全部要点在于：拒绝
/// 语义来自**新增的兄弟入口**，而不是来自改动一个已发布的签名。
#[cfg(feature = "syntax")]
#[test]
fn the_strict_requires_entry_refuses_a_malformed_entry() {
    use nichlink_kernel::authoring::parse::try_parse_requirements_owned;
    let error = try_parse_requirements_owned("a=>b,broken")
        .expect_err("the strict entry refuses a `requires` entry without `=>`");
    assert!(
        error.to_string().contains("capability=>provider"),
        "the refusal names the expected shape: {error}"
    );
    assert_eq!(
        try_parse_requirements_owned("a=>b,c=>d")
            .expect("a well-formed list parses")
            .len(),
        2
    );
}

/// The published entry still exists with its released signature and keeps its documented lossy
/// behaviour: a malformed entry is dropped and the well-formed rest is returned. It does *not*
/// report an error, and a short list is not proof that the source was well-formed.
/// 已发布的入口仍然存在、签名与发布时一致，并保持其文档化的有损行为：畸形条目被丢掉，合规的其余
/// 条目照常返回。它**不会**报错，而一份更短的列表也不是"源文件合规"的证明。
#[cfg(feature = "syntax")]
#[test]
fn the_published_requires_entry_keeps_its_lossy_released_shape() {
    use nichlink_kernel::authoring::parse::parse_requirements_owned;
    let requirements: Vec<_> = parse_requirements_owned("a=>b,broken,c=>d");
    assert_eq!(
        requirements.len(),
        2,
        "the published entry drops the malformed item and keeps the rest: {requirements:?}"
    );
    assert!(
        requirements.iter().any(|entry| entry.capability == "a")
            && requirements.iter().any(|entry| entry.capability == "c"),
        "the well-formed entries survive: {requirements:?}"
    );
}

/// The documented semantics of both entries, read from the source: the published one says it
/// drops entries and must not claim to refuse them, and the strict sibling carries the refusing
/// wording. A doc string is public surface — saying "reports an error" about a lossy `-> Vec<…>`
/// is exactly the drift this pin exists to stop.
/// 两个入口的文档语义，读自源码：已发布那个写明它会丢条目，且不得声称它会拒绝；严格兄弟承载拒绝的
/// 措辞。文档字符串是公开面——对一个有损的 `-> Vec<…>` 说"会报错"，正是本钉子要拦下的漂移。
#[test]
fn the_requires_entries_document_which_one_is_lossy() {
    let source = include_str!("../src/registry_core/authoring/parse/rules.rs");
    let doc_block = |marker: &str| -> String {
        let at = source
            .find(marker)
            .unwrap_or_else(|| panic!("`{marker}` is declared in this file"));
        source[..at]
            .lines()
            .rev()
            .take_while(|line| line.trim_start().starts_with("///"))
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n")
    };
    let published = doc_block("pub fn parse_requirements_owned").to_ascii_lowercase();
    assert!(
        published.contains("lossy") || published.contains("drops"),
        "the published entry's doc must say it is lossy: {published}"
    );
    assert!(
        !published.contains("refuses"),
        "the published entry's doc must not claim it refuses anything: {published}"
    );
    let strict = doc_block("pub fn try_parse_requirements_owned").to_ascii_lowercase();
    assert!(
        strict.contains("strict") && strict.contains("fail"),
        "the strict sibling's doc must say it fails on a bad entry: {strict}"
    );
}

/// `LGC-LG-30`: a type path carrying generic arguments derives its trait labels. The
/// generic list's own comma is not a separator between paths.
/// `LGC-LG-30`：带泛型实参的类型路径能派生出它的 trait 标签；泛型表里的逗号不是路径之间的分隔符。
#[cfg(feature = "syntax")]
#[test]
fn a_generic_argument_with_a_comma_still_derives_trait_labels() {
    let labels = nichlink_kernel::authoring::parse::trait_names_from_paths(
        "crate::ui::ControlHandle<u8, u16>, crate::parts::ActionParts",
    )
    .expect("a type path with generic arguments derives its labels");
    assert!(
        labels.contains("ControlHandle") && labels.contains("ActionParts"),
        "both paths contribute, generic arguments and all: {labels}"
    );
}

/// `KRN-C-02`: the `CallSite` doc block describes a call *site*, not the call *edge* whose
/// sentence it used to repeat verbatim. A doc string is public surface, and `#![warn(missing_docs)]`
/// makes docs.rs its main reader.
/// `KRN-C-02`：`CallSite` 的文档块描述的是调用**点**，不再是它过去逐字复制的调用**边**那句。
/// 文档字符串是公开面的一部分，而 `#![warn(missing_docs)]` 让 docs.rs 成为它的主要读者。
#[test]
fn the_call_site_doc_describes_a_call_site() {
    let source = include_str!("../src/registry_core/declaration/call_evidence.rs");
    let declaration = source
        .find("pub struct CallSite")
        .expect("`CallSite` is declared in this file");
    let documentation = &source[declaration.saturating_sub(400)..declaration];
    assert!(
        !documentation.to_ascii_lowercase().contains("call edge"),
        "the `CallSite` doc block must not describe a call edge: {documentation}"
    );
}

/// `t84`: `merge_authored` follows **one** rule, and a field that was unconditional stays
/// unconditional.
/// `t84`：`merge_authored` 只遵循**一条**规则，而原本无条件的字段仍然无条件。
///
/// The rule now lives in one place (`declaration/owned.rs`'s `Declared` trait plus the sentence on
/// `merge_authored`): an author-side value is applied only when the file declared it. `summary` has
/// no such marker, so the author side wins even when it is silent — this pin is here to keep a
/// later cleanup from "helpfully" turning an unconditional field into a declared one.
/// 规则现在只有一处（`declaration/owned.rs` 的 `Declared` trait 与 `merge_authored` 上那一句）：
/// 作者侧取值仅在该文件声明过时应用。`summary` 没有这类标记，因此作者侧即使沉默也生效——本钉子用来
/// 挡住后来的清理"顺手"把无条件字段改成"声明过才应用"。
#[cfg(feature = "syntax")]
#[test]
fn an_unconditional_field_stays_unconditional_in_the_merge() {
    let compiled = snapshot_from_values(&values(&[("summary_en", "compiled summary")]), "ns")
        .expect("the compiled-side snapshot builds");
    let authored = snapshot_from_values(&values(&[] as &[(&str, &str)]), "ns")
        .expect("the author-side snapshot builds");
    assert_eq!(compiled.summary.en, "compiled summary");
    let merged = compiled.merge_authored(authored);
    assert!(
        merged.summary.en.is_empty(),
        "the author side is silent and still wins: `summary` is applied unconditionally, \
         got {:?}",
        merged.summary.en
    );
}

/// `t84`: the conditional fields answer the same rule, so they call the same place.
/// `t84`：条件字段回答同一条规则，因此它们调用同一处。
///
/// A silent author side keeps the compiled value for each of them, and a declared author side wins
/// for each of them — the two halves of "declared wins".
/// 作者侧沉默时它们各自保留编译期取值，作者侧声明过时它们各自优先——"声明过才应用"的两半。
#[cfg(feature = "syntax")]
#[test]
fn the_declared_rule_is_one_place_for_the_conditional_fields() {
    let compiled = snapshot_from_values(
        &values(&[
            ("flow_provider", "path::To::CompiledFlow"),
            ("runtime_checks", "finite_number"),
        ]),
        "ns",
    )
    .expect("the compiled-side snapshot builds");
    let silent = compiled.clone().merge_authored(
        snapshot_from_values(&values(&[] as &[(&str, &str)]), "ns")
            .expect("the author-side snapshot builds"),
    );
    assert_eq!(
        silent.flow_provider.as_deref(),
        Some("path::To::CompiledFlow"),
        "a silent author side keeps the compiled provider"
    );
    assert_eq!(
        silent.runtime_checks,
        vec![RuntimeCheckSpec::FiniteNumber],
        "a silent author side keeps the compiled checks"
    );
    let declared = compiled.merge_authored(
        snapshot_from_values(
            &values(&[
                ("flow_provider", "path::To::AuthoredFlow"),
                ("runtime_checks", "coordinates_in_viewport"),
            ]),
            "ns",
        )
        .expect("the author-side snapshot builds"),
    );
    assert_eq!(
        declared.flow_provider.as_deref(),
        Some("path::To::AuthoredFlow"),
        "a declared provider wins"
    );
    assert_eq!(
        declared.runtime_checks,
        vec![RuntimeCheckSpec::CoordinatesInViewport],
        "declared checks win"
    );
}
