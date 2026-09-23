//! The default `source` of an `external_object!` face.
//! `external_object!` 注册面 `source` 的默认值。
//!
//! The roadmap carried this as an unverified inference for a while:
//! `external_object!` lets an author write `source: …`, and the out-of-project
//! example does exactly that, so nothing exercised the default branch. The
//! default is `manifest_relative_source(env!("CARGO_MANIFEST_DIR"), file!())`,
//! and both halves of that expression expand in the *declaring* crate — which is
//! what makes an out-of-project face report its own file rather than the host's.
//! 路线图曾把这件事长期记为"推断未验证"：`external_object!` 允许作者写 `source: …`，而
//! 项目外的示例正是这么写的，因此默认分支从未被执行。默认值是
//! `manifest_relative_source(env!("CARGO_MANIFEST_DIR"), file!())`，其中两部分都在
//! **声明它的** crate 里展开——这正是项目外的注册面报告自己的文件而不是宿主文件的原因。
//!
//! What the helper does to that path is the easy part to get wrong, so this file
//! pins it: it strips the manifest prefix and then a leading `src/`, and it leaves
//! a path that does not start with the manifest directory untouched. A published
//! crate's `src/control/control.rs` therefore records `control/control.rs` (the
//! registry-layout path the tooling uses), while this integration test — built
//! from a workspace, where Cargo hands rustc a workspace-relative path — records
//! `run_method/tests/external_source_default.rs`. Both name the declaring crate's
//! own file, and neither bakes in an absolute path.
//! 该辅助函数对路径做的事最容易弄错，因此本文件把它钉住：先剥离 manifest 前缀，再剥离
//! 开头的 `src/`；而不以 manifest 目录开头的路径原样保留。因此发布包里
//! `src/control/control.rs` 记录为 `control/control.rs`（工具使用的注册布局路径），而本集成
//! 测试——从工作区构建、Cargo 交给 rustc 的是工作区相对路径——记录为
//! `run_method/tests/external_source_default.rs`。两者都指向声明方 crate 自己的文件，也都
//! 不会把绝对路径烤进去。
//!
//! The fixture declares nothing but `kind`, so it also pins the compact form the B3b
//! batch shrank to one required field.
//! 夹具除 `kind` 外什么都不写，因此它同时钉住 B3b 收缩到"一个必填字段"的紧凑形式。
//!
//! Pinned by `an_external_face_defaults_its_source_to_this_file`.
//! 由 `an_external_face_defaults_its_source_to_this_file` 钉住。

use std::path::Path;

use nichlink_run_method::registry_core::{FrameworkId, Registry};

nichlink_run_method::external_object! {
    kind: DefaultSourced,
}

#[test]
fn an_external_face_defaults_its_source_to_this_file() {
    let mut registry = Registry::root_for_namespace(
        FrameworkId::new("external-source-default"),
        env!("CARGO_PKG_NAME"),
    );
    registry
        .register_all(&[REGISTRATION])
        .expect("the external face registers");

    let face = registry
        .depth_first()
        .into_iter()
        .find(|info| info.kind == "DefaultSourced")
        .expect("the declared face is registered");

    // Separator-normalized so the expectation holds on Windows too: `file!()`
    // records the platform separator, and a declaration cannot rewrite it.
    // 做分隔符归一化以便在 Windows 上也成立：`file!()` 记录平台分隔符，声明无法改写它。
    let source = face.source.file.replace('\\', "/");
    assert!(
        source.ends_with("tests/external_source_default.rs"),
        "the default names this file, however Cargo spelled it: {source}"
    );
    assert!(
        !Path::new(&face.source.file).is_absolute(),
        "a recorded source must not bake in the build machine: {source}"
    );
}
