use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use arc_swap::ArcSwapOption;
use nichlink::{
    FlowContract, FrameworkId, PluginAssurance, PluginMode, PluginSource, VerifiedPluginArtifact,
};

use crate::{HostError, PluginInstance, WasmBackend, WasmInstance};

/// Trust lane enabled for one runtime plugin slot.
/// 运行时插件槽允许使用的信任通道。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationChannel {
    Official,
    Community,
    Local,
}

/// A release-time opening for one Wasm extension or replacement.
/// 正式发布时保留的一个 Wasm 扩展或替换入口。
#[derive(Clone, Copy, Debug)]
pub struct WasmPluginSlot {
    pub name: &'static str,
    pub framework: FrameworkId,
    pub mode: PluginMode,
    pub contract: FlowContract,
    pub channels: &'static [ValidationChannel],
}

impl WasmPluginSlot {
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

struct PendingPlugin {
    generation: u64,
    artifact: VerifiedPluginArtifact,
}

struct LoadedPlugin {
    generation: u64,
    instance: WasmInstance,
}

struct SlotState {
    definition: WasmPluginSlot,
    active: ArcSwapOption<LoadedPlugin>,
    pending: Mutex<Option<PendingPlugin>>,
    has_pending: AtomicBool,
    next_generation: AtomicU64,
}

impl SlotState {
    fn install(
        &self,
        channel: ValidationChannel,
        artifact: VerifiedPluginArtifact,
    ) -> Result<u64, HostError> {
        validate_artifact(self.definition, channel, &artifact)?;
        let generation = self.next_generation.fetch_add(1, Ordering::Relaxed) + 1;
        let mut pending = self
            .pending
            .lock()
            .map_err(|_| HostError::State("plugin slot lock was poisoned".to_owned()))?;
        *pending = Some(PendingPlugin {
            generation,
            artifact,
        });
        self.has_pending.store(true, Ordering::Release);
        Ok(generation)
    }

    fn activate(&self, backend: WasmBackend) -> Result<(), HostError> {
        if !self.has_pending.load(Ordering::Acquire) {
            return Ok(());
        }
        let mut pending = self
            .pending
            .lock()
            .map_err(|_| HostError::State("plugin slot lock was poisoned".to_owned()))?;
        let Some(candidate) = pending.take() else {
            self.has_pending.store(false, Ordering::Release);
            return Ok(());
        };

        let loaded = backend.load(candidate.artifact).and_then(|instance| {
            instance.health_check()?;
            Ok(LoadedPlugin {
                generation: candidate.generation,
                instance,
            })
        });
        match loaded {
            Ok(loaded) => {
                self.active.store(Some(Arc::new(loaded)));
                self.has_pending.store(false, Ordering::Release);
                Ok(())
            }
            Err(error) => {
                self.has_pending.store(false, Ordering::Release);
                Err(error)
            }
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
    pub fn new(slots: &'static [WasmPluginSlot]) -> Result<Self, HostError> {
        Self::with_backend(slots, WasmBackend::default())
    }

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

    pub fn is_loaded(&self, slot: &str) -> Result<bool, HostError> {
        let state = self.slot(slot)?;
        Ok(!state.has_pending.load(Ordering::Acquire) && state.active.load().is_some())
    }

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
    if !slot.channels.contains(&channel) {
        return Err(HostError::Policy(format!(
            "plugin slot `{}` does not allow the {channel:?} channel",
            slot.name
        )));
    }
    let registration = artifact.registration();
    let manifest = registration
        .plugin
        .ok_or_else(|| HostError::Policy("verified plugin has no manifest".to_owned()))?;
    if manifest.framework != slot.framework {
        return Err(HostError::Policy(format!(
            "plugin targets `{}`, slot `{}` belongs to `{}`",
            manifest.framework, slot.name, slot.framework
        )));
    }
    if manifest.mode != slot.mode {
        return Err(HostError::Policy(format!(
            "plugin mode {:?} does not match slot mode {:?}",
            manifest.mode, slot.mode
        )));
    }
    match channel {
        ValidationChannel::Official
            if manifest.source != PluginSource::Official
                || artifact.assurance() != PluginAssurance::Signature =>
        {
            return Err(HostError::Policy(
                "official plugins require an official manifest and verified signature".to_owned(),
            ));
        }
        ValidationChannel::Community | ValidationChannel::Local
            if manifest.source != PluginSource::User =>
        {
            return Err(HostError::Policy(
                "community and local channels accept user manifests only".to_owned(),
            ));
        }
        _ => {}
    }
    if slot.mode == PluginMode::Replacement && !registration.flow.is_declared() {
        return Err(HostError::Contract(
            "replacement plugin has no flow contract".to_owned(),
        ));
    }
    if slot.contract.is_declared()
        && !registration
            .flow
            .semantically_compatible_with(slot.contract)
    {
        return Err(HostError::Contract(format!(
            "plugin flow {:?} does not match slot flow {:?}",
            registration.flow, slot.contract
        )));
    }
    Ok(())
}
