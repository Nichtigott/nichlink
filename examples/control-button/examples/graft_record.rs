//! 运行 `cargo run -p nichlink-example-control-button --example graft_record`
//! 演示一个真实宿主如何把磁盘上的 graft 记录接进有效树：
//! `apply_recorded_grafts` 读取临时包根下的 `.nichlink/external-grafts/`，
//! 记录胜出时有效树携带记录的实现，而原树与构建捕获的静态计划都不动。
//! Run `cargo run -p nichlink-example-control-button --example graft_record` to
//! show how a real host wires on-disk graft records into its effective tree:
//! `apply_recorded_grafts` reads `.nichlink/external-grafts/` under a throwaway
//! package root, and when a record wins the effective tree carries the record's
//! implementation while neither the base tree nor the build-captured static plan
//! moves.
//!
//! Why the direct implementation would be wrong: the shipped declaration is
//! **typed** (`cut(NODE_ID) graft(NODE_ID)`), and a typed declaration is the
//! host's final word — a record over it is reported as `TypedDeclarationKept`
//! and ignored. This example therefore proves both halves: the typed plan keeps
//! `ButtonFast`, and a **string** declaration yields to the record so the
//! effective tree carries `SliderFast`. It also never points the loader at
//! `env!("CARGO_MANIFEST_DIR")`: a record is runtime input and must not leak
//! into the user's real `.nichlink/`.
//! 直白写法错在哪：出厂声明是**类型化**的（`cut(NODE_ID) graft(NODE_ID)`），而类型化
//! 声明是宿主的最终裁决——覆盖它的记录会被报告为 `TypedDeclarationKept` 并忽略。因此
//! 本示例证明三个情形：类型化计划保留 `ButtonFast`，**字符串**声明让位给记录使有效树
//! 携带 `SliderFast`，而没有声明保住的记录被跳过并打印原因。它也绝不把加载器指向
//! `env!("CARGO_MANIFEST_DIR")`：记录是运行期输入，不能泄漏进用户真实的 `.nichlink/`。
//!
//! The third case shows the failure that would otherwise be invisible: a record
//! whose slot no declaration keeps alive is skipped, and the skip is printed to
//! stderr as a `warning:` line rather than swallowed — the effective tree keeps
//! the base face, and the run says why.
//! 第三种情形展示那种否则不可见的失败：没有声明保住槽位的记录会被跳过，而这次跳过会
//! 作为 `warning:` 行打印到 stderr 而不是被吞掉——有效树保留原面，运行过程说明原因。
//!
//! Boundary: `apply_recorded_grafts` takes the package root as a parameter
//! precisely so an example or test can redirect it; this one passes a unique
//! directory under `std::env::temp_dir()` and removes it on drop.
//! 边界：`apply_recorded_grafts` 把包根作为参数，正是为了让示例或测试能重定向它；
//! 本示例传入 `std::env::temp_dir()` 下的唯一目录，并在析构时删除。
//! Pinned by `a_record_moves_the_effective_tree_but_not_the_static_plan` and
//! `run_method/tests/graft_record.rs::a_record_on_disk_reaches_overlay`.
//! 由 `a_record_moves_the_effective_tree_but_not_the_static_plan` 与
//! `run_method/tests/graft_record.rs::a_record_on_disk_reaches_overlay` 钉住。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use nichlink_run_method::registry_core::lexicon;
use nichlink_run_method::registry_core::{Registry, StaticGraftCut};
use nichlink_run_method::{
    GraftPlanDocument, RecordReport, apply_recorded_grafts, graft_record_root,
};

/// A throwaway package root, removed when the example ends.
/// 示例结束时删除的一次性包根。
struct TempRoot(PathBuf);

impl TempRoot {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "nichlink-graft-record-example-{}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("create the throwaway package root");
        Self(root)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Read the kind registered at a logical path.
/// 读取某逻辑路径上注册的 kind。
fn kind_at(registry: &Registry, wanted: &str) -> String {
    registry
        .depth_first()
        .iter()
        .find(|info| registry.path_for(info.id).as_deref() == Some(wanted))
        .map(|info| info.kind.clone())
        .unwrap_or_else(|| panic!("no registration face at `{wanted}`"))
}

fn main() {
    let root = TempRoot::new();
    // The directory name and the document's `graft` agree, which is now required:
    // a record whose directory and plan disagree is refused outright, because two
    // candidate implementations with no way to choose is not a state the author
    // could see in the source tree.
    // 目录名与文档的 `graft` 一致，而这是现在的硬性要求：目录与计划不一致的记录会被直接
    // 拒绝，因为"有两个候选实现且无从选择"是作者在源码树里看不到的状态。
    let selector = "slider_fast";
    let directory = graft_record_root(root.path()).join(selector);
    std::fs::create_dir_all(&directory).expect("create the record directory");
    let document = GraftPlanDocument::new(
        control_button::control::object::button::NODE_ID,
        "root/control/button",
        selector,
        false,
    );
    std::fs::write(directory.join(lexicon::GRAFT_PLAN_FILE), document.render())
        .expect("write the record");
    println!(
        "wrote {} under {}",
        lexicon::GRAFT_PLAN_FILE,
        directory.display()
    );

    let base = control_button::base_registry();
    let external = control_button_graft::external_registry();

    // 1. The shipped typed declaration is final: the record is reported and the
    //    effective tree keeps the declared implementation.
    // 1. 出厂类型化声明是最终裁决：记录被报告，有效树保留声明的实现。
    let typed = apply_recorded_grafts(
        &base,
        &external,
        control_button::builtin_static_plan().grafts(),
        root.path(),
    )
    .expect("the typed declaration still publishes");
    assert_eq!(
        kind_at(&typed.effective, "root/control/button"),
        "ButtonFast",
        "a typed declaration is not overridden by a record"
    );
    assert!(
        typed.reports.iter().any(|report| matches!(
            report,
            RecordReport::TypedDeclarationKept { recorded, .. } if recorded == selector
        )),
        "the ignored record is reported, not hidden: {:?}",
        typed.reports
    );

    // 2. A string declaration yields: the record's implementation reaches the
    //    effective tree, and the base tree still carries the declared face.
    // 2. 字符串声明让位：记录的实现进入有效树，而原树仍携带声明的面。
    let declared = [StaticGraftCut::new(
        "root/control/button",
        "button_fast",
        false,
    )];
    let recorded = apply_recorded_grafts(&base, &external, &declared, root.path())
        .expect("the string declaration yields to the record");
    assert!(
        recorded.reports.iter().any(|report| matches!(
            report,
            RecordReport::DeclarationOverridden { recorded, .. } if recorded == selector
        )),
        "the override is reported: {:?}",
        recorded.reports
    );
    assert_eq!(
        kind_at(&recorded.effective, "root/control/button"),
        "SliderFast",
        "the effective tree carries the record's implementation"
    );
    assert_eq!(
        kind_at(&base, "root/control/button"),
        "Button",
        "the base tree still carries the declared face"
    );
    assert_eq!(
        control_button::builtin_static_plan().grafts()[0]
            .graft()
            .id(),
        Some(control_button_graft::button_fast::NODE_ID),
        "the build-captured static plan never moves"
    );

    // 3. A record whose slot no declaration keeps alive is skipped, and the
    //    reason reaches stderr as a `warning:` line instead of being swallowed:
    //    the effective tree keeps the base face, and the run says why.
    // 3. 没有声明保住槽位的记录会被跳过，原因作为 `warning:` 行到达 stderr 而不是被
    //    吞掉：有效树保留原面，运行过程说明原因。
    let undeclared: [StaticGraftCut; 0] = [];
    let skipped = apply_recorded_grafts(&base, &external, &undeclared, root.path())
        .expect("an unkept record is skipped, not fatal");
    assert!(
        skipped.reports.iter().any(|report| matches!(
            report,
            RecordReport::UnkeptSlot { selector: reported, .. } if reported == selector
        )),
        "the skip is reported rather than hidden: {:?}",
        skipped.reports
    );
    assert_eq!(
        kind_at(&skipped.effective, "root/control/button"),
        "Button",
        "no graft is applied when no declaration keeps the slot alive"
    );

    println!("reports for the typed plan: {:?}", typed.reports);
    println!("reports for the string plan: {:?}", recorded.reports);
    println!("reports for the undeclared plan: {:?}", skipped.reports);
    println!("effective root/control/button kind: SliderFast");
    println!("base root/control/button kind: Button");
}
