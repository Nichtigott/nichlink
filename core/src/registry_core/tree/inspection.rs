//! Registry tree inspection output.
//! 注册树检视输出。

use std::fmt::Write as _;

use super::Registry;
use crate::RuntimeValue;
use crate::registry_core::declaration::{CallSite, SourceLocation};
use crate::registry_core::diagnostic::{RegistryError, RegistryResult};
use crate::registry_core::identity::NodeId;
use crate::registry_core::release::StaticGraftCut;

impl Registry {
    fn ordered_entries(&self) -> Vec<&super::entry_pages::RegisteredEntry> {
        let mut entries = self.entries.values().collect::<Vec<_>>();
        entries.sort_by(|left, right| {
            (&left.info.registry_name, left.info.id)
                .cmp(&(&right.info.registry_name, right.info.id))
        });
        entries
    }

    /// Validate one observed runtime value against the checks a registry face declares.
    /// 用注册面声明的检查校验一个观测到的运行期取值。
    ///
    /// This is a host API: the kernel never observes a value, so the caller owns the boundary.
    /// Call it where a host-side value crosses into a plugin or consumer, with the `NodeId` of
    /// the face whose `runtime_checks:` list applies, the `RuntimeValue` the host built, and the
    /// current call path.
    /// 这是宿主 API：内核从不观测取值，边界由调用方拥有。请在宿主侧取值跨入插件或消费者的
    /// 那一点调用它，传入适用该取值的注册面 `NodeId`、宿主构造的 `RuntimeValue` 与当前调用路径。
    ///
    /// `call_path` is `CallTrace::current_path()` when a trace is active; a host that does not
    /// trace passes `Vec::new()`. The kernel never synthesizes a call path.
    /// 有活动 trace 时传 `CallTrace::current_path()`；不接 trace 的宿主传 `Vec::new()`。
    /// 内核从不合成调用路径。
    ///
    /// An empty `runtime_checks:` list passes for any value. On failure the error aggregates one
    /// child per failed check, each carrying the face's declaration source and the value's
    /// provenance; render it with `Display`. Presence or absence is the host's decision signal:
    /// the registry does not choose whether a failed check is fatal.
    /// `runtime_checks:` 为空的面对任何取值都通过。失败时按每条失败的检查聚合子错误，各自携带
    /// 声明源与来源链；用 `Display` 渲染。存在与否就是宿主的决策信号：注册机不替宿主决定是否致命。
    ///
    /// The aggregate keeps the four essential facts readable: `node()`, `path()`, `source()`,
    /// and `message()`, plus `children()` with one entry per failed check, so a host that has to
    /// branch on *which* check failed does not parse `Display`.
    /// 聚合错误保留四项必要事实的可读读取器：`node()`、`path()`、`source()`、`message()`，
    /// 以及每条失败检查一项的 `children()`；因此必须按“哪条检查失败”分支的宿主无需解析
    /// `Display`。
    ///
    /// ```no_run
    /// # use nichlink::{Provenance, Registry, RuntimeValue};
    /// # fn demo(registry: &Registry, node: nichlink::NodeId) {
    /// let value = RuntimeValue::number(0.5, Provenance::default().push(node, "Slider", "measure", "0.5"));
    /// if let Err(error) = registry.health_check(node, &value, Vec::new()) {
    ///     // The aggregate names the face and path; each child names the failed check.
    ///     eprintln!("{} {} {}: {}", error.node(), error.path(), error.source(), error.message());
    ///     for failure in error.children() {
    ///         eprintln!("{} [{}]", failure.message(), failure.source());
    ///     }
    /// }
    /// # }
    /// ```
    pub fn health_check(
        &self,
        node: NodeId,
        value: &RuntimeValue,
        call_path: Vec<CallSite>,
    ) -> RegistryResult<()> {
        let Some(entry) = self.entry_at(node) else {
            return Err(RegistryError::new(
                node,
                format!("<unknown:{node}>"),
                SourceLocation {
                    file: "<runtime>",
                    line: 0,
                    column: 0,
                    function: "health_check",
                },
                "object path was not registered",
            )
            .into());
        };
        let path = self
            .path_for(node)
            .expect("a registered entry always has a display path");
        let mut failures = Vec::new();
        for check in &entry.info.runtime_checks {
            if let Err(failure) = check.run(value) {
                let mut child = RegistryError::new(
                    node,
                    path.clone(),
                    entry.info.source.clone(),
                    format!("check `{}`: {}", failure.check, failure.message),
                );
                *child.source_chain_mut() = failure.provenance.steps;
                *child.call_path_mut() = call_path.clone();
                *child.registration_chain_mut() = self.registration_chain(node);
                failures.push(child);
            }
        }
        if failures.is_empty() {
            return Ok(());
        }
        let mut error = RegistryError::new(
            node,
            path,
            entry.info.source.clone(),
            "runtime health check failed",
        );
        *error.call_path_mut() = call_path;
        *error.registration_chain_mut() = self.registration_chain(node);
        *error.children_mut() = failures;
        Err(Box::new(error))
    }

    /// Render the registration tree as a human-readable multi-line report.
    /// 把注册树渲染成多行的人类可读报告。
    pub fn dump(&self) -> String {
        let mut output = String::new();
        writeln!(
            output,
            "REGISTRATION TREE  nodes={} registries={} checks={}",
            self.node_count(),
            self.registry_count(),
            self.check_count()
        )
        .unwrap();
        writeln!(
            output,
            "root [registry] kind=root node={} entries={} path={}",
            self.header.id,
            self.len(),
            self.header.path
        )
        .unwrap();
        self.render_dump_entries("", &mut output);
        output
    }

    /// Render the effective tree an overlay produces, without mutating either
    /// input.
    /// 渲染一次覆盖产生的有效树，不修改任一输入。
    ///
    /// The overlay result used to have no way out of the library except the
    /// caller reading fields off the returned `Registry`; this is the output
    /// path for the same question [`Registry::dump`] answers about a base tree.
    /// A host that holds the real `base_registry()` and `external_registry()`
    /// calls this with the build-captured `builtin_static_plan().grafts()`; a
    /// command outside the host cannot, because neither registry is linked into
    /// it.
    /// 覆盖结果过去除了调用方读取返回的 `Registry` 字段外没有出口；这就是
    /// [`Registry::dump`] 对基树回答的同一个问题的输出路径。持有真实
    /// `base_registry()` 与 `external_registry()` 的宿主用它搭配构建捕获的
    /// `builtin_static_plan().grafts()`；宿主之外的命令做不到，因为两棵注册树都没有
    /// 链接进它。
    pub fn dump_effective(
        &self,
        cuts: &[StaticGraftCut],
        external: &Registry,
    ) -> RegistryResult<String> {
        Ok(self.overlay_static(cuts, external)?.dump())
    }

    /// Render one subtree of the full dump, indented under `prefix`.
    /// 渲染完整 dump 的一棵子树，按 `prefix` 缩进。
    fn render_dump_entries(&self, prefix: &str, output: &mut String) {
        let count = self.entries.len();
        for (index, entry) in self.ordered_entries().into_iter().enumerate() {
            let last = index + 1 == count;
            let connector = if last { "`--" } else { "|--" };
            let path = format!("{}/{}", self.header.path, entry.info.registry_name);
            let state = entry.child.as_ref().map_or_else(
                || "object".to_owned(),
                |child| format!("registry entries={}", child.len()),
            );
            writeln!(
                output,
                "{}{} {} [{}] kind={} node={} path={} declared-at={} function={} checks={}",
                prefix,
                connector,
                entry.info.registry_name,
                state,
                entry.info.kind,
                entry.info.id,
                path,
                entry.info.source,
                entry.info.source.function,
                entry.info.runtime_checks.len()
            )
            .unwrap();
            if let Some(child) = entry.child.as_ref() {
                let child_prefix = format!("{}{}", prefix, if last { "   " } else { "|  " });
                child.render_dump_entries(&child_prefix, output);
            }
        }
    }
}
