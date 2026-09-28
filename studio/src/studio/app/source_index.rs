//! Source indexing and registry snapshot helpers for Studio.
//! Studio 的源码索引与注册快照辅助逻辑。
//!
//! The pure Rust-source lexer lives in the kernel `source` module; this shim
//! keeps the historical paths and hosts the Studio-specific file-bound helpers.
//! 纯 Rust 源码词法器本体在 kernel 的 `source` 模块；本 shim 保留历史路径，
//! 并承载 Studio 特有的文件绑定辅助逻辑。

// Kernel lexer re-exports. Historical Studio paths stay valid through these.
// kernel 词法器重导出，Studio 历史路径经由它们保持可用。
pub use nichlink_run_method::source::{body_calls, function_source_range, function_symbols};

use super::support::{package_namespace, package_root, with_authoring_context};
use super::*;

/// Compose compiled, external, and newly authored faces into one snapshot.
/// 将已编译、外部和刚落盘的注册面装配成同一个快照。
pub(super) fn load_registry() -> Result<Registry, String> {
    // Studio owns an empty, namespace-isolated root and lets the authoring UI
    // add faces. A host can choose a stable namespace per library through the
    // environment when several libraries share one process.
    let namespace = package_namespace();
    let mut registry = Registry::root_for_namespace(
        nichlink_run_method::FrameworkId::new("nichlink.studio"),
        namespace,
    );
    // Scan only the host package's `src/` tree. When Studio is launched from
    // the NichLink workspace itself there is no host `src/`; show an empty
    // registry instead of mistaking build/debug fixtures for faces.
    // 只扫描宿主包的 `src/`。直接从 NichLink workspace 启动时没有宿主 `src/`，
    // 此时显示空注册树，不要把 build/debug 测试夹具误认成注册面。
    let source_root = package_root().join("src");
    if !source_root.is_dir() {
        return Ok(registry);
    }
    let snapshots =
        with_authoring_context(|| nichlink_run_method::generated_snapshots_from(&source_root))
            .map_err(|error| format!("cannot load generated registration faces: {error}"))?;
    registry
        .register_snapshot_batch(snapshots)
        .map_err(|error| format!("generated registration faces were rejected: {error}"))?;
    Ok(registry)
}

/// Render one admission policy as the compact clause form the Edit form carries.
/// 将一条 admission 策略渲染成 Edit 表单所携带的紧凑子句形式。
///
/// One clause per list, in the kernel's grammar (`allow:a,b`, `deny:c`,
/// `allow:a,b;deny:c`, `ANY`), and both clauses are rendered when both lists are
/// present: the deny list is the veto `Admission::accepts` gives priority to, so a
/// renderer that kept only the allow list widened the gate the Edit form shows and
/// the inspector row prints — and the widened form is what the next save wrote
/// back to the source (audit `LGC-LG-02`).
/// 每张列表一个子句，用内核的语法（`allow:a,b`、`deny:c`、`allow:a,b;deny:c`、`ANY`），
/// 且两张列表同时存在时两个子句都渲染：deny 是 `Admission::accepts` 优先采用的否决权，只保留
/// allow 列表的渲染会放宽 Edit 表单与检视器那一行显示的门禁——而下次保存写回源码的正是被放宽的
/// 那一份（审计 `LGC-LG-02`）。
///
/// Why this renders the kernel's grammar instead of calling the kernel's
/// `render_admission`: that function renders the **source expression**
/// (`crate::Admission::new(&[…], &[…])`), while this value is parsed with
/// `parse_admission_owned`, which reads only the compact spelling
/// (`run_method/src/authoring/manifest/face/face.rs`); the kernel exposes no public
/// renderer for the compact form. The two directions are therefore pinned against
/// each other by the tests below, and a `debug_assert` asks the kernel parser about
/// every value this function emits.
/// 这里渲染内核语法、而不是调用内核的 `render_admission`，原因是后者渲染的是**源码表达式**
/// （`crate::Admission::new(&[…], &[…])`），而本值会被 `parse_admission_owned` 解析，它只读紧凑
/// 拼法（`run_method/src/authoring/manifest/face/face.rs`）；内核没有公开的紧凑形式渲染器。因此
/// 两个方向由下面的测试互钉，另有 `debug_assert` 把本函数产出的每个值交给内核解析器裁决。
pub(crate) fn admission_text(admission: &nichlink_run_method::OwnedAdmission) -> String {
    let clauses = [
        ("allow", &admission.allowed_paths),
        ("deny", &admission.denied_paths),
    ]
    .into_iter()
    .filter(|(_, paths)| !paths.is_empty())
    .map(|(key, paths)| format!("{key}:{}", paths.join(",")))
    .collect::<Vec<_>>();
    let text = if clauses.is_empty() {
        "ANY".to_owned()
    } else {
        clauses.join(";")
    };
    debug_assert!(
        nichlink_run_method::authoring::parse::parse_admission_owned(&text).is_ok(),
        "the rendered value must be one the kernel's compact parser reads: {text}"
    );
    text
}

/// Render one registry rule as compact `preset:...;parts:...` clauses.
/// 将一条注册规则渲染成紧凑的 `preset:...;parts:...` 子句。
pub(crate) fn registration_rule_text(rule: &nichlink_run_method::OwnedRegistrationRule) -> String {
    let mut clauses = Vec::new();
    if let Some(preset) = &rule.required_preset {
        clauses.push(format!("preset:{preset}"));
    }
    if !rule.required_parts.is_empty() {
        clauses.push(format!("parts:{}", rule.required_parts.join(",")));
    }
    if !rule.required_exports.is_empty() {
        clauses.push(format!("exports:{}", rule.required_exports.join(",")));
    }
    if !rule.required_handle_traits.is_empty() {
        clauses.push(format!("handle:{}", rule.required_handle_traits.join(",")));
    }
    if !rule.required_part_traits.is_empty() {
        clauses.push(format!(
            "part_trait:{}",
            rule.required_part_traits.join(",")
        ));
    }
    if clauses.is_empty() {
        "ANY".to_owned()
    } else {
        clauses.join(";")
    }
}

/// Locate the 1-based declaration line of a function in a registry source file.
/// 在注册面源码文件中定位函数声明的 1 起始行号。
pub(crate) fn function_line(file: &str, function: &str) -> Option<u32> {
    let text = std::fs::read_to_string(source_path_for(file)).ok()?;
    function_symbols(&text)
        .into_iter()
        .find(|item| item.name == function)
        .map(|item| item.line)
}

#[cfg(test)]
pub(super) fn function_bodies(source: &str) -> Vec<(String, String)> {
    function_symbols(source)
        .into_iter()
        .map(|function| (function.name, function.body))
        .collect()
}

#[cfg(test)]
mod admission_text_tests {
    //! The compact admission rendering, pinned against the kernel's parser (audit `LGC-LG-02`).
    //! 紧凑 admission 渲染，与内核解析器互相钉住（审计 `LGC-LG-02`）。

    use super::*;
    use nichlink_run_method::authoring::parse::parse_admission_owned;

    /// A policy that names both lists must render both: the deny list is the veto
    /// `Admission::accepts` prioritises, so a rendering that keeps only the allow
    /// list widens the gate the reader sees and the next save writes back.
    /// 同时点名两张列表的策略必须把两张都渲染出来：deny 是 `Admission::accepts` 优先的否决权，
    /// 只保留 allow 列表的渲染会放宽读者看到的、以及下次保存写回的那道门禁。
    #[test]
    fn both_lists_survive_the_compact_rendering() {
        let policy = nichlink_run_method::OwnedAdmission {
            allowed_paths: vec!["ui".to_owned()],
            denied_paths: vec!["ui/experimental".to_owned()],
        };
        let text = admission_text(&policy);
        assert_eq!(text, "allow:ui;deny:ui/experimental", "{text}");
        let read = parse_admission_owned(&text).expect("the kernel reads what this renderer emits");
        assert_eq!(read.allowed_paths, ["ui"], "{read:?}");
        assert_eq!(read.denied_paths, ["ui/experimental"], "{read:?}");
    }

    /// The two directions are pinned against each other, byte for byte: the
    /// canonical compact spellings the kernel parses must be exactly what this
    /// renderer emits for the policy they parse into, and every value it emits
    /// must parse. The kernel's refusals are the other half of the same grammar.
    /// 两个方向逐字节互钉：内核能解析的规范紧凑拼法，必须正是本渲染器对同一策略的输出；而它输出的
    /// 每个值都必须可解析。同一套语法的另一半是内核的拒绝行为。
    #[test]
    fn the_kernel_reads_back_every_value_this_renderer_emits() {
        for canonical in ["ANY", "allow:a,b", "deny:c", "allow:a,b;deny:c"] {
            let policy = parse_admission_owned(canonical).expect("canonical spelling");
            assert_eq!(
                admission_text(&policy),
                canonical,
                "the renderer must reproduce the kernel's canonical spelling"
            );
        }

        let matrix = [
            (Vec::new(), Vec::new()),
            (vec!["ui".to_owned()], Vec::new()),
            (Vec::new(), vec!["ui/experimental".to_owned()]),
            (vec!["ui".to_owned()], vec!["ui/experimental".to_owned()]),
            (
                vec!["ui".to_owned(), "control".to_owned()],
                vec!["ui/experimental".to_owned()],
            ),
        ];
        for (allowed_paths, denied_paths) in matrix {
            let policy = nichlink_run_method::OwnedAdmission {
                allowed_paths,
                denied_paths,
            };
            let text = admission_text(&policy);
            assert!(
                parse_admission_owned(&text).is_ok(),
                "the kernel refuses what this renderer emits: {text:?}"
            );
        }

        for malformed in [
            "allow:a;allow:b",
            "allow:a;veto:b",
            "allow:a;deny:",
            "allow:a;b",
            "allow:",
        ] {
            assert!(
                parse_admission_owned(malformed).is_err(),
                "the grammar must refuse `{malformed}`"
            );
        }
    }
}
