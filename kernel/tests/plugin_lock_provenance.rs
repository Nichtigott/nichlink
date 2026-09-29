//! A ten-field plugin-lock line is not the seven-field form with extra pipes.
//! 十字段插件锁记录不是多出几根竖线的七字段形式。
//!
//! `X-1` / `LGC-LG-40`: a ten-field record whose provenance columns (fields 8, 9,
//! 10 — signature, key fingerprint, revocation snapshot) were empty used to be
//! read as "no signature / no fingerprint / no revocation", byte-identical in
//! meaning to the seven-field form that never mentioned them. The three columns
//! were silently dropped, while every *other* empty column in the same grammar is
//! refused: an empty `source` is `unknown plugin source` and an empty identity
//! field is `contains an empty field`. This file pins the chosen semantics —
//! **the empty column is recorded, not dropped** — from outside the crate, so the
//! distinction a consumer sees (`Some("")` against `None`) and the admission
//! outcome that follows from it are both checkable without touching kernel
//! internals.
//! `X-1` / `LGC-LG-40`：来源字段（第 8、9、10 列——签名、密钥指纹、撤销快照）为空的十字段记录
//! 过去被读成"无签名/无指纹/无撤销"，含义上与从未提到它们的七字段形式逐字节等同。这三列被静默
//! 丢掉，而同一套语法里**其它**空列一律被拒：空 `source` 是 `unknown plugin source`，空身份字段
//! 是 `contains an empty field`。本文件从 crate 之外钉住选定的语义——**空列被记录，而不是被丢掉**——
//! 因此消费方能看到的区分（`Some("")` 对 `None`）以及由此得到的准入结局都可在不碰内核内部的前提下
//! 检查。
//!
//! Why recording rather than refusing: refusing the spelling outright turns the
//! ten-field artifact a host may already hold into an unreadable lock, and this
//! crate cannot decide that for every writer. Recording keeps the artifact
//! readable while making the empty column an *assertion* the trust rule honours
//! instead of a value nobody can observe.
//! 为什么是记录而不是拒绝：直接拒绝这种拼法会把宿主手上可能已有的十字段工件变成读不出的锁，而本
//! crate 不能替所有写入方做这个决定。记录让工件仍然可读，同时把空列变成信任规则会遵守的**声明**，
//! 而不是一个没人看得见的值。

use nichlink_kernel::declaration::{FrameworkId, PluginManifest, PluginMode, PluginSource};
use nichlink_kernel::plugin::PluginCatalog;

/// The seven-field official record a host's own plugin UI writes.
/// 宿主自己的插件界面写下的七字段官方记录。
const SEVEN_FIELD: &str = "official|com.nichui.editor|canvas|1.0.0|canvas|sha256:a|extension\n";

/// The same record in the ten-field form with all three provenance columns empty.
/// 同一条记录的十字段形式，三个来源列全为空。
const TEN_FIELD_EMPTY_PROVENANCE: &str =
    "official|com.nichui.editor|canvas|1.0.0|canvas|sha256:a|extension|||\n";

/// One official manifest for the identity both locks above name.
/// 两条锁都点名的那个身份对应的一份官方 manifest。
fn manifest(
    signature: Option<&'static str>,
    fingerprint: Option<&'static str>,
    revocation: Option<&'static str>,
) -> PluginManifest {
    PluginManifest {
        name: "canvas",
        crate_name: "canvas",
        version: "1.0.0",
        framework: FrameworkId::new("com.nichui.editor"),
        source: PluginSource::Official,
        mode: PluginMode::Extension,
        checksum: "sha256:a",
        signature,
        public_key_fingerprint: fingerprint,
        revocation_list: revocation,
    }
}

/// An empty provenance column keeps its own value instead of collapsing into the
/// seven-field form's "not mentioned".
/// 空的来源列保留自己的值，而不是塌缩成七字段形式的"没提过"。
#[test]
fn an_empty_provenance_column_is_not_the_seven_field_form() {
    let seven =
        PluginCatalog::parse_plugin_catalog(SEVEN_FIELD).expect("the seven-field lock parses");
    let ten = PluginCatalog::parse_plugin_catalog(TEN_FIELD_EMPTY_PROVENANCE)
        .expect("the ten-field lock parses");
    let (seven, ten) = (&seven.records()[0], &ten.records()[0]);

    for unnamed in [
        &seven.signature,
        &seven.public_key_fingerprint,
        &seven.revocation_list,
    ] {
        assert_eq!(*unnamed, None, "the seven-field form declares no column");
    }
    // Field 8, 9 and 10 were spelled, and each one is an empty value — not the
    // absence of a value. Before `X-1` all three read `None` here.
    // 第 8、9、10 列被写出来了，每一列都是一个空值——而不是没有值。修 `X-1` 之前这三处都是 `None`。
    for declared_empty in [
        &ten.signature,
        &ten.public_key_fingerprint,
        &ten.revocation_list,
    ] {
        assert_eq!(
            *declared_empty,
            Some(String::new()),
            "an empty column must survive as an explicit empty value"
        );
    }
    assert_ne!(
        seven, ten,
        "the two spellings must not be the same record once the columns are read"
    );

    // A partly filled ten-field line keeps each column separate.
    // 部分填写的十字段行把每一列分开保留。
    let partial = PluginCatalog::parse_plugin_catalog(
        "official|com.nichui.editor|canvas|1.0.0|canvas|sha256:a|extension||key-v1|official-2026\n",
    )
    .expect("a partly filled ten-field lock parses");
    let partial = &partial.records()[0];
    assert_eq!(partial.signature, Some(String::new()));
    assert_eq!(partial.public_key_fingerprint, Some("key-v1".to_owned()));
    assert_eq!(partial.revocation_list, Some("official-2026".to_owned()));
}

/// The recorded empty column is an assertion the trust rule honours: a lock that
/// says "no signature" no longer admits a manifest that names one.
/// 记录下来的空列是信任规则会遵守的声明：说"没有签名"的锁不再放行一份点了签名的 manifest。
#[test]
fn an_explicitly_empty_provenance_column_pins_absence() {
    let seven =
        PluginCatalog::parse_plugin_catalog(SEVEN_FIELD).expect("the seven-field lock parses");
    let ten = PluginCatalog::parse_plugin_catalog(TEN_FIELD_EMPTY_PROVENANCE)
        .expect("the ten-field lock parses");
    let unsigned = manifest(None, None, None);
    let signed = manifest(Some("sig-v1"), Some("key-v1"), Some("official-2026"));

    assert!(
        seven.contains_manifest(signed),
        "the seven-field form leaves the three provenance fields to the signature check"
    );
    assert!(seven.contains_manifest(unsigned));
    assert!(ten.contains_manifest(unsigned), "{ten:?}");

    // The whole point: these two spellings used to answer alike.
    // 全部要点：这两种拼法过去给出同一个答案。
    assert!(
        !ten.contains_manifest(signed),
        "an empty signature column is a declaration that there is no signature"
    );

    let partial = PluginCatalog::parse_plugin_catalog(
        "official|com.nichui.editor|canvas|1.0.0|canvas|sha256:a|extension||key-v1|official-2026\n",
    )
    .expect("a partly filled ten-field lock parses");
    assert!(partial.contains_manifest(manifest(None, Some("key-v1"), Some("official-2026"))));
    assert!(
        !partial.contains_manifest(signed),
        "the empty column pins that one field while the filled ones pin theirs"
    );
}

/// The neighbouring empty columns are still refused, which is the contrast that
/// makes the ten-field reading a bug and not a house style.
/// 相邻的空列仍然被拒绝——正是这一对比说明十字段的读法是一个缺陷，而不是本仓的风格。
#[test]
fn the_other_empty_columns_are_still_refused() {
    let empty_source = PluginCatalog::parse_plugin_catalog(
        "|com.nichui.editor|canvas|1.0.0|canvas|sha256:a|extension\n",
    )
    .expect_err("an empty source names no trust lane")
    .to_string();
    assert!(
        empty_source.contains("unknown plugin source"),
        "{empty_source}"
    );

    let empty_identity =
        PluginCatalog::parse_plugin_catalog("official||canvas|1.0.0|canvas|sha256:a|extension\n")
            .expect_err("an empty identity field names nothing")
            .to_string();
    assert!(
        empty_identity.contains("contains an empty field"),
        "{empty_identity}"
    );
}
