//! The plugin catalog's own tests, in their own file so the catalog stays inside the
//! line ceiling.
//! 插件目录自己的测试，放在独立文件里，使目录本体留在行数上限之内。

use super::*;

#[test]
fn plugin_lock_parser_keeps_official_and_user_records_typed() {
    let catalog = PluginCatalog::parse(
            "# source|framework|package|version|crate|checksum|mode|signature|key|revocations\n\
             official|com.nichui.editor|canvas|1.0.0|canvas|sha256:a|replacement|sig-v1|key-v1|official-2026\n\
             user|com.nichui.editor|local|0.1.0|local_canvas|sha256:b|extension\n",
        )
        .expect("lock should parse");
    assert_eq!(catalog.records().len(), 2);
    assert_eq!(catalog.records()[0].source, PluginSource::Official);
    assert_eq!(catalog.records()[0].signature.as_deref(), Some("sig-v1"));
    assert_eq!(
        catalog.records()[0].revocation_list.as_deref(),
        Some("official-2026")
    );
    assert_eq!(catalog.records()[1].mode, PluginMode::Extension);
}

#[test]
fn plugin_lock_parser_rejects_ambiguous_duplicate_identity() {
    let lock = "official|com.nichui.editor|canvas|1.0.0|canvas|sha256:a|extension\n\
                    official|com.nichui.editor|canvas|1.0.0|canvas|sha256:b|extension\n";
    let error = PluginCatalog::parse(lock).unwrap_err().to_string();
    assert!(error.contains("line 2"));
    assert!(error.contains("duplicates package identity"));
}

/// A writer asks the parser before it writes: a second record reusing the
/// identity five-tuple is refused with the parser's own reason, and a record
/// the parser accepts comes back as the text to write.
/// 写入方在写之前先问解析器：复用身份五元组的第二条记录被解析器自己的理由拒绝，而解析器
/// 接受的记录作为待写文本交回。
#[test]
fn appending_a_duplicate_identity_is_refused_by_the_parser() {
    let seed = "user|com.nichui.editor|canvas|1.0.0|canvas|sha256:a|extension\n";
    let duplicate = "user|com.nichui.editor|canvas|1.0.0|canvas|sha256:b|extension";
    let error = PluginCatalog::with_appended_line(seed, duplicate)
        .expect_err("a duplicate identity must not be appended");
    assert!(
        error.to_string().contains("duplicates package identity"),
        "{error}"
    );

    let fresh = "user|com.nichui.editor|panel|1.0.0|panel|sha256:b|extension";
    let text = PluginCatalog::with_appended_line(seed, fresh).expect("a fresh record appends");
    assert_eq!(
        PluginCatalog::parse(&text)
            .expect("the text handed back parses")
            .records()
            .len(),
        2
    );
}

/// The separator belongs to the parser, not to the writer: a lock whose last
/// line carries no terminator gains one instead of gluing two records into a
/// line that is neither.
/// 分隔符属于解析器而不属于写入方：末行没有终止符的锁会补上一个换行，而不是把两条记录粘成
/// 一条两者都不是的行。
#[test]
fn appending_completes_a_missing_line_terminator() {
    let seed = "user|com.nichui.editor|canvas|1.0.0|canvas|sha256:a|extension";
    let line = "user|com.nichui.editor|panel|1.0.0|panel|sha256:b|extension";
    let text = PluginCatalog::with_appended_line(seed, line).expect("the text parses");
    assert!(text.ends_with('\n'), "{text:?}");
    assert_eq!(
        PluginCatalog::parse(&text)
            .expect("two records")
            .records()
            .len(),
        2
    );
}

/// The seven-field record leaves the three provenance fields to the
/// signature check, and a record that carries one pins it.
/// 七字段记录把三个来源字段交给签名校验；携带某个值的记录则把它钉住。
fn manifest() -> PluginManifest {
    PluginManifest {
        name: "canvas",
        crate_name: "canvas",
        version: "1.0.0",
        framework: crate::FrameworkId::new("com.nichui.editor"),
        source: PluginSource::Official,
        mode: PluginMode::Extension,
        checksum: "sha256:a",
        signature: Some("sig-v1"),
        public_key_fingerprint: Some("key-v1"),
        revocation_list: Some("official-2026"),
    }
}

#[test]
fn a_ten_field_record_accounts_for_a_seven_field_lock() {
    let bare =
        PluginCatalog::parse("official|com.nichui.editor|canvas|1.0.0|canvas|sha256:a|extension\n")
            .expect("a seven-field lock parses");
    let pinned = PluginCatalog::parse(
            "official|com.nichui.editor|canvas|1.0.0|canvas|sha256:a|extension|sig-v1|key-v1|official-2026\n",
        )
        .expect("a ten-field lock parses");
    let candidate = pinned.records()[0].clone();

    assert!(
        !bare.contains(&candidate),
        "an identical-record rule refuses the write — the old writer's answer"
    );
    assert!(
        bare.contains_record(&candidate),
        "the runtime's rule, asked about the same candidate record, accepts it"
    );
    assert!(
        bare.contains_manifest(manifest()),
        "and the manifest spelling of the same plugin agrees"
    );

    // The expectation still binds in the other direction: a lock that pins a signature does
    // not account for a record written without it.
    // 期望在另一个方向上仍然生效：钉住了签名的锁，不覆盖一条没写签名的记录。
    let other = PluginCatalog::parse(
            "official|com.nichui.editor|canvas|1.0.0|canvas|sha256:a|extension|sig-v2|key-v1|official-2026\n",
        )
        .expect("a ten-field lock parses");
    assert!(
        !other.contains_record(&bare.records()[0].clone()),
        "a record the lock pins differently must still be refused"
    );
}

#[test]
fn a_record_without_provenance_fields_does_not_pin_them() {
    let bare =
        PluginCatalog::parse("official|com.nichui.editor|canvas|1.0.0|canvas|sha256:a|extension\n")
            .expect("a seven-field lock parses");
    assert!(
        bare.contains_manifest(manifest()),
        "a record the host's own UI wrote must match a signed official manifest"
    );

    let pinned = PluginCatalog::parse(
            "official|com.nichui.editor|canvas|1.0.0|canvas|sha256:a|extension|sig-v1|key-v1|official-2026\n",
        )
        .expect("a ten-field lock parses");
    assert!(pinned.contains_manifest(manifest()));

    let other = PluginCatalog::parse(
            "official|com.nichui.editor|canvas|1.0.0|canvas|sha256:a|extension|sig-v2|key-v1|official-2026\n",
        )
        .expect("a ten-field lock parses");
    assert!(
        !other.contains_manifest(manifest()),
        "a record that names a signature must pin it"
    );
}

#[test]
fn plugin_lock_parser_rejects_unknown_identity_schema() {
    let error = PluginCatalog::parse(
        "# nichlink-schema=2\n\
             user|com.nichui.editor|local|0.1.0|local_canvas|sha256:b|extension\n",
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("identity schema 2"));
    assert!(error.contains("expected v3"));
}

/// The canonical spelling is `v3`; the bare `3` written before the `v`
/// prefix existed stays readable, and nothing else does.
/// 规范写法是 `v3`；加 `v` 前缀之前写下的裸 `3` 仍可读，别的写法都不行。
#[test]
fn plugin_lock_parser_accepts_canonical_and_legacy_schema_spellings() {
    for schema in ["v3", "3"] {
        let lock = format!(
            "# nichlink-schema={schema}\n\
                 user|com.nichui.editor|local|0.1.0|local_canvas|sha256:b|extension\n"
        );
        assert!(
            PluginCatalog::parse(&lock).is_ok(),
            "schema `{schema}` must stay readable"
        );
    }
    for schema in ["v4", "V3", "vv3", ""] {
        let lock = format!(
            "# nichlink-schema={schema}\n\
                 user|com.nichui.editor|local|0.1.0|local_canvas|sha256:b|extension\n"
        );
        assert!(
            PluginCatalog::parse(&lock).is_err(),
            "schema `{schema}` must be refused"
        );
    }
}

/// One digest, two accepted spellings: the lock still accounts for the manifest.
/// 同一份摘要的两种可接受拼法：锁仍然覆盖清单。
///
/// Red before the fix: the comparison was a literal string equality, so a lock written as
/// `sha256:a` did not account for a record written as `a` (and vice versa), and the write
/// was refused with `LockMismatch` — "the lock does not have this plugin" — even though
/// `PluginManifest::verify_bytes` accepts both spellings of the same digest
/// (audit `LGC-LG-40`).
/// 修前为红：比较是字面字符串相等，因此写成 `sha256:a` 的锁不覆盖写成 `a` 的记录（反之亦然），
/// 写入被 `LockMismatch`——"锁里没有这个插件"——拒绝，即使 `PluginManifest::verify_bytes` 接受
/// 同一份摘要的两种拼法（审计 `LGC-LG-40`）。
#[test]
fn a_checksum_is_compared_by_its_digest_not_its_spelling() {
    let lock = |checksum: &str| {
        PluginCatalog::parse(&format!(
            "official|com.nichui.editor|canvas|1.0.0|canvas|{checksum}|extension\n"
        ))
        .expect("a lock parses")
        .records()[0]
            .clone()
    };
    let bare = lock("a");
    assert!(
        PluginCatalog::parse("official|com.nichui.editor|canvas|1.0.0|canvas|sha256:a|extension\n")
            .expect("a prefixed lock parses")
            .contains_record(&bare),
        "the coverage rule reads the `sha256:` prefix either way"
    );
    assert!(
        PluginCatalog::parse("official|com.nichui.editor|canvas|1.0.0|canvas|A|extension\n")
            .expect("an upper-case lock parses")
            .contains_record(&bare),
        "the coverage rule reads the hex case-insensitively, like `is_sha256`"
    );
    assert!(
        !PluginCatalog::parse("official|com.nichui.editor|canvas|1.0.0|canvas|b|extension\n")
            .expect("another digest parses")
            .contains_record(&bare),
        "a different digest is still a mismatch"
    );
    // The identical-record rule is deliberately stricter — it is how a duplicate line is
    // refused — so it keeps comparing the recorded spelling verbatim.
    // 同记录规则有意更严——重复行就是靠它拒绝的——因此它继续逐字比较记录的拼法。
    assert!(
        !PluginCatalog::parse(
            "official|com.nichui.editor|canvas|1.0.0|canvas|sha256:a|extension\n"
        )
        .expect("a prefixed lock parses")
        .contains(&bare),
        "the duplicate-identity rule is exact"
    );
}
