use std::sync::Mutex;

use nichlink_run_method::{PluginAdapter, VerifiedPluginArtifact};
use wasmi::{
    Config, Engine, Instance, Linker, Memory, Module, Store, StoreLimits, StoreLimitsBuilder,
};

use crate::{HostError, PluginInstance};

/// Resource budget applied to every Wasm call.
/// 每次 Wasm 调用使用的资源预算。
#[derive(Clone, Copy, Debug)]
pub struct WasmLimits {
    pub memory_bytes: usize,
    pub fuel_per_call: u64,
    pub max_input_bytes: usize,
    pub max_output_bytes: usize,
}

impl Default for WasmLimits {
    fn default() -> Self {
        Self {
            memory_bytes: 16 * 1024 * 1024,
            fuel_per_call: 1_000_000,
            max_input_bytes: 1024 * 1024,
            max_output_bytes: 1024 * 1024,
        }
    }
}

/// Loads Wasm without WASI or host imports.
/// 在没有 WASI 和宿主导入的环境中加载 Wasm。
#[derive(Clone, Copy, Debug, Default)]
pub struct WasmBackend {
    limits: WasmLimits,
}

impl WasmBackend {
    pub const fn new(limits: WasmLimits) -> Self {
        Self { limits }
    }

    pub fn load(&self, artifact: VerifiedPluginArtifact) -> Result<WasmInstance, HostError> {
        let (_, bytes) = artifact.into_parts();
        let mut config = Config::default();
        config.consume_fuel(true);
        let engine = Engine::new(&config);
        let module = Module::new(&engine, &bytes)
            .map_err(|error| HostError::InvalidArtifact(error.to_string()))?;
        let store_limits = StoreLimitsBuilder::new()
            .memory_size(self.limits.memory_bytes)
            .instances(1)
            .memories(1)
            .tables(1)
            .trap_on_grow_failure(true)
            .build();
        let mut store = Store::new(&engine, store_limits);
        store.limiter(|limits| limits);
        store
            .set_fuel(self.limits.fuel_per_call)
            .map_err(|error| HostError::Limit(error.to_string()))?;
        let linker = Linker::new(&engine);
        let instance = linker
            .instantiate(&mut store, &module)
            .and_then(|pre| pre.start(&mut store))
            .map_err(|error| HostError::InvalidArtifact(error.to_string()))?;
        if let Ok(abi) = instance.get_typed_func::<(), i32>(&store, "nichlink_abi_version") {
            let version = abi
                .call(&mut store, ())
                .map_err(|error| HostError::Abi(error.to_string()))?;
            if version != nichlink_run_method::PLUGIN_ABI_VERSION as i32 {
                return Err(HostError::Abi(format!(
                    "plugin ABI version {version} is incompatible with host ABI {}",
                    nichlink_run_method::PLUGIN_ABI_VERSION
                )));
            }
        }
        let memory = instance
            .get_memory(&store, "memory")
            .ok_or_else(|| HostError::Abi("missing exported memory `memory`".to_owned()))?;
        instance
            .get_typed_func::<(i32, i32), i64>(&store, "nichlink_health")
            .map_err(|_| HostError::Abi("missing `nichlink_health(i32, i32) -> i64`".to_owned()))?;

        Ok(WasmInstance {
            state: Mutex::new(WasmState {
                store,
                instance,
                memory,
            }),
            limits: self.limits,
        })
    }
}

struct WasmState {
    store: Store<StoreLimits>,
    instance: Instance,
    memory: Memory,
}

/// A loaded, fuel-metered Wasm plugin.
/// 已加载并启用燃料计量的 Wasm 插件。
pub struct WasmInstance {
    state: Mutex<WasmState>,
    limits: WasmLimits,
}

impl PluginInstance for WasmInstance {
    fn adapter(&self) -> PluginAdapter {
        PluginAdapter::Wasm
    }

    fn call(&self, operation: &str, input: &[u8]) -> Result<Vec<u8>, HostError> {
        if input.len() > self.limits.max_input_bytes {
            return Err(HostError::Limit(format!(
                "input is {} bytes; limit is {}",
                input.len(),
                self.limits.max_input_bytes
            )));
        }
        let export = operation_export(operation)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| HostError::Process("Wasm store lock was poisoned".to_owned()))?;
        state
            .store
            .set_fuel(self.limits.fuel_per_call)
            .map_err(|error| HostError::Limit(error.to_string()))?;
        let memory = state.memory;
        memory
            .write(&mut state.store, 0, input)
            .map_err(|error| HostError::Abi(error.to_string()))?;
        let instance = state.instance;
        let function = instance
            .get_typed_func::<(i32, i32), i64>(&state.store, &export)
            .map_err(|_| HostError::Abi(format!("missing `{export}(i32, i32) -> i64`")))?;
        let packed = function
            .call(&mut state.store, (0, input.len() as i32))
            .map_err(|error| HostError::Process(error.to_string()))? as u64;
        let offset = (packed >> 32) as usize;
        let length = (packed & u64::from(u32::MAX)) as usize;
        if length > self.limits.max_output_bytes {
            return Err(HostError::Limit(format!(
                "output is {length} bytes; limit is {}",
                self.limits.max_output_bytes
            )));
        }
        let end = offset
            .checked_add(length)
            .ok_or_else(|| HostError::Abi("output range overflowed".to_owned()))?;
        if end > memory.data_size(&state.store) {
            return Err(HostError::Abi(format!(
                "output range {offset}..{end} is outside linear memory"
            )));
        }
        let mut output = vec![0; length];
        memory
            .read(&state.store, offset, &mut output)
            .map_err(|error| HostError::Abi(error.to_string()))?;
        Ok(output)
    }
}

fn operation_export(operation: &str) -> Result<String, HostError> {
    if nichlink_run_method::validate_operation_name(operation).is_err() {
        return Err(HostError::InvalidOperation(operation.to_owned()));
    }
    Ok(format!("nichlink_{operation}"))
}
