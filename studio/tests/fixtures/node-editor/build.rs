//! Fixture host build entry, mirroring `examples/control-button`.
//! 夹具宿主的构建入口，与 `examples/control-button` 保持一致。
//!
//! This script is never run for the `prototype-fixtures` tests: Studio reads the
//! source tree as text and does not compile the fixture.
//! `prototype-fixtures` 测试从不运行本脚本：Studio 把源码树当文本读取，不编译夹具。

fn main() {
    nichlink_build_method::run();
}
