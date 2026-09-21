//! 项目外 crate：它不参与宿主那棵生成树，靠 `external_object!` 显式声明来源，
//! 在自己的命名空间里构建一个注册机，供宿主 overlay 使用。
//! An out-of-project crate: it takes no part in the host's generated tree. It
//! declares its origin explicitly with `external_object!`, builds a registry in
//! its own namespace, and the host overlays it.

use nichlink_run_method::registry_core::{FrameworkId, Registry};

/// 必须与宿主共享同一个 framework，overlay 才接受这棵外部树。
/// Must match the host framework; `overlay` rejects a foreign tree.
pub const FRAMEWORK: FrameworkId = FrameworkId::new("nichlink.example.control-button");

pub mod button_fast;

/// 外部实现自己的注册机（项目外注册）。
/// The external implementation's own registry (out-of-project registration).
pub fn external_registry() -> Registry {
    let mut registry = Registry::root_for_namespace(FRAMEWORK, env!("CARGO_PKG_NAME"));
    registry
        .register_all(&[button_fast::REGISTRATION])
        .expect("external face registers");
    registry
}
