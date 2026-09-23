//! Private generation state machine for one lazily activated Wasm slot.
//! 单个懒激活 Wasm 槽的私有代际状态机。

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use arc_swap::ArcSwapOption;
use nichlink_run_method::VerifiedPluginArtifact;

use crate::{HostError, PluginInstance, WasmBackend, WasmInstance};

use super::{ValidationChannel, WasmPluginSlot, validate_artifact};

/// One verified generation waiting for lazy compilation.
/// 一个等待懒编译的已验证代际。
pub(super) struct PendingPlugin {
    generation: u64,
    artifact: VerifiedPluginArtifact,
}

/// One activated generation paired with its live instance.
/// 一个已激活的代际及其存活实例。
pub(super) struct LoadedPlugin {
    pub(super) generation: u64,
    pub(super) instance: WasmInstance,
}

/// Generation bookkeeping for one named slot.
/// 单个具名槽的代际簿记。
pub(super) struct SlotState {
    pub(super) definition: WasmPluginSlot,
    pub(super) active: ArcSwapOption<LoadedPlugin>,
    pub(super) pending: Mutex<Option<PendingPlugin>>,
    pub(super) has_pending: AtomicBool,
    pub(super) next_generation: AtomicU64,
}

impl SlotState {
    pub(super) fn install(
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

    pub(super) fn activate(&self, backend: WasmBackend) -> Result<(), HostError> {
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
