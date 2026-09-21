//! Static built-in topology validation.
//! 内置注册树静态拓扑校验。

use std::fs;
use std::path::Path;

use super::diagnostics::{BuildDiagnostic, BuildDiagnostics};
use super::registry_identity;
use super::types::Node;
use super::{
    SourceScope, cached_parent_id, face_source_is_active, node_id, parsed_face, relative_display,
};
use nichlink::{TopologyRecord, validate_face_topology};

#[derive(Clone, Debug)]
pub(crate) struct StaticFaceRecord {
    pub(crate) id: registry_identity::NodeId,
    pub(crate) parent: registry_identity::NodeId,
    pub(crate) owns_registry: bool,
    pub(crate) source: String,
    pub(crate) module: String,
}

pub(crate) fn source_module_path(relative: &str) -> String {
    relative
        .rsplit_once('/')
        .map_or_else(
            || relative.trim_end_matches(".rs").to_owned(),
            |(parent, _)| parent.to_owned(),
        )
        .replace('/', "::")
}

pub(crate) fn static_plan(
    src: &Path,
    nodes: &[Node],
    scope: &SourceScope,
) -> (Vec<StaticFaceRecord>, BuildDiagnostics) {
    let mut records = Vec::new();
    let mut errors = BuildDiagnostics::default();
    collect_static_faces(src, nodes, scope, false, &mut records, &mut errors);

    let mut topology = records
        .iter()
        .map(|record| TopologyRecord {
            id: record.id,
            parent: record.parent,
            owns_registry: record.owns_registry,
            source: record.source.clone(),
        })
        .collect::<Vec<_>>();
    let mut topology_errors =
        validate_face_topology(&mut topology, registry_identity::package_root_node_id());
    errors.extend(std::mem::take(&mut topology_errors));
    (records, errors)
}

fn collect_static_faces(
    src: &Path,
    nodes: &[Node],
    scope: &SourceScope,
    selected_ancestor: bool,
    records: &mut Vec<StaticFaceRecord>,
    errors: &mut BuildDiagnostics,
) {
    for node in nodes {
        if node.name == "compile_error_demo" || !scope.includes(src, node, selected_ancestor) {
            continue;
        }
        let selected_here = selected_ancestor
            || node_id(src, node).is_some_and(|id| {
                scope
                    .roots
                    .as_ref()
                    .is_some_and(|roots| roots.contains(&id))
            });
        if face_source_is_active(src, node, scope, selected_ancestor) {
            let file = node
                .file
                .as_ref()
                .expect("an active registration face has a source file");
            let relative = relative_display(src, file);
            if !relative.starts_with("registry_core/")
                && let Ok(source) = fs::read_to_string(file)
                && let Some(face) = parsed_face(&source, &relative)
                && face.field("plugin").is_none()
            {
                // A declaration the compiler drops must not keep a plan entry:
                // the plan and the compiled crate have to agree on which faces
                // exist. Only `feature` gates can be followed, because they are
                // the ones this build script can evaluate from its own
                // environment; anything else is refused instead of guessed.
                // 编译器会丢弃的声明不能留下计划条目：计划与编译产物必须对"有哪些面"
                // 取得一致。只有 `feature` 门控可以跟随——它是 build script 能从自己的
                // 环境里求值的那一种；其余一律拒绝而不是猜测。
                if let Some(cfg) = face.cfg() {
                    match face_cfg_enabled(cfg, &feature_enabled) {
                        Ok(true) => {}
                        Ok(false) => continue,
                        Err(message) => {
                            errors.push(
                                BuildDiagnostic::new("face-cfg", message)
                                    .at(relative.clone(), face.location.line),
                            );
                            continue;
                        }
                    }
                }
                let id = node_id(src, node).expect("a parsed face has an identity");
                let parent = cached_parent_id(src, &face).or_else(|| {
                    face.field("parent")
                        .is_none()
                        .then_some(registry_identity::package_root_node_id())
                });
                match parent {
                    Some(parent) => records.push(StaticFaceRecord {
                        id,
                        parent,
                        owns_registry: face.boolean("needs_registry").unwrap_or(false),
                        module: source_module_path(&relative),
                        source: relative,
                    }),
                    None => errors.push(
                        BuildDiagnostic::new(
                            "static-plan",
                            "parent declaration cannot be resolved",
                        )
                        .at(relative, 0),
                    ),
                }
            }
        }
        collect_static_faces(
            src,
            &node.children,
            scope,
            selected_ancestor || selected_here,
            records,
            errors,
        );
    }
}

/// Whether a declaration's `cfg` gate is satisfied for this build.
/// 声明的 `cfg` 门控在本次构建中是否成立。
///
/// `enabled` answers for one feature name, so the decision is testable without
/// touching the process environment.
/// `enabled` 回答单个特性名，因此该判断无需触碰进程环境即可测试。
pub(crate) fn face_cfg_enabled(cfg: &str, enabled: &dyn Fn(&str) -> bool) -> Result<bool, String> {
    let meta = syn::parse_str::<syn::Meta>(cfg)
        .map_err(|error| unsupported_cfg(cfg, &error.to_string()))?;
    evaluate_cfg(&meta, cfg, enabled)
}

fn evaluate_cfg(
    meta: &syn::Meta,
    original: &str,
    enabled: &dyn Fn(&str) -> bool,
) -> Result<bool, String> {
    if let syn::Meta::NameValue(pair) = meta
        && pair.path.is_ident("feature")
    {
        return match &pair.value {
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(name),
                ..
            }) => Ok(enabled(&name.value())),
            _ => Err(unsupported_cfg(
                original,
                "`feature` needs a string literal",
            )),
        };
    }
    if let syn::Meta::List(list) = meta
        && list.path.is_ident("not")
    {
        let inner = syn::parse2::<syn::Meta>(list.tokens.clone())
            .map_err(|error| unsupported_cfg(original, &error.to_string()))?;
        return Ok(!evaluate_cfg(&inner, original, enabled)?);
    }
    Err(unsupported_cfg(
        original,
        "only `feature = \"…\"` and `not(feature = \"…\")` can be followed",
    ))
}

fn unsupported_cfg(cfg: &str, reason: &str) -> String {
    format!(
        "unsupported `cfg` on a registration face: `{cfg}` ({reason}); the generated plan \
         cannot follow it, so the plan and the compiled crate would disagree"
    )
}

/// Whether a Cargo feature is enabled for this build script run.
/// 本次 build script 运行中某个 Cargo 特性是否启用。
fn feature_enabled(name: &str) -> bool {
    let key = format!(
        "CARGO_FEATURE_{}",
        name.to_uppercase().replace(['-', '.'], "_")
    );
    std::env::var_os(key).is_some_and(|value| !value.is_empty())
}

#[cfg(test)]
mod face_cfg_tests {
    use super::face_cfg_enabled;

    /// Feature gates are followed; anything this build cannot evaluate is
    /// refused rather than guessed.
    /// 特性门控会被跟随；本次构建无法求值的写法一律拒绝，而不是猜测。
    #[test]
    fn feature_gates_are_followed_and_others_refused() {
        let enabled = |name: &str| name == "optional-face";
        assert_eq!(
            face_cfg_enabled(r#"feature = "optional-face""#, &enabled),
            Ok(true)
        );
        assert_eq!(
            face_cfg_enabled(r#"feature = "other""#, &enabled),
            Ok(false)
        );
        assert_eq!(
            face_cfg_enabled(r#"not(feature = "other")"#, &enabled),
            Ok(true)
        );
        assert_eq!(
            face_cfg_enabled(r#"not(feature = "optional-face")"#, &enabled),
            Ok(false)
        );
        let error = face_cfg_enabled("test", &enabled).expect_err("unsupported gate");
        assert!(error.contains("unsupported"), "{error}");
    }
}
