//! 运行 `cargo run -p nichlink-example-control-button --example health_check`
//! 在真实宿主面上演示 `Registry::health_check`：Button 面声明了
//! `runtime_checks: [NON_EMPTY_TEXT]`，因此空标签必须被拒绝。
//! Run `cargo run -p nichlink-example-control-button --example health_check` to
//! exercise `Registry::health_check` on a real host face: the Button face
//! declares `runtime_checks: [NON_EMPTY_TEXT]`, so a blank label must be
//! rejected.
//!
//! Why a direct "just print the tree" example would be wrong: this is the host
//! side of a documented host API, so it has to show the *boundary* call — build
//! a `RuntimeValue`, hand it to `health_check` with an empty call path (this
//! example runs no trace), and render the aggregate. A face with no declared
//! check would pass everything and prove nothing, which is why the Button face
//! really declares one.
//! 直白写法错在哪：这是有文档的宿主 API 的宿主侧，因此必须展示**边界**调用——构造
//! `RuntimeValue`、用空调用路径（本示例不接 trace）交给 `health_check`、渲染聚合错误。
//! 没有声明检查的面会对一切取值通过，什么都证明不了——所以 Button 面确实声明了一条。

use nichlink_run_method::{Provenance, RuntimeValue};

fn main() {
    let registry = control_button::base_registry();
    let node = control_button::control::object::button::NODE_ID;

    // One good label and one whitespace-only label; `call_path` is `Vec::new()`
    // because this example has no active trace.
    // 一个合格标签与一个纯空白标签；本示例没有活动 trace，因此 `call_path` 传
    // `Vec::new()`。
    for label in ["OK", "   "] {
        let value = RuntimeValue::text(
            label,
            Provenance::default().push(node, "Button", "paint", label),
        );
        match registry.health_check(node, &value, Vec::new()) {
            Ok(()) => println!("label {label:?}: accepted"),
            Err(error) => println!("label {label:?}: rejected\n{error}"),
        }
    }
}
