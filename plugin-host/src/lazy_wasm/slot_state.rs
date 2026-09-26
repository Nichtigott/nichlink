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
    /// Why the last activation of a pending generation failed, if it did.
    /// 上一次待发布代激活失败的原因（如果有）。
    ///
    /// A failed activation leaves `active` untouched, so the previous generation
    /// keeps answering and `is_loaded` keeps saying `true`; without this record
    /// the failure was invisible to every poller.
    /// 激活失败不会改动 `active`，上一代继续作答、`is_loaded` 继续说 `true`；没有这份记录，
    /// 失败对每个轮询方都不可见。
    pub(super) activation_error: Mutex<Option<String>>,
}

impl SlotState {
    pub(super) fn install(
        &self,
        channel: ValidationChannel,
        artifact: VerifiedPluginArtifact,
    ) -> Result<u64, HostError> {
        validate_artifact(self.definition, channel, &artifact)?;
        let mut pending = self
            .pending
            .lock()
            .map_err(|_| HostError::State("plugin slot lock was poisoned".to_owned()))?;
        // The generation is allocated while the lock is held. Allocating it
        // first let two racing installs number themselves out of order: the one
        // that took the lock second stored a *lower* generation last, so the
        // number a caller received could be superseded by an older one and the
        // higher generation was silently dropped.
        // 代际在持锁期间分配。先分配会让两个并发安装把编号排反：后拿到锁的那个最后写入一个
        // **更小**的代际，于是调用方收到的编号可能被更老的编号取代，更大的那个被静默丢弃。
        let generation = self.next_generation.fetch_add(1, Ordering::Relaxed) + 1;
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
                self.record_activation(None);
                Ok(())
            }
            Err(error) => {
                self.has_pending.store(false, Ordering::Release);
                self.record_activation(Some(error.to_string()));
                Err(error)
            }
        }
    }

    /// Remember, or clear, the outcome of the last activation.
    /// 记下或清除上一次激活的结果。
    fn record_activation(&self, error: Option<String>) {
        if let Ok(mut slot) = self.activation_error.lock() {
            *slot = error;
        }
    }
}
