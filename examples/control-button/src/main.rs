//! 宿主入口。graft 计划按设计写在入口，构建期会把它固化成 `StaticPlan`。
//! Host entry. The graft plan belongs here by design; the build step captures it
//! into the `StaticPlan`.

// `cut A graft X` 只替换逻辑槽位 A，A 原来的子节点继续接在新实现下面。
// `cut A graft X` replaces the logical slot A only; A's children stay attached.
control_button::static_graft_plan!(
    control_button::FRAMEWORK,
    cut "root/control/button" graft "button_fast",
);

fn main() {
    for row in control_button::outline() {
        println!("{row}");
    }
}
