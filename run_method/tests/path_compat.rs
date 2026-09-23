//! Public path compatibility, pinned in one place.
//! 公开路径兼容性，集中钉在这里。
//!
//! P2 made the module hierarchy the kernel's official surface and narrowed the
//! crate root to a whitelist. A host, an editor and every execution surface had
//! four ways to name one identity before that change, and all four have to keep
//! resolving: the official module path, the kernel's own module page, the
//! surface's flat re-export, and the surface's kernel module path. This file
//! fails to compile if any of them disappears.
//! P2 让模块层级成为内核的官方表面，并把 crate 根部收窄为白名单。改动之前，宿主、编辑器
//! 与每个执行面有四种方式命名同一个身份，四种都必须继续可解析：官方模块路径、内核自己的
//! 模块页、执行面的平铺重导出、执行面的内核模块路径。任何一种消失，本文件就编译不过。

use std::marker::PhantomData;

use nichlink::identity::NodeId as OfficialNodeId;
use nichlink::registry_core::identity::NodeId as KernelNodeId;
use nichlink_run_method::NodeId as SurfaceNodeId;
use nichlink_run_method::registry_core::identity::NodeId as SurfaceKernelNodeId;

/// The same identity, reached four ways: module path, kernel module page, the
/// surface's flat re-export, and the surface's kernel module path.
/// 同一个身份的四条路径：模块路径、内核模块页、执行面平铺重导出、执行面的内核模块路径。
#[test]
fn one_identity_resolves_through_every_historical_path() {
    let official = OfficialNodeId::from_namespaced_path("ns", "a/a.rs", "A");
    assert_eq!(
        official,
        KernelNodeId::from_namespaced_path("ns", "a/a.rs", "A")
    );
    assert_eq!(
        official,
        SurfaceNodeId::from_namespaced_path("ns", "a/a.rs", "A")
    );
    assert_eq!(
        official,
        SurfaceKernelNodeId::from_namespaced_path("ns", "a/a.rs", "A")
    );
}

/// The root whitelist keeps the nouns host code writes as bare names, while the
/// module page stays the place a type is owned.
/// 根部白名单保留宿主代码以裸名书写的名词；类型仍由所属模块页拥有。
#[test]
fn the_whitelist_and_the_module_pages_both_hold() {
    let _: PhantomData<nichlink::NodeId> = PhantomData;
    let _: PhantomData<nichlink::Registry> = PhantomData;
    let _: PhantomData<nichlink::identity::NodeId> = PhantomData;
    let _: PhantomData<nichlink::declaration::RegistrationInfo> = PhantomData;
    let _: PhantomData<nichlink::plugin::catalog::PluginCatalog> = PhantomData;
    let _: PhantomData<nichlink::plugin::graft_document::GraftPlanDocument> = PhantomData;
    let _: PhantomData<nichlink::plugin::graft::GraftCut> = PhantomData;
    assert_eq!(nichlink::lexicon::GENERATED_LIB_FILE, "generated_lib.rs");
}

/// The surfaces keep every kernel name they had: a host that writes
/// `nichlink_run_method::PluginManifest` or `nichlink_run_method::sha256_hex`
/// must not notice the kernel root getting smaller.
/// 执行面保留它们原有的全部内核名字：写 `nichlink_run_method::PluginManifest` 或
/// `nichlink_run_method::sha256_hex` 的宿主不该察觉内核根部变小了。
#[test]
fn the_surface_keeps_its_flat_kernel_names() {
    let _: PhantomData<nichlink_run_method::PluginManifest> = PhantomData;
    let _: PhantomData<nichlink_run_method::Registry> = PhantomData;
    let _: PhantomData<nichlink_run_method::plugin::catalog::PluginCatalog> = PhantomData;
    assert_eq!(nichlink_run_method::sha256_hex(b"").len(), 64);
}

/// Every kernel module page is part of the official surface, so a host that
/// reaches a noun through its module must keep being able to. One
/// representative item per module pins that the page itself still exists.
/// 每个内核模块页都是官方表面的一部分，因此通过模块取得名词的宿主必须保持可用。
/// 每个模块取一个有代表性的 item，钉住该页本身仍然存在。
#[test]
fn every_kernel_module_page_still_resolves() {
    let _: PhantomData<nichlink::identity::NodeId> = PhantomData;
    let _: PhantomData<nichlink::declaration::RegistrationInfo> = PhantomData;
    let _: PhantomData<nichlink::diagnostic::RegistryError> = PhantomData;
    let _: PhantomData<nichlink::mir::MirGraph> = PhantomData;
    let _: PhantomData<nichlink::release::StaticPlan> = PhantomData;
    let _: PhantomData<nichlink::requirements::CapabilityDeclaration> = PhantomData;
    let _: PhantomData<nichlink::source::SourceFunction> = PhantomData;
    let _: PhantomData<nichlink::tree::Registry> = PhantomData;
    let _: PhantomData<nichlink::plugin::catalog::PluginCatalog> = PhantomData;
    let _: PhantomData<nichlink::registry_core::identity::NodeId> = PhantomData;
    let _: PhantomData<nichlink::registry_core::tree::Registry> = PhantomData;
    assert_eq!(nichlink::lexicon::GENERATED_LIB_FILE, "generated_lib.rs");
    assert_eq!(
        nichlink::authoring::FACE_FIELD_COUNT,
        nichlink::authoring::FACE_FIELD_NAMES.len()
    );
}

/// The plugin protocol keeps its module pages on both the kernel and the
/// surface shim; a host that wrote `nichlink_run_method::plugin::trust::…`
/// must not notice the shim narrowing.
/// 插件协议在内核与执行面 shim 两侧都保留模块页；写过
/// `nichlink_run_method::plugin::trust::…` 的宿主不该察觉 shim 变窄。
#[test]
fn the_plugin_module_pages_still_resolve() {
    let _: PhantomData<nichlink::plugin::artifact::PluginArtifact> = PhantomData;
    let _: PhantomData<nichlink::plugin::contracts::FlowContract> = PhantomData;
    let _: PhantomData<nichlink::plugin::graft::GraftCut> = PhantomData;
    let _: PhantomData<nichlink::plugin::plugin_policy::PluginPolicy> = PhantomData;
    let _: PhantomData<nichlink::plugin::slot::PluginChannel> = PhantomData;
    let _: PhantomData<nichlink::plugin::trust::PluginTrustPolicy> = PhantomData;
    let _: PhantomData<nichlink_run_method::plugin::catalog::PluginCatalog> = PhantomData;
    let _: PhantomData<nichlink_run_method::plugin::graft_document::GraftPlanDocument> =
        PhantomData;
    let _: PhantomData<nichlink_run_method::plugin::trust::PluginTrustError> = PhantomData;
}

/// The runtime evidence surface stays reachable both through the
/// `runtime::*` page and through the historical `runtime::trace::locals::*`
/// page.
/// 运行期证据表面既可经 `runtime::*` 页抵达，也可经历史的
/// `runtime::trace::locals::*` 页抵达。
#[test]
fn the_runtime_and_locals_pages_still_resolve() {
    let _: PhantomData<nichlink_run_method::runtime::CallTrace> = PhantomData;
    let _: PhantomData<nichlink_run_method::runtime::Coordinates> = PhantomData;
    let _: PhantomData<nichlink_run_method::runtime::Provenance> = PhantomData;
    let _: PhantomData<nichlink_run_method::runtime::RuntimeValue> = PhantomData;
    let _: PhantomData<nichlink_run_method::runtime::trace::CallTrace> = PhantomData;
    let _: PhantomData<nichlink_run_method::runtime::trace::locals::LocalId> = PhantomData;
    let _: PhantomData<nichlink_run_method::runtime::trace::locals::LocalKind> = PhantomData;
    let _: PhantomData<nichlink_run_method::runtime::trace::locals::LocalValue> = PhantomData;
    let _: PhantomData<nichlink_run_method::runtime::trace::locals::Observation> = PhantomData;
}

/// The exported macro vocabulary keeps resolving under its public names, and
/// under the hidden helpers the generated aliases call. An unresolved macro
/// import is a hard compile error, which is the whole point of this file.
/// 导出的宏词表在其公开名称下保持可解析，隐藏辅助宏（生成的别名会调用它们）同样如此。
/// 宏导入无法解析就是硬编译错误——这正是本文件存在的意义。
#[test]
fn the_public_macro_names_still_resolve() {
    #[allow(unused_imports)]
    use nichlink_run_method::{
        __admission, __assert_impls, __control_object, __external_object, __face_string_or,
        __face_ty_name_or, __face_ty_or, __face_value_or, __flow, __flow_provider, __flow_select,
        __nichlink_object, __plugin, __registration_face, __stable_name, __string_list,
        __submit_registration,
    };
    #[allow(unused_imports)]
    use nichlink_run_method::{
        application, external_object, face_fields, face_fields_mirror, graft_plan, host,
        static_graft_plan, trace_call, trace_call_result, trace_consume, trace_transform,
        trace_value,
    };
    assert_eq!(
        nichlink_run_method::lexicon::GENERATED_LIB_FILE,
        "generated_lib.rs"
    );
}

/// The authoring surface is feature-gated, so it is pinned only when that
/// feature is on: the executor's public entry points and submodules must keep
/// their names. `edit_module` is deliberately absent — B3a deleted it.
/// authoring 表面受特性门控，因此只在打开该特性时钉住：执行器的公开入口与子模块必须
/// 保持名字。`edit_module` 刻意不在其中——B3a 已删除它。
#[cfg(feature = "authoring")]
#[test]
fn the_authoring_surface_still_resolves() {
    #[allow(unused_imports)]
    use nichlink_run_method::authoring::{
        AuthoringChange, AuthoringContext, FACE_FIELD_COUNT, FACE_FIELD_NAMES, ModuleFacePatch,
        NewModuleFace, add_module, add_module_from_face, add_module_with_registration,
        delete_module, edit_module_face, generated_snapshots, generated_snapshots_from,
    };
    #[allow(unused_imports)]
    use nichlink_run_method::authoring::{
        external_graft, filesystem, manifest, operations, parse, snapshot, validation,
    };
    assert_eq!(
        nichlink_run_method::authoring::FACE_FIELD_COUNT,
        nichlink_run_method::authoring::FACE_FIELD_NAMES.len()
    );
}
