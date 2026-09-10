use std::sync::{Arc, Mutex};

use arc_swap::ArcSwap;
use nichlink_run_method::{GraftPlan, Registry};

use crate::{HostError, PluginInstance};

/// One consistent view of registration metadata and executable code.
/// 注册元数据与可执行代码的一致视图。
pub struct Deployment<I> {
    pub generation: u64,
    pub registry: Registry,
    pub plugin: Arc<I>,
}

/// Lock-free reads with serialized, validated replacement.
/// 读取无锁，替换串行校验。
pub struct HotDeployment<I> {
    current: ArcSwap<Deployment<I>>,
    writer: Mutex<()>,
}

impl<I: PluginInstance> HotDeployment<I> {
    pub fn new(registry: Registry, plugin: I) -> Result<Self, HostError> {
        plugin.health_check()?;
        Ok(Self {
            current: ArcSwap::from_pointee(Deployment {
                generation: 0,
                registry,
                plugin: Arc::new(plugin),
            }),
            writer: Mutex::new(()),
        })
    }

    pub fn load(&self) -> Arc<Deployment<I>> {
        self.current.load_full()
    }

    /// Validate code and registry graft before publishing one new generation.
    /// 发布新代之前，先完成代码体检和注册树嫁接校验。
    pub fn replace(
        &self,
        plugin: I,
        plan: &GraftPlan,
        external: &Registry,
    ) -> Result<u64, HostError> {
        plugin.health_check()?;
        let _writer = self
            .writer
            .lock()
            .map_err(|_| HostError::Process("deployment writer lock was poisoned".to_owned()))?;
        let live = self.current.load_full();
        let registry = live
            .registry
            .overlay(plan, external)
            .map_err(|error| HostError::Registry(error.to_string()))?;
        let generation = live.generation.saturating_add(1);
        self.current.store(Arc::new(Deployment {
            generation,
            registry,
            plugin: Arc::new(plugin),
        }));
        Ok(generation)
    }
}
