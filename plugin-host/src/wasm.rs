//! Fuel-metered, import-free Wasm plugin loading and invocation.
//! 启用燃料计量、无导入的 Wasm 插件加载与调用。

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
    /// Linear-memory ceiling per instance, in bytes; growth beyond it traps.
    /// 每个实例的线性内存上限（字节）；超出即触发 trap。
    pub memory_bytes: usize,
    /// Fuel granted to each call; an exhausted call traps.
    /// 每次调用授予的燃料；燃料耗尽时调用触发 trap。
    ///
    /// The unit belongs to the engine, not to this host: it is a bound on how
    /// much work a call may do, not a promise that a given number buys a given
    /// amount of work. A major engine upgrade can re-scale it, so a host that
    /// tuned this value against an older engine should re-measure rather than
    /// assume the same number still fits.
    /// 该单位属于引擎而不属于本宿主：它是"一次调用最多做多少工作"的上限，而不是"某个数值
    /// 能买到多少工作"的承诺。引擎大版本升级可能重新标定它，因此针对旧引擎调过这个值的宿主
    /// 应当重新实测，而不是假设同一个数字仍然够用。
    pub fuel_per_call: u64,
    /// Largest request payload accepted, in bytes.
    /// 接受的最大请求负载字节数。
    pub max_input_bytes: usize,
    /// Largest response payload accepted, in bytes.
    /// 接受的最大响应负载字节数。
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
    /// Build a backend that applies `limits` to every loaded instance.
    /// 构造一个对每个已加载实例施加 `limits` 的后端。
    pub const fn new(limits: WasmLimits) -> Self {
        Self { limits }
    }

    /// Compile and instantiate the artifact, requiring the host ABI version and the
    /// `nichlink_health` export to match before an instance is returned.
    /// 编译并实例化工件；返回实例前要求宿主 ABI 版本与 `nichlink_health` 导出一致。
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
        // `instantiate_and_start` is what `instantiate` plus `PreInstance::start`
        // became in wasmi 1.x; it runs the module's `start` function, which is
        // why the fuel above is set before it. A plugin cannot dodge its budget
        // by doing work during instantiation.
        // `instantiate_and_start` 就是 `instantiate` 加 `PreInstance::start` 在 wasmi 1.x
        // 中的形态；它会运行模块的 `start` 函数，这也正是上面那笔燃料必须在此之前设定好的
        // 原因。插件无法靠把工作放进实例化阶段来逃避预算。
        let instance = linker
            .instantiate_and_start(&mut store, &module)
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
