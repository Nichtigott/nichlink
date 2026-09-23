//! Source-only Studio fixture host package.
//! 仅源码的 Studio 夹具宿主包。
//!
//! `host!()` is the one build wiring point a real host crate needs; look at
//! `examples/control-button` for the compiled analogue. The `prototype-fixtures`
//! tests never compile this crate — they only read the registration faces under
//! `src/` — but keeping the wiring here makes the fixture a faithful host.
//! `host!()` 是真实宿主 crate 唯一需要的构建接线点，可编译的同类见
//! `examples/control-button`。`prototype-fixtures` 测试从不编译本 crate，只读取 `src/`
//! 下的注册面；保留这里的接线是为了让夹具保持真实宿主的形态。

nichlink_run_method::host!();

/// 这个夹具的宿主身份。所有注册面共享同一个 framework。
/// The fixture's host identity. Every face shares this framework.
pub const FRAMEWORK: FrameworkId = FrameworkId::new("nichlink.fixture.node-editor");
