//! Pins for the facade's expression rewriting.
//! facade 表达式改写的钉子。

use super::{names_a_compiled_module, owned};
use crate::build_time::diagnostics::BuildDiagnostics;
use crate::build_time::registry_syntax::GraftSyntax;
use crate::build_time::renderer::ShapeRender;
use crate::build_time::renderer::test_support::temporary_directory;
use crate::build_time::static_plan::StaticFaceRecord;
use crate::build_time::{SourceScope, render_lib};
use nichlink_kernel::registry_core::identity::NodeId;
use nichlink_kernel::registry_core::syntax::{GraftExpressions, SyntaxLocation};

/// Facade mode names the crate that compiles each module; every other mode leaves expressions exactly
/// as the author wrote them (audit `M7`, §M7.33).
/// facade 模式点出编译每个模块的那个 crate；其余模式让表达式保持作者写下的样子（审计 `M7`，§M7.33）。
#[test]
fn facade_mode_names_the_crate_that_compiles_each_module() {
    let owners = [
        ("panel::frame".to_owned(), "fix_widgets".to_owned()),
        ("control".to_owned(), "fix".to_owned()),
    ];
    let facade = ShapeRender {
        cut_out: &[],
        only: None,
        mounts: &[],
        ancestors: &[],
        facade: true,
        owners: &owners,
    };
    // A typed graft expression loses its `crate::` and gains its owner. The longest matching owner
    // wins, so a face under a claim goes to the ghost that claims it.
    // 类型化切口表达式丢掉 `crate::`、换上属主。最长匹配者胜，因此认领之下的面归认领它的幽灵。
    assert_eq!(
        owned(&facade, "crate::panel::frame::widget::NODE_ID"),
        "fix_widgets::panel::frame::widget::NODE_ID"
    );
    // A bare module path — how the registration-rules block writes them — gains the same prefix.
    // 裸模块路径——注册规则块就是这么写的——加同一个前缀。
    assert_eq!(
        owned(&facade, "panel::frame::REGISTRATION"),
        "fix_widgets::panel::frame::REGISTRATION"
    );
    // A **registration rule** spells `<module>::REGISTRATION.registry_rule`: the whole expression has to
    // be matched against the owner table, because splitting at the last `::` yields
    // `panel::frame::REGISTRATION`, an owner nobody has (measured in the facade, §M7.38).
    // **注册规则**写的是 `<模块>::REGISTRATION.registry_rule`：整条表达式都要拿去匹配属主表，因为按最后一个
    // `::` 切会得到 `panel::frame::REGISTRATION`——一个谁都不是的属主（在 facade 里实测，§M7.38）。
    assert_eq!(
        owned(&facade, "panel::frame::REGISTRATION.registry_rule"),
        "fix_widgets::panel::frame::REGISTRATION.registry_rule"
    );
    assert_eq!(
        owned(&facade, "crate::panel::frame::widget::NODE_ID"),
        "fix_widgets::panel::frame::widget::NODE_ID"
    );

    // An external implementation crate is not part of this tree: it passes through untouched.
    // 项目外的实现 crate 不属于这棵树：原样通过。
    assert_eq!(
        owned(&facade, "control_button_graft::button_fast::NODE_ID"),
        "control_button_graft::button_fast::NODE_ID"
    );
    // A host module is owned by the host, because `crate::` inside the facade means the facade.
    // 宿主模块归宿主，因为 facade 里的 `crate::` 指的是 facade 自己。
    assert_eq!(
        owned(&facade, "crate::control::NODE_ID"),
        "fix::control::NODE_ID"
    );

    // Any other mode is the author's spelling, byte for byte.
    // 其余模式一律是作者写下的拼写，逐字节不变。
    let host = ShapeRender::whole();
    for expression in [
        "crate::panel::frame::widget::NODE_ID",
        "panel::frame::REGISTRATION",
        "control_button_graft::button_fast::NODE_ID",
    ] {
        assert_eq!(owned(&host, expression), expression);
    }
}

/// The face this fixture's graft cuts, as the plan records it.
/// 本夹具的 graft 所切的那个面，按规划记录的样子。
fn widget() -> StaticFaceRecord {
    StaticFaceRecord {
        id: NodeId::from_bytes(b"widget"),
        parent: NodeId::from_bytes(b"frame"),
        owns_registry: false,
        source: "panel/frame/widget/widget.rs".to_owned(),
        module: "panel::frame::widget".to_owned(),
    }
}

/// A typed graft entry is emitted only by a crate that compiles its cut face (audit `M7`, §M7.39).
/// 类型化 graft 条目只由编译其切口面的 crate 发射（审计 `M7`，§M7.39）。
///
/// The negative half is what a partition needs: the host hands `panel::frame` away, so its own
/// generated tree must not name `panel::frame::widget` any more — not in the modules, and not in the
/// graft table, which is the half that first end-to-end build with a graft inside a claim missed.
/// 否定的那一半正是分区需要的：宿主把 `panel::frame` 交出去，因此它自己的生成树不能再点名
/// `panel::frame::widget`——模块里不能，graft 表里也不能，而后者正是"含切口落在认领之内的首次端到端构建"
/// 漏掉的那一半。
#[test]
fn a_crate_emits_only_the_graft_entries_it_compiles() {
    let root = temporary_directory("graft-visibility");
    let graft = GraftSyntax {
        cfg: None,
        cut: "crate::panel::frame::widget::NODE_ID".to_owned(),
        cut_end: None,
        graft: "fast_widget::fast::NODE_ID".to_owned(),
        full: false,
        location: SyntaxLocation { line: 7, column: 8 },
        expressions: Some(GraftExpressions {
            cut: "crate::panel::frame::widget::NODE_ID".to_owned(),
            cut_end: None,
            graft: "fast_widget::fast::NODE_ID".to_owned(),
        }),
    };
    let entries = std::slice::from_ref(&graft);
    let render = |faces: &[StaticFaceRecord]| {
        render_lib(
            &root,
            &[],
            &BuildDiagnostics::default(),
            &BuildDiagnostics::default(),
            &SourceScope {
                roots: None,
                reason: "test",
            },
            faces,
            entries,
            ShapeRender::whole(),
        )
    };

    // The crate that compiles the cut face emits the entry the author wrote.
    // 编译切口面的 crate 发射作者写下的那条条目。
    let host = render(&[widget()]);
    assert!(
        host.contains(
            "StaticGraftCut::from_ids(crate::panel::frame::widget::NODE_ID, \
             fast_widget::fast::NODE_ID, false)"
        ),
        "{host}"
    );
    assert!(
        host.contains("assert_contract::<crate::panel::frame::widget::__Preset"),
        "{host}"
    );

    // The crate that handed that face away emits neither the entry nor its contract assertion — the
    // table is left empty rather than left naming a module this crate does not have.
    // 把那个面交出去的 crate 既不发射条目、也不发射契约断言——表留空，而不是留着一个本 crate 没有的模块名。
    let elsewhere = render(&[]);
    assert!(
        elsewhere.contains("BUILTIN_GRAFT_CUTS: &[::nichlink_toolchain::runtime::registry_core::StaticGraftCut] = &[\n];"),
        "{elsewhere}"
    );
    assert!(
        !elsewhere.contains("panel::frame::widget"),
        "a cut face this crate does not compile must not be named here: {elsewhere}"
    );

    // A selector the parser did not resolve to a module names nothing to check, so it stays data.
    // 解析器没有解析成模块的选择器没有可查的东西，因此保持为数据。
    assert!(names_a_compiled_module(&[], "root/control/button"));
    assert!(names_a_compiled_module(
        &[widget()],
        "crate::panel::frame::widget"
    ));
    assert!(!names_a_compiled_module(
        &[],
        "crate::panel::frame::widget::NODE_ID"
    ));
    std::fs::remove_dir_all(root).expect("temporary fixture cleanup");
}
