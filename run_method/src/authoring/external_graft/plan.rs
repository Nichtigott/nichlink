//! Persistent declarations for immutable external grafts.
//! 不可变外部 graft 的持久化声明。
//!
//! A plan file is an authoring record, not a declaration the compiler sees and
//! not an overlay application. It names the logical slot (`target_path`), the
//! base face's identity (`target`), the external implementation's selector, and
//! whether the whole subtree or only the node is replaced. `GraftPlanDocument`
//! in the kernel owns the text format; this module owns the filesystem.
//! 计划文件是创作记录：它不是编译器看到的声明，也不是覆盖应用。它记录逻辑槽位
//! （`target_path`）、原注册面的身份（`target`）、外部实现的选择器，以及替换整棵
//! 子树还是只替换节点。文本格式由 kernel 的 `GraftPlanDocument` 拥有，本模块只
//! 负责文件系统。

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::runtime::{LoadedGraft, graft_record_root, load_graft_record, load_graft_records};
use crate::{GraftPlanDocument, NodeId, Registry};
use nichlink::lexicon;

use super::super::filesystem::atomic_write;
use super::super::validation::package_root;

/// The directory that owns every external graft plan.
/// 拥有全部外部 graft 计划的目录。
///
/// The layout lives in the non-gated runtime loader now, so Studio and a
/// runtime host resolve the same path from the same constant list.
/// 版式现在住在不受门控的运行期加载器里，因此 Studio 与运行期宿主按同一份常量表解析
/// 同一条路径。
pub fn external_graft_root() -> PathBuf {
    graft_record_root(&package_root())
}

/// An external overlay declaration owned by the host package.
/// 宿主包拥有的外部覆盖声明。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalGraftPlanFile {
    /// The plan's directory name under `.nichlink/external-grafts/`.
    /// 计划在 `.nichlink/external-grafts/` 下的目录名。
    pub selector: String,
    /// The parsed plan body.
    /// 解析后的计划主体。
    pub document: GraftPlanDocument,
    /// The plan directory itself.
    /// 该计划所在目录。
    pub root: PathBuf,
}

impl ExternalGraftPlanFile {
    /// The path of the plan file inside `root`.
    /// `root` 内计划文件的路径。
    pub fn plan_path(&self) -> PathBuf {
        self.root.join(lexicon::GRAFT_PLAN_FILE)
    }

    /// The identity of the base face this plan replaces.
    /// 该计划所替换原注册面的身份。
    pub fn target(&self) -> NodeId {
        self.document.target
    }

    /// The logical slot path this plan replaces.
    /// 该计划替换的逻辑槽位路径。
    pub fn target_path(&self) -> &str {
        &self.document.target_path
    }

    /// The selector naming the external implementation.
    /// 命名外部实现的选择器。
    pub fn graft(&self) -> &str {
        &self.document.graft
    }

    /// Whether the whole subtree is replaced rather than only the node.
    /// 替换整棵子树还是仅替换该节点。
    pub fn full(&self) -> bool {
        self.document.full
    }
}

/// One directory under `.nichlink/external-grafts/`.
/// `.nichlink/external-grafts/` 下的一个目录。
///
/// A plan the tooling can read and a plan it cannot are both listed: a broken
/// file is something the author has to see, not something to hide.
/// 读得懂与读不懂的计划都会被列出：坏文件是作者必须看见的东西，不该被藏起来。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalGraftPlanEntry {
    /// The plan's directory name under `.nichlink/external-grafts/`.
    /// 计划在 `.nichlink/external-grafts/` 下的目录名。
    pub selector: String,
    /// The plan directory itself.
    /// 该计划所在目录。
    pub root: PathBuf,
    /// The parsed plan, or the reason it could not be read.
    /// 解析后的计划，或无法读取的原因。
    pub document: Result<GraftPlanDocument, String>,
}

impl ExternalGraftPlanEntry {
    /// The path of the plan file inside `root`.
    /// `root` 内计划文件的路径。
    pub fn plan_path(&self) -> PathBuf {
        self.root.join(lexicon::GRAFT_PLAN_FILE)
    }

    /// Borrow the parsed plan, or the stored reason it is unusable.
    /// 借出解析后的计划，或它不可用的已存原因。
    pub fn document(&self) -> Result<&GraftPlanDocument, &str> {
        self.document.as_ref().map_err(String::as_str)
    }
}

/// Reject a selector the plan format could not read back.
/// 拒绝计划格式读不回来的选择器。
fn checked_selector(selector: &str) -> Result<&str, String> {
    let selector = selector.trim();
    crate::validate_graft_selector(selector)?;
    Ok(selector)
}

/// Create an external graft plan without touching the base source tree.
/// 创建外部 graft 计划，不接触原树源码。
pub fn create_external_graft(
    registry: &Registry,
    target: NodeId,
    graft: impl Into<String>,
    full: bool,
) -> Result<ExternalGraftPlanFile, String> {
    let target_path = registry
        .path_for(target)
        .ok_or_else(|| format!("graft target `{target}` is not registered"))?;
    let selector = checked_selector(&graft.into())?.to_owned();

    let root = external_graft_root().join(&selector);
    if root.exists() {
        return Err(format!(
            "external graft `{selector}` already exists at {}",
            root.join(lexicon::GRAFT_PLAN_FILE).display()
        ));
    }
    let document = GraftPlanDocument::new(target, target_path, selector.clone(), full);
    fs::create_dir_all(&root)
        .map_err(|error| format!("cannot create external graft directory: {error}"))?;
    if let Err(error) = atomic_write(&root.join(lexicon::GRAFT_PLAN_FILE), &document.render()) {
        let _ = fs::remove_dir_all(&root);
        return Err(format!("cannot write external graft plan: {error}"));
    }

    Ok(ExternalGraftPlanFile {
        selector,
        document,
        root,
    })
}

/// Read one plan back, so an authoring surface can show and edit it.
/// 读回一条计划，供创作界面显示与编辑。
///
/// The read and parse go through the runtime loader, so Studio and a host refuse
/// exactly the same documents.
/// 读取与解析都走运行期加载器，因此 Studio 与宿主拒绝的文档完全相同。
pub fn read_external_graft(selector: &str) -> Result<ExternalGraftPlanFile, String> {
    let selector = checked_selector(selector)?;
    let document = load_graft_record(&package_root(), selector)?;
    Ok(ExternalGraftPlanFile {
        selector: selector.to_owned(),
        document,
        root: external_graft_root().join(selector),
    })
}

/// Every plan directory, including the ones that do not parse.
/// 列出全部计划目录，包括解析失败的。
pub fn list_external_grafts() -> Result<Vec<ExternalGraftPlanEntry>, String> {
    let root = external_graft_root();
    let loaded = load_graft_records(&package_root())?;
    Ok(loaded
        .into_iter()
        .map(|entry| match entry {
            LoadedGraft::Record(record) => ExternalGraftPlanEntry {
                root: root.join(&record.selector),
                selector: record.selector.clone(),
                document: Ok(record.document),
            },
            LoadedGraft::Unreadable { selector, reason } => ExternalGraftPlanEntry {
                root: root.join(&selector),
                selector,
                document: Err(reason),
            },
        })
        .collect())
}

/// Change whether a plan replaces the whole subtree.
/// 改变一条计划是否替换整棵子树。
pub fn rewrite_external_graft(selector: &str, full: bool) -> Result<ExternalGraftPlanFile, String> {
    let mut plan = read_external_graft(selector)?;
    if plan.document.full == full {
        return Ok(plan);
    }
    plan.document.full = full;
    let path = plan.plan_path();
    atomic_write(&path, &plan.document.render())?;
    Ok(plan)
}

/// Move one plan to the recoverable NichLink trash and return its new path.
/// 把一条计划移到可恢复的 NichLink 回收目录，并返回新路径。
pub fn remove_external_graft(selector: &str) -> Result<PathBuf, String> {
    let plan = read_external_graft(selector)?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("clock error: {error}"))?
        .as_nanos();
    let trash = package_root()
        .join(lexicon::NICHLINK_DIR)
        .join("trash")
        .join(lexicon::EXTERNAL_GRAFT_DIR)
        .join(format!("{}-{stamp}", plan.selector));
    fs::create_dir_all(trash.parent().expect("trash has a parent"))
        .map_err(|error| format!("cannot create NichLink trash: {error}"))?;
    fs::rename(&plan.root, &trash).map_err(|error| {
        format!(
            "cannot move {} to {}: {error}",
            plan.root.display(),
            trash.display()
        )
    })?;
    Ok(trash)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::AuthoringContext;

    /// Run one plan test against a throwaway package root.
    /// 在一个临时包根上运行一条计划测试。
    ///
    /// The name carries a counter as well as the clock: two tests can start in
    /// the same nanosecond on a platform with a coarse clock, and sharing one
    /// root made them overwrite each other's plan.
    /// 名字里除了时钟还有一个计数器：在时钟精度较粗的平台上两个测试可能落在同一纳秒，
    /// 共用一个根目录就会互相覆盖对方的计划。
    fn with_temp_root<T>(operation: impl FnOnce(&Path) -> T) -> T {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "nichlink-external-graft-{}-{stamp}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create root");
        let result =
            AuthoringContext::new(root.clone(), "nichlink.test").scope(|| operation(&root));
        let _ = fs::remove_dir_all(&root);
        result
    }

    /// A registry carrying one registered face, so a plan has a real target.
    /// 带一个已注册面的注册机，使计划有真实目标。
    fn registry_with_button() -> Registry {
        use crate::registry_core::{
            Admission, NodeId, OwnedFlowContract, OwnedLocalizedText, OwnedObjectContract,
            OwnedSourceLocation, RegistrationRule, RegistrationSnapshot, root_node_id,
        };

        let namespace = "nichlink.test";
        let kind = "Button";
        let mut registry =
            Registry::root_for_namespace(crate::FrameworkId::new("nichlink.test"), namespace);
        registry
            .register_snapshot_batch([RegistrationSnapshot {
                namespace: namespace.to_owned(),
                id: NodeId::from_namespaced_path(
                    namespace,
                    "control/object/button/button.rs",
                    kind,
                ),
                parent: root_node_id(namespace),
                kind: kind.to_owned(),
                preset: "NoPreset".to_owned(),
                parts: "NoParts".to_owned(),
                params: kind.to_owned(),
                handle: kind.to_owned(),
                stable_name: None,
                name: OwnedLocalizedText {
                    zh: kind.to_owned(),
                    en: kind.to_owned(),
                },
                summary: OwnedLocalizedText {
                    zh: String::new(),
                    en: String::new(),
                },
                exports: Vec::new(),
                needs_registry: false,
                registry_name: "button".to_owned(),
                getting_from_other_registry: None,
                registry_rule_path: "<test>".to_owned(),
                registry_rule: RegistrationRule::ANY.into_owned(),
                admission: Admission::ANY.into_owned(),
                requires: Vec::new(),
                provides: Vec::new(),
                contract: OwnedObjectContract {
                    required_parts: Vec::new(),
                    provided_parts: Vec::new(),
                },
                flow: OwnedFlowContract::none(),
                flow_provider: None,
                handle_traits: Vec::new(),
                part_traits: Vec::new(),
                runtime_checks: Vec::new(),
                plugin: None,
                source: OwnedSourceLocation {
                    file: "control/object/button/button.rs".to_owned(),
                    line: 1,
                    column: 1,
                    function: kind.to_owned(),
                },
            }])
            .expect("button registers");
        registry
    }

    /// The identity of the one face `registry_with_button` registers.
    /// `registry_with_button` 注册的那个面的身份。
    fn button_target(registry: &Registry) -> NodeId {
        registry
            .depth_first()
            .into_iter()
            .find(|info| info.registry_name == "button")
            .expect("button is registered")
            .id
    }

    #[test]
    fn a_created_plan_round_trips_is_listed_and_moves_to_trash() {
        with_temp_root(|root| {
            let registry = registry_with_button();
            let created =
                create_external_graft(&registry, button_target(&registry), "button_graft", false)
                    .expect("plan is created");
            assert_eq!(created.selector, "button_graft");
            assert_eq!(created.target_path(), "root/button");
            assert_eq!(created.graft(), "button_graft");
            assert!(!created.full());

            let text = fs::read_to_string(created.plan_path()).expect("plan text");
            assert_eq!(
                GraftPlanDocument::parse(&text).expect("plan parses"),
                created.document
            );
            assert!(text.contains("target_path=root/button\n"), "{text}");
            assert!(text.contains("full=false\n"), "{text}");

            assert_eq!(
                read_external_graft("button_graft").expect("read back"),
                created
            );
            let listed = list_external_grafts().expect("list plans");
            assert_eq!(listed.len(), 1);
            assert_eq!(listed[0].selector, "button_graft");
            assert_eq!(listed[0].document(), Ok(&created.document));

            let rewritten = rewrite_external_graft("button_graft", true).expect("toggle full");
            assert!(rewritten.full());
            assert!(
                read_external_graft("button_graft")
                    .expect("read back")
                    .full()
            );
            assert_eq!(
                rewrite_external_graft("button_graft", true).expect("idempotent"),
                rewritten
            );

            let trash = remove_external_graft("button_graft").expect("remove");
            assert!(
                trash.starts_with(
                    root.join(lexicon::NICHLINK_DIR)
                        .join("trash")
                        .join(lexicon::EXTERNAL_GRAFT_DIR),
                )
            );
            assert!(trash.join(lexicon::GRAFT_PLAN_FILE).is_file());
            assert!(list_external_grafts().expect("list plans").is_empty());
        });
    }

    #[test]
    fn a_duplicate_selector_is_refused_with_its_path() {
        with_temp_root(|_| {
            let registry = registry_with_button();
            let target = button_target(&registry);
            create_external_graft(&registry, target, "button_graft", false).expect("first plan");
            let error = create_external_graft(&registry, target, "button_graft", true)
                .expect_err("duplicate selector is refused");
            assert!(error.contains("already exists"), "{error}");
            assert!(error.contains(lexicon::GRAFT_PLAN_FILE), "{error}");
        });
    }

    #[test]
    fn selectors_that_escape_or_confuse_a_plan_are_refused() {
        with_temp_root(|_| {
            let registry = registry_with_button();
            let target = button_target(&registry);
            for selector in ["", "   ", "a/b", "a\\b"] {
                assert!(
                    create_external_graft(&registry, target, selector, false).is_err(),
                    "selector `{selector}` must be refused"
                );
            }
            assert!(read_external_graft("a/b").is_err());
        });
    }

    #[test]
    fn a_broken_plan_is_listed_with_its_reason() {
        with_temp_root(|_| {
            let broken = external_graft_root().join("broken");
            fs::create_dir_all(&broken).expect("create directory");
            fs::write(
                broken.join(lexicon::GRAFT_PLAN_FILE),
                "version=9\ntarget_path=root\ngraft=x\nfull=false\n",
            )
            .expect("write");
            let listed = list_external_grafts().expect("list plans");
            assert_eq!(listed.len(), 1);
            assert_eq!(listed[0].selector, "broken");
            let reason = listed[0].document().expect_err("a broken plan is reported");
            assert!(reason.contains("version `9`"), "{reason}");
        });
    }

    #[test]
    fn listing_a_package_without_plans_is_empty() {
        with_temp_root(|_| {
            assert!(list_external_grafts().expect("list plans").is_empty());
        });
    }
}
