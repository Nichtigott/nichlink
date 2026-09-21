//! 运行 `cargo run -p nichlink-example-control-button --example tree` 打印示例的
//! 注册树。
//! Run `cargo run -p nichlink-example-control-button --example tree` to print the
//! example's registration tree.

fn main() {
    for row in control_button::outline() {
        println!("{row}");
    }
}
