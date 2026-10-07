//! Pins for the facade's expression rewriting.
//! facade 表达式改写的钉子。

use super::owned;
use crate::build_time::renderer::ShapeRender;

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
