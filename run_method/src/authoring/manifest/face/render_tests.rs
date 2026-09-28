//! Tests for the face renderer: the field set it writes back, and the fields it
//! refuses to drop.
//! 注册面渲染器的测试：它写回的字段集合，以及它拒绝丢掉的字段。

#[cfg(test)]
mod field_truth_tests {
    use super::super::{NON_FIELD_KEYS, RENDERED_FIELDS};
    use crate::authoring::manifest::parse;
    use std::path::PathBuf;

    /// The accepted set is the rendered set: every field an edit accepts comes
    /// back after render → parse, except where the round trip *canonicalizes* the
    /// spelling (named per row).
    /// 接受集就是渲染集：每个被 edit 接受的字段在 渲染 → 解析 之后都能读回，除了往返会**规范化**
    /// 拼写的那一个（逐行点名）。
    ///
    /// This is the test the audit found missing: the idempotency pin built its
    /// field table with `BTreeMap::from([...])` and went straight to
    /// `from_values → as_patch`, so it never parsed a file, applied an edit, or
    /// rendered — "the fields parse produced can survive a rewrite" was untested,
    /// and three accepted fields could not survive one at all (audit `LG-38`).
    /// 这正是审计指出缺失的那条测试：幂等钉子用 `BTreeMap::from([...])` 手搓字段表、直接走
    /// `from_values → as_patch`，因此它从未解析文件、应用编辑或渲染——"解析产生的字段能否活过
    /// 一次重写"根本没被验过，而有三个被接受的字段根本活不过（审计 `LG-38`）。
    #[test]
    fn the_round_trip_keeps_every_editable_field() {
        let root = unique_root("roundtrip");
        let face_path = root.join("widget/widget.rs");
        std::fs::create_dir_all(face_path.parent().expect("fixture parent")).expect("fixture dir");
        std::fs::write(
            &face_path,
            "// generated-by=NichLink\ncrate::root_object! {\n    kind: Widget,\n    parent: crate::ROOT_NODE_ID,\n}\n",
        )
        .expect("fixture face");

        // Every field `edit` accepts, with a value that survives the round trip.
        // Two rows keep the fixture's initial value (`registration_rule` and
        // `admission`, both `ANY`): a custom rule needs its sibling rule file, and a
        // non-`ANY` admission is a different spelling rather than a different
        // field, so their rows are the weak half of this pin — and `admission`'s
        // non-default rendering is covered by the refusal pin below plus
        // `the_keys_no_template_emits_are_refused_not_stored`'s counterpart in the
        // operations tests.
        // `runtime_checks` is the one canonicalization: the compact
        // `NON_EMPTY_TEXT` is written back as the expression
        // `crate::NON_EMPTY_TEXT`, and parsing that back yields the expression.
        // `edit` 接受的每个字段，取值都能活过往返。`runtime_checks` 是唯一的规范化：紧凑的
        // `NON_EMPTY_TEXT` 被写成表达式 `crate::NON_EMPTY_TEXT`，再解析回来得到的是表达式形式。
        // The count is deliberately not written here: the field *set* comes from
        // `RENDERED_FIELDS` and the assertion below compares the two, so a number in
        // this line would be a third declaration of the set.
        // 这里有意不写数量：字段**集合**来自 `RENDERED_FIELDS`，下面的断言会比对两者，因此在这一行
        // 写数字就等于对同一个集合的第三次声明。
        let table: &[(&str, &str, &str)] = &[
            ("kind", "Gadget", "Gadget"),
            ("preset", "crate::ProbePreset", "crate::ProbePreset"),
            ("parts", "crate::ProbeParts", "crate::ProbeParts"),
            ("name_zh", "组件", "组件"),
            ("name_en", "Widget", "Widget"),
            ("summary_zh", "摘要", "摘要"),
            ("summary_en", "summary", "summary"),
            ("stable_name", "widget-stable", "widget-stable"),
            ("exports", "widget", "widget"),
            ("provides", "canvas", "canvas"),
            ("handle_traits", "fast", "fast"),
            ("handle_contracts", "crate::Foo::Bar", "crate::Foo::Bar"),
            ("part_traits", "slow", "slow"),
            ("part_contracts", "crate::Foo::Baz", "crate::Foo::Baz"),
            ("requires", "canvas=>Canvas", "canvas=>Canvas"),
            ("runtime_checks", "NON_EMPTY_TEXT", "crate::NON_EMPTY_TEXT"),
            ("needs_registry", "true", "true"),
            ("registration_rule", "ANY", "ANY"),
            ("admission", "ANY", "ANY"),
            (
                "flow",
                "canvas|1|CanvasFrame|CanvasFrame",
                "canvas|1|CanvasFrame|CanvasFrame",
            ),
            (
                "flow_provider",
                "crate::flow::Render",
                "crate::flow::Render",
            ),
            (
                "getting_from_other_registry",
                "Some(\"engine\")",
                "Some(\"engine\")",
            ),
        ];

        // The rows are samples; the *set* of fields is not this table's to
        // declare. It comes from `RENDERED_FIELDS`, and the two have to match, or
        // the renderer and its evidence are two half-lists again (audit
        // `LGC-LG-38`).
        // 这些行是样本；字段**集合**不是本表该声明的东西。它来自 `RENDERED_FIELDS`，两者必须一致，
        // 否则渲染器与它的证据又是一份半的清单（审计 `LGC-LG-38`）。
        let mut sampled = table.iter().map(|(field, _, _)| *field).collect::<Vec<_>>();
        sampled.sort_unstable();
        let mut declared = RENDERED_FIELDS.to_vec();
        declared.sort_unstable();
        assert_eq!(
            sampled, declared,
            "the sample table must cover exactly the rendered field set"
        );
        for key in NON_FIELD_KEYS {
            assert!(
                !RENDERED_FIELDS.contains(key),
                "`{key}` is an input or metadata key, not an on-disk field"
            );
            assert!(
                !sampled.contains(key),
                "`{key}` must not be sampled as an editable field"
            );
        }

        // Two passes, because the trait labels and the contracts they name are
        // one rendered field each: with a contract present the declaration
        // *derives* the labels from it, so a manifest that sets both would read
        // the labels back from the contracts rather than from the row.
        // 分两遍，因为 trait 标签与它们点名的契约各自是同一处渲染：有契约在场时，声明会从它**派生**
        // 标签，因此同时设两者的清单会把标签读成契约派生的结果，而不是本行的取值。
        let without_contracts: Vec<_> = table
            .iter()
            .copied()
            .filter(|(field, _, _)| !field.ends_with("_contracts"))
            .collect();
        let without_traits: Vec<_> = table
            .iter()
            .copied()
            .filter(|(field, _, _)| !field.ends_with("_traits"))
            .collect();

        for (label, rows) in [
            ("without contracts", &without_contracts),
            ("without traits", &without_traits),
        ] {
            let root = unique_root(&format!("roundtrip-{}", label.replace(' ', "-")));
            let face_path = root.join("widget/widget.rs");
            std::fs::create_dir_all(face_path.parent().expect("fixture parent"))
                .expect("fixture dir");
            std::fs::write(
                &face_path,
                "// generated-by=NichLink\ncrate::root_object! {\n    kind: Widget,\n    parent: crate::ROOT_NODE_ID,\n}\n",
            )
            .expect("fixture face");

            let mut manifest = parse::source(&face_path).expect("the fixture parses");
            for (field, value, _) in rows.iter().copied() {
                manifest
                    .edit(field, value)
                    .unwrap_or_else(|error| panic!("{label}: {field} is not editable: {error}"));
            }
            let rendered = manifest.render_source().expect("the face renders");
            std::fs::write(&face_path, &rendered).expect("write the rendered face");
            let second = parse::source(&face_path).expect("the rendered face parses again");
            for (field, value, expected) in rows.iter().copied() {
                let back = second
                    .values
                    .get(field)
                    .map(String::as_str)
                    .unwrap_or("<absent>");
                assert_eq!(
                    back, expected,
                    "{label}: {field} was set to {value} and read back as {back}"
                );
            }
            let _ = std::fs::remove_dir_all(&root);
        }
    }

    /// The three keys that were accepted while nothing rendered them are now
    /// refused by name, so an edit fails loudly instead of being dropped.
    /// 三个"被接受却无人渲染"的键现在按名被拒，因此编辑会响亮地失败，而不是被丢掉。
    ///
    /// Red before the fix: each of these returned `Ok` from `edit` and changed
    /// nothing that a later `render_source` would write.
    /// 修前为红：这三个键的 `edit` 都返回 `Ok`，且不会改变任何 `render_source` 会写出的东西。
    #[test]
    fn the_keys_no_template_emits_are_refused_not_stored() {
        let root = unique_root("refused");
        let face_path = root.join("widget/widget.rs");
        std::fs::create_dir_all(face_path.parent().expect("fixture parent")).expect("fixture dir");
        std::fs::write(
            &face_path,
            "// generated-by=NichLink\ncrate::root_object! {\n    kind: Widget,\n    parent: crate::ROOT_NODE_ID,\n}\n",
        )
        .expect("fixture face");
        let mut manifest = parse::source(&face_path).expect("the fixture parses");
        for field in ["registry_name", "handle", "params"] {
            let error = manifest
                .edit(field, "anything")
                .expect_err("a key no template emits is not editable");
            assert!(
                error.contains("is not editable"),
                "{field} must be refused as not editable: {error}"
            );
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A `requires` entry the declaration cannot reproduce refuses the rewrite
    /// instead of being dropped by it.
    /// 声明复现不了的 `requires` 条目会让这次重写失败，而不是被它丢掉。
    ///
    /// `edit` validates this field, so the value is set the way a hand-written or
    /// older declaration reaches the renderer: `parse` stores the list as text and
    /// only the caller above decides what to make of it. The published
    /// `render_requirements` is lossy (it drops the entry), which is exactly what a
    /// rewrite must not do: the result goes back into the author's file.
    /// `edit` 会校验这个字段，因此这里按"手写或旧版本声明到达渲染器"的方式设值：`parse` 把列表按文本
    /// 存下，只由上面那个调用方决定怎么处理它。已发布的 `render_requirements` 是有损的（它会丢掉该
    /// 条目），而这正是重写绝不能做的事：结果要写回作者的文件。
    #[test]
    fn a_malformed_requires_entry_refuses_the_rewrite() {
        let root = unique_root("malformed-requires");
        let face_path = root.join("widget/widget.rs");
        std::fs::create_dir_all(face_path.parent().expect("fixture parent")).expect("fixture dir");
        std::fs::write(
            &face_path,
            "// generated-by=NichLink\ncrate::root_object! {\n    kind: Widget,\n    parent: crate::ROOT_NODE_ID,\n}\n",
        )
        .expect("fixture face");
        let mut manifest = parse::source(&face_path).expect("the fixture parses");
        manifest
            .values
            .insert("requires".to_owned(), "canvas=>Canvas, broken".to_owned());

        let error = manifest
            .render_source()
            .expect_err("a malformed `requires` entry must refuse the rewrite");
        assert!(
            error.contains("capability=>provider"),
            "the refusal names the shape it wanted: {error}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A directory no other test shares.
    /// 一个没有别的测试共用的目录。
    fn unique_root(label: &str) -> PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "nichlink-lg38-{label}-{}-{sequence}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        root
    }
}

#[cfg(test)]
mod plugin_preservation_tests {
    use crate::authoring::manifest::parse;

    /// Rebuilding a declaration must never delete a field it cannot reproduce:
    /// a face carrying `plugin:` refuses the rewrite instead of losing it.
    /// 重建声明绝不能删掉自己无法复现的字段：带 `plugin:` 的面拒绝重写，而不是把它
    /// 弄丢。
    #[test]
    fn a_face_with_a_plugin_refuses_a_silent_rewrite() {
        let root = std::env::temp_dir().join("nichlink-plugin-face-fixture");
        let face = root.join("widget/widget.rs");
        std::fs::create_dir_all(face.parent().expect("fixture parent")).expect("fixture dir");
        std::fs::write(
            &face,
            "// generated-by=NichLink\n\
             crate::root_object! {\n\
                 kind: Widget,\n\
                 parent: crate::ROOT_NODE_ID,\n\
                 plugin: crate::PluginSpec::new(\"widget\"),\n\
             }\n",
        )
        .expect("fixture face");

        let manifest = parse::source(&face).expect("face manifest");
        let error = manifest
            .render_source()
            .expect_err("a plugin field must refuse the rewrite")
            .to_owned();
        assert!(error.contains("plugin:"), "{error}");
        let _ = std::fs::remove_dir_all(&root);
    }
}
