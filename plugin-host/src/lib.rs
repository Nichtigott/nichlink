//! Isolated execution and atomic deployment for NichLink plugins.
//! NichLink 插件的隔离执行与原子部署。

mod deployment;
mod error;
#[cfg(feature = "wasm")]
mod lazy_wasm;
#[cfg(feature = "process-tools")]
mod process;
mod verifier;
#[cfg(feature = "wasm")]
mod wasm;

pub use deployment::{Deployment, HotDeployment};
pub use error::HostError;
#[cfg(feature = "wasm")]
pub use lazy_wasm::{ValidationChannel, WasmPluginSlot, WasmPluginTable};
#[cfg(feature = "process-tools")]
pub use process::{ProcessBackend, ProcessInstance, ProcessLimits, ProcessProgram};
pub use verifier::{Ed25519Verifier, TrustedPublicKey};
#[cfg(feature = "wasm")]
pub use wasm::{WasmBackend, WasmInstance, WasmLimits};

use nichlink_run_method::PluginAdapter;

/// One callable plugin implementation.
/// 一个可调用的插件实现。
pub trait PluginInstance: Send + Sync + 'static {
    fn adapter(&self) -> PluginAdapter;
    fn call(&self, operation: &str, input: &[u8]) -> Result<Vec<u8>, HostError>;

    fn health_check(&self) -> Result<(), HostError> {
        let response = self.call("health", &[])?;
        if response == b"ok" {
            Ok(())
        } else {
            Err(HostError::Health(format!(
                "expected `ok`, received {:?}",
                String::from_utf8_lossy(&response)
            )))
        }
    }
}
