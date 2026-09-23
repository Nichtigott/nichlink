//! Lazy, slot-scoped activation of verified Wasm plugins.
//! 已验证 Wasm 插件的懒加载、按槽激活。

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use arc_swap::ArcSwapOption;
use nichlink_run_method::{FlowContract, FrameworkId, PluginMode, VerifiedPluginArtifact};

use crate::{HostError, PluginInstance, WasmBackend};

#[path = "lazy_wasm/slot_state.rs"]
mod slot_state;
use slot_state::SlotState;

/// Trust lane enabled for one runtime plugin slot.
/// 运行时插件槽允许使用的信任通道。
///
/// The definition lives in the kernel `plugin` module; this alias keeps the
/// historical `nichlink_plugin_host::ValidationChannel` path.
/// 定义本体在 kernel 的 `plugin` 模块；本别名保留
/// `nichlink_plugin_host::ValidationChannel` 历史路径。
pub use nichlink_run_method::PluginChannel as ValidationChannel;

/// A release-time opening for one Wasm extension or replacement.
/// 正式发布时保留的一个 Wasm 扩展或替换入口。
#[derive(Clone, Copy, Debug)]
pub struct WasmPluginSlot {
    /// Slot name, unique within one table and used to address installs and calls.
    /// 槽名，在单个表内唯一，用于寻址安装与调用。
    pub name: &'static str,
    /// Framework whose artifacts this slot admits.
    /// 该槽接纳的 framework。
    pub framework: FrameworkId,
    /// Whether the plugin extends or replaces the framework.
    /// 插件是扩展还是替换该 framework。
    pub mode: PluginMode,
    /// Flow contract the artifact must match before activation.
    /// 激活前工件必须匹配的数据流合同。
    pub contract: FlowContract,
    /// Trust lanes allowed to install into this slot; must be non-empty.
    /// 允许安装进该槽的信任通道；不得为空。
    pub channels: &'static [ValidationChannel],
}

impl WasmPluginSlot {
    /// Build a slot definition; `const` so tables can live in static storage.
    /// 构造槽定义；为 `const`，便于把表放在静态存储中。
    pub const fn new(
        name: &'static str,
        framework: FrameworkId,
        mode: PluginMode,
        contract: FlowContract,
        channels: &'static [ValidationChannel],
    ) -> Self {
        Self {
            name,
            framework,
            mode,
            contract,
            channels,
        }
    }
}

/// Lazily activates verified Wasm plugins in explicitly retained slots.
/// 仅在显式保留的槽中懒加载已验证 Wasm 插件。
pub struct WasmPluginTable {
    backend: WasmBackend,
    slots: BTreeMap<&'static str, SlotState>,
}

impl WasmPluginTable {
    /// Build a table over the default backend, rejecting malformed slot definitions.
    /// 在默认后端上建表，并拒绝非法的槽定义。
    pub fn new(slots: &'static [WasmPluginSlot]) -> Result<Self, HostError> {
        Self::with_backend(slots, WasmBackend::default())
    }

    /// Build a table with an explicit backend so callers can supply their own limits.
    /// 用显式后端建表，便于调用方提供自己的限制。
    pub fn with_backend(
        slots: &'static [WasmPluginSlot],
        backend: WasmBackend,
    ) -> Result<Self, HostError> {
        let mut states = BTreeMap::new();
        for definition in slots {
            if definition.name.trim().is_empty() {
                return Err(HostError::Slot("plugin slot name is empty".to_owned()));
            }
            if definition.channels.is_empty() {
                return Err(HostError::Slot(format!(
                    "plugin slot `{}` has no validation channel",
                    definition.name
                )));
            }
            if states
                .insert(
                    definition.name,
                    SlotState {
                        definition: *definition,
                        active: ArcSwapOption::empty(),
                        pending: Mutex::new(None),
                        has_pending: AtomicBool::new(false),
                        next_generation: AtomicU64::new(0),
                    },
                )
                .is_some()
            {
                return Err(HostError::Slot(format!(
                    "duplicate plugin slot `{}`",
                    definition.name
                )));
            }
        }
        Ok(Self {
            backend,
            slots: states,
        })
    }

    /// Queue verified bytes without compiling or instantiating them.
    /// 挂起已验证字节，不在安装阶段编译或实例化。
    pub fn install(
        &self,
        slot: &str,
        channel: ValidationChannel,
        artifact: VerifiedPluginArtifact,
    ) -> Result<u64, HostError> {
        self.slot(slot)?.install(channel, artifact)
    }

    /// Activate a pending generation on demand, then invoke it.
    /// 首次使用时激活待发布代，再执行调用。
    pub fn call(&self, slot: &str, operation: &str, input: &[u8]) -> Result<Vec<u8>, HostError> {
        let state = self.slot(slot)?;
        state.activate(self.backend)?;
        let active = state
            .active
            .load_full()
            .ok_or_else(|| HostError::Slot(format!("plugin slot `{slot}` is not installed")))?;
        active.instance.call(operation, input)
    }

    /// Report whether the slot has an active generation and nothing pending.
    /// 报告该槽是否已有激活代际且没有待处理代际。
    pub fn is_loaded(&self, slot: &str) -> Result<bool, HostError> {
        let state = self.slot(slot)?;
        Ok(!state.has_pending.load(Ordering::Acquire) && state.active.load().is_some())
    }

    /// Return the active generation, or `None` while the slot is not yet activated.
    /// 返回激活代际；槽尚未激活时返回 `None`。
    pub fn generation(&self, slot: &str) -> Result<Option<u64>, HostError> {
        Ok(self
            .slot(slot)?
            .active
            .load_full()
            .map(|active| active.generation))
    }

    fn slot(&self, name: &str) -> Result<&SlotState, HostError> {
        self.slots
            .get(name)
            .ok_or_else(|| HostError::Slot(format!("unknown plugin slot `{name}`")))
    }
}

fn validate_artifact(
    slot: WasmPluginSlot,
    channel: ValidationChannel,
    artifact: &VerifiedPluginArtifact,
) -> Result<(), HostError> {
    nichlink_run_method::validate_artifact(
        slot.name,
        slot.framework,
        slot.mode,
        slot.contract,
        slot.channels,
        channel,
        artifact,
    )
    .map_err(|error| match error {
        nichlink_run_method::SlotValidationError::Policy(message) => HostError::Policy(message),
        nichlink_run_method::SlotValidationError::Contract(message) => HostError::Contract(message),
    })
}
