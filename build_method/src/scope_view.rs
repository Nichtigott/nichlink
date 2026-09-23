//! The scope and pruning readers: what the build published in `out_dir`.
//! 作用域与修剪读取方：构建发布在 `out_dir` 中的内容。
//!
//! Split decision: this page only reads the two manifests the build writes
//! (`source_scope.tsv`, `pruning_manifest.tsv`) and the view type derived from
//! them. It re-exports through `super::face_view` so `nichlink_build_method`
//! keeps its `face_view::{BuildScopeView, PruningRow, read_build_scope,
//! read_pruning_manifest}` surface unchanged, and it deliberately sits beside
//! the discovery walk (which reads source) because both are the build's own
//! read-only view of a host it cannot link.
//! 拆分决定：本页只读取构建写出的两份清单（`source_scope.tsv`、
//! `pruning_manifest.tsv`）以及由它们派生的视图类型。它经 `super::face_view` 重新
//! 导出，使 `nichlink_build_method` 的 `face_view::{BuildScopeView, PruningRow,
//! read_build_scope, read_pruning_manifest}` 表面保持不变；它刻意与发现遍历
//! （读取源码）相邻，因为两者都是构建对无法链接的宿主的只读视图。

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use super::super::registry_identity::NodeId;

/// One row of `pruning_manifest.tsv`: a symbol release-time pruning tracks for
/// one face.
/// `pruning_manifest.tsv` 的一行：发布期修剪为一个面跟踪的一个符号。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PruningRow {
    /// The face the symbol belongs to.
    /// 该符号所属的注册面。
    pub id: NodeId,
    /// The face's source path, relative to `src/`.
    /// 该面的源码路径，相对 `src/`。
    pub source: String,
    /// The tracked symbol, or `-` when the face has none.
    /// 被跟踪的符号；该面没有时是 `-`。
    pub symbol: String,
}

/// The build scope the last pipeline run published in `out_dir`.
/// 上一次管线运行发布在 `out_dir` 的构建作用域。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildScopeView {
    /// `explicit` when the scope came from `NICH_LINK_SCOPE`, `auto` otherwise.
    /// 作用域来自 `NICH_LINK_SCOPE` 时为 `explicit`，否则为 `auto`。
    pub mode: String,
    /// True when the manifest recorded `# result all`: every face is in scope.
    /// 清单记录 `# result all` 时为真：每个面都在作用域内。
    pub all: bool,
    /// The build's own one-line reason for the scope.
    /// 构建对作用域给出的一行原因。
    pub reason: Option<String>,
    /// The source paths of the selected faces, as the manifest's second column
    /// records them.
    /// 被选中注册面的源码路径，按清单第二列记录。
    pub selected_sources: BTreeSet<String>,
    /// The identities of the selected faces.
    /// 被选中注册面的身份。
    pub selected_ids: BTreeSet<NodeId>,
    /// The modules of the selected faces, as the manifest's third column
    /// records them.
    /// 被选中注册面的模块，按清单第三列记录。
    pub selected_modules: BTreeSet<String>,
}

impl BuildScopeView {
    /// Whether the published scope keeps a face whose module is `module`.
    /// 已发布的作用域是否保留模块为 `module` 的注册面。
    ///
    /// A narrowed scope records only its *roots* — the faces a declared cut
    /// names — but a parent face survives because a selected child needs it and
    /// because the entry reaches it. Treating "not a root" as "pruned" would
    /// tell an operator that a face present in the shipped tree is gone, so this
    /// test also accepts any selected module nested under `module`. The boundary
    /// is the `::` segment, exactly like the scope's own subtree selector, so
    /// `control` keeps `control::object::button` and never `control_extra`.
    /// 收窄的作用域只记录它的**根**——已声明切口命名的那些面——但父面会因为被选中的
    /// 子级需要它、入口能到达它而存活。把"不是根"当成"被剪掉"会告诉操作者一个仍然
    /// 存在于发布树中的面消失了，因此这里也接受嵌套在 `module` 之下的任何被选中模块。
    /// 边界是 `::` 段，与作用域自己的子树选择器完全一致：`control` 保留
    /// `control::object::button`，绝不保留 `control_extra`。
    pub fn keeps(&self, module: &str) -> bool {
        self.all
            || self.selected_modules.iter().any(|selected| {
                selected == module
                    || (selected.starts_with(module)
                        && selected.as_bytes().get(module.len()) == Some(&b':'))
            })
    }
}

/// Read `source_scope.tsv` as the build wrote it.
/// 按构建写出的样子读取 `source_scope.tsv`。
///
/// The format is build output, so this reader lives beside the writer instead of
/// in the command; a command that re-parsed it would freeze a second copy of the
/// layout. A missing file means the build has not run yet, which is an error the
/// caller reports with the path that is missing.
/// 该格式是构建产物，因此读取方与写入方放在一起，而不是放进命令里；命令自行解析会
/// 冻结第二份版式。文件缺失意味着构建尚未运行，调用方按缺失路径报错。
pub fn read_build_scope(out_dir: &Path) -> Result<BuildScopeView, String> {
    let path = out_dir.join("source_scope.tsv");
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mut scope = BuildScopeView {
        mode: "auto".to_owned(),
        all: false,
        reason: None,
        selected_sources: BTreeSet::new(),
        selected_ids: BTreeSet::new(),
        selected_modules: BTreeSet::new(),
    };
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("# mode\t") {
            scope.mode = rest.to_owned();
        } else if let Some(rest) = line.strip_prefix("# result\t") {
            scope.all = rest == "all";
        } else if let Some(rest) = line.strip_prefix("# reason\t") {
            scope.reason = Some(rest.to_owned());
        } else if line.is_empty() || line.starts_with('#') {
            continue;
        } else {
            let mut fields = line.split('\t');
            if let Some(id) = fields.next().and_then(|value| value.parse::<NodeId>().ok()) {
                scope.selected_ids.insert(id);
            }
            if let Some(source) = fields.next() {
                scope.selected_sources.insert(source.to_owned());
            }
            if let Some(module) = fields.next() {
                scope.selected_modules.insert(module.to_owned());
            }
        }
    }
    Ok(scope)
}

/// Read `pruning_manifest.tsv` as the build wrote it.
/// 按构建写出的样子读取 `pruning_manifest.tsv`。
pub fn read_pruning_manifest(out_dir: &Path) -> Result<Vec<PruningRow>, String> {
    let path = out_dir.join("pruning_manifest.tsv");
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mut rows = Vec::new();
    for line in text.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split('\t');
        let Some(id) = fields.next().and_then(|value| value.parse::<NodeId>().ok()) else {
            continue;
        };
        let Some(source) = fields.next() else {
            continue;
        };
        let Some(symbol) = fields.next() else {
            continue;
        };
        rows.push(PruningRow {
            id,
            source: source.to_owned(),
            symbol: symbol.to_owned(),
        });
    }
    Ok(rows)
}
