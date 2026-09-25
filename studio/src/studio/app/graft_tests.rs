//! External-graft screen tests.
//! 外部 graft 界面测试。
//!
//! A separate page so the module under test stays inside the size ratchet: a test
//! module is excluded from it, and this one was the larger half of the file.
//! 独立一页，使被测模块留在尺寸棘轮之内：测试模块不受棘轮约束，而这一份是文件里更大的
//! 那一半。

use super::*;

fn declared(cut: DeclaredGraft) -> DeclaredGrafts {
    DeclaredGrafts {
        entry: PathBuf::from("src/lib.rs"),
        cuts: vec![cut],
    }
}

fn string_cut(path: &str) -> DeclaredGraft {
    DeclaredGraft {
        cut: path.to_owned(),
        cut_end: None,
        graft: "button_fast".to_owned(),
        full: false,
        cfg: None,
        expressions: None,
        line: 4,
    }
}

#[test]
fn a_string_cut_names_the_logical_path() {
    let declared = declared(string_cut("root/control/button"));
    assert_eq!(
        declaration_for(
            "root/control/button",
            Some("control::object::button"),
            &declared
        ),
        GraftDeclaration::Declared {
            expression: "cut \"root/control/button\" graft \"button_fast\"".to_owned(),
            line: 4,
            cfg: None,
        }
    );
    assert!(matches!(
        declaration_for(
            "root/control/slider",
            Some("control::object::slider"),
            &declared
        ),
        GraftDeclaration::Absent { .. }
    ));
}

#[test]
fn a_typed_cut_names_the_module_from_the_crate_root() {
    let declared = declared(DeclaredGraft {
        cut: "crate::control::object::button::NODE_ID".to_owned(),
        cut_end: None,
        graft: "control_button_graft::button_fast::NODE_ID".to_owned(),
        full: true,
        cfg: None,
        expressions: Some(nichlink_build_method::DeclaredGraftExpressions {
            cut: "crate::control::object::button::NODE_ID".to_owned(),
            cut_end: None,
            graft: "control_button_graft::button_fast::NODE_ID".to_owned(),
        }),
        line: 7,
    });
    assert_eq!(
            declaration_for(
                "root/control/button",
                Some("control::object::button"),
                &declared
            ),
            GraftDeclaration::Declared {
                expression:
                    "cut(crate::control::object::button::NODE_ID) full graft(control_button_graft::button_fast::NODE_ID)"
                        .to_owned(),
                line: 7,
                cfg: None,
            }
        );
    // The build strips `crate::` before comparing; so does this check.
    assert!(matches!(
        declaration_for("root/control/button", Some("other::module"), &declared),
        GraftDeclaration::Absent { .. }
    ));
}

#[test]
fn a_range_cut_names_both_endpoints() {
    let mut range = string_cut("root/a");
    range.cut_end = Some("root/c".to_owned());
    let declared = declared(range);
    for path in ["root/a", "root/c"] {
        assert!(matches!(
            declaration_for(path, Some("a"), &declared),
            GraftDeclaration::Declared { .. }
        ));
    }
    assert!(matches!(
        declaration_for("root/b", Some("b"), &declared),
        GraftDeclaration::Absent { .. }
    ));
    // The rendered clause keeps the endpoints as two literals, so pasting it
    // back declares a range instead of one path named `root/a to root/c`.
    // 渲染出的子句把端点保持为两个字面量，粘回去声明的是区间，而不是一条名为
    // `root/a to root/c` 的路径。
    assert_eq!(
        declaration_for("root/a", Some("a"), &declared),
        GraftDeclaration::Declared {
            expression: "cut [\"root/a\" to \"root/c\"] graft \"button_fast\"".to_owned(),
            line: 4,
            cfg: None,
        }
    );
}

/// A typed cut cannot match once the target module cannot be resolved.
/// 目标模块解析不出来时，类型化切口不再匹配。
#[test]
fn an_unresolved_module_only_matches_a_string_cut() {
    let typed = declared(DeclaredGraft {
        cut: "crate::control::object::button::NODE_ID".to_owned(),
        cut_end: None,
        graft: "control_button_graft::button_fast::NODE_ID".to_owned(),
        full: false,
        cfg: None,
        expressions: Some(nichlink_build_method::DeclaredGraftExpressions {
            cut: "crate::control::object::button::NODE_ID".to_owned(),
            cut_end: None,
            graft: "control_button_graft::button_fast::NODE_ID".to_owned(),
        }),
        line: 7,
    });
    assert!(matches!(
        declaration_for("root/control/button", None, &typed),
        GraftDeclaration::Absent { .. }
    ));
    let text = declared(string_cut("root/control/button"));
    assert!(matches!(
        declaration_for("root/control/button", None, &text),
        GraftDeclaration::Declared { .. }
    ));
}

#[test]
fn selectors_that_escape_the_plan_directory_are_refused() {
    assert!(graft_selector_error("button_fast").is_none());
    assert!(graft_selector_error(" ").is_some());
    assert!(graft_selector_error("a/b").is_some());
    assert!(graft_selector_error("a\\b").is_some());
}
