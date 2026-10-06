//! The build-time graph, published as one artifact beside the records (audit `M7`, P1.1).
//! 构建期的图，作为记录旁边的一个产物发布（审计 `M7`，P1.1）。
//!
//! The facts this file carries were already in the build's hands — the resolved parent chain, the
//! file each face lives in, the names each face's file calls — but they were spread across
//! `pruning_manifest.tsv`, `file_manifest.tsv` and the reader's own re-derivation, so "who depends on
//! whom" was a question every reader had to answer for itself. This is that answer as **data**: one
//! row per edge, with the node count, the edge count and a content hash in the header.
//! 这份文件承载的事实本来就在构建手里——解析后的父链、每个面所在的文件、每个面的文件调用的名字——但它们
//! 散落在 `pruning_manifest.tsv`、`file_manifest.tsv` 与读者各自的重新推导里，于是"谁依赖谁"成了每个读者
//! 都得自己回答的问题。这里把那个答案变成**数据**：每条边一行，头部带节点数、边数与内容哈希。
//!
//! The one resolution it does make is the **unambiguous** one: a called name that exactly one file in
//! this tree declares is a dependency on that file, so the edge is `calls-file`; a name two files
//! declare, or none, stays a `name:` node. Ambiguity is therefore published as ambiguity rather than
//! guessed, and a reader that can resolve it further does — the read path's call graph owns that rule
//! 它做的唯一一次解析是**无歧义**的那一次：一个恰好被这棵树里一份文件声明的被调用名字，就是对那份文件的
//! 依赖，于是边是 `calls-file`；被两份文件声明、或没有任何文件声明的名字留作 `name:` 节点。因此歧义按歧义
//! 发布，而不是猜一个；能进一步解析它的读者去解析——那条规则属于读路径的调用图。
//!
//! A graph that could not be published is **not** written at all: the pipeline only writes the
//! fingerprint after every payload landed, so a partial graph would be read as a complete one.
//! 发布不出来的图**一个字节都不写**：管线只在每份载荷都落地之后才写指纹，因此半份图会被读成完整的。

use std::fmt::Write as _;
use std::path::Path;

use super::face_view::PruningRow;
use super::registry_syntax::GraftSyntax;
use super::{graft_view::graft_cut_label, write_if_changed};

/// The first line of the graph artifact, so a reader can name what it is holding.
/// 图产物的第一行，好让读者说出自己手里是什么。
pub(crate) const GRAPH_MARKER: &str = "nichlink-build-graph";

/// One edge of the build-time graph, with the kind that made it.
/// 构建期图的一条边，连同构成它的那种关系。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Edge {
    from: String,
    to: String,
    kind: &'static str,
}

/// Write the graph the record's own rows describe (audit `M7`, P1.1).
/// 写出记录自己的行所描述的图（审计 `M7`，P1.1）。
///
/// The rows are handed in rather than read back from `pruning_manifest.tsv`: the graph is a second
/// view of one computation, so the two files cannot disagree about a face, and the 50,000-row parse
/// that reading them back would cost is the shape this batch keeps removing.
/// 行是交进来的，而不是从 `pruning_manifest.tsv` 读回来的：图是同一次计算的第二个视图，因此两份文件不可能
/// 对一个面产生分歧，而读回来要付的五万行解析正是这一批一直在去掉的那种形状。
pub(crate) fn write_graph_manifest(
    out_dir: &Path,
    rows: &[PruningRow],
    file_rows: &[(String, String, String)],
    grafts: &[GraftSyntax],
) -> Result<(), String> {
    // Which file declares each bare name, and `None` as soon as two of them do: the ambiguity has to
    // be remembered rather than dropped, or a name declared twice would silently become an edge to
    // whichever file happened to be visited last.
    // 每个裸名由哪份文件声明，而一旦有两份就记 `None`：歧义必须被记住而不是丢掉，否则被声明两次的名字会
    // 静默变成"指向最后访问到的那份文件"的边。
    let mut declared_in: std::collections::BTreeMap<&str, Option<&str>> =
        std::collections::BTreeMap::new();
    for (source, function, _) in file_rows {
        if function == "-" {
            continue;
        }
        // The scanner reports a declaration as `module::name`, while a call site spells the bare name.
        // 扫描器把声明报成 `module::name`，而调用点拼的是裸名。
        let bare = function.rsplit("::").next().unwrap_or(function.as_str());
        declared_in
            .entry(bare)
            .and_modify(|known| {
                if known.is_some_and(|file| file != source.as_str()) {
                    *known = None;
                }
            })
            .or_insert(Some(source.as_str()));
    }
    let mut edges = Vec::new();
    for row in rows {
        let face = format!("face:{}", row.id);
        edges.push(Edge {
            from: face.clone(),
            to: format!("file:{}", row.source),
            kind: "in",
        });
        // A resolved parent that is this face itself would be a self-edge, which says nothing; the
        // reader of the record already treats an unresolvable chain as "derive".
        // 解析出来等于自身的父级是一条什么也没说的自环；记录的读者本来就把解析不出的链当作"推导"。
        if let Some(parent) = row.parent_node.as_deref()
            && parent != "-"
            && parent != row.id.to_string()
        {
            edges.push(Edge {
                from: face.clone(),
                to: format!("face:{parent}"),
                kind: "parent",
            });
        }
        for name in row
            .calls
            .as_deref()
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty() && *name != "-")
        {
            match declared_in.get(name).copied().flatten() {
                Some(source) => edges.push(Edge {
                    from: face.clone(),
                    to: format!("file:{source}"),
                    kind: "calls-file",
                }),
                None => edges.push(Edge {
                    from: face.clone(),
                    to: format!("name:{name}"),
                    kind: "calls",
                }),
            }
        }
    }
    // The graft plan is a dependency too: the host's cut site takes the grafted face, and the two
    // are spelled as Rust paths here because that is what the plan carries — resolution belongs to
    // the reader that has the declarations (see the module doc).
    // 嫁接计划也是一种依赖：宿主的切口接纳被嫁接的面，而两者在这里按计划携带的样子拼成 Rust 路径——解析
    // 属于手里有声明的读者（见模块文档）。
    for graft in grafts {
        edges.push(Edge {
            from: format!(
                "cut:{}",
                graft_cut_label(&graft.cut, graft.cut_end.as_deref())
            ),
            to: format!("graft:{}", graft.graft),
            kind: "graft",
        });
    }
    edges.sort();
    edges.dedup();

    let nodes: std::collections::BTreeSet<&str> = edges
        .iter()
        .flat_map(|edge| [edge.from.as_str(), edge.to.as_str()])
        .collect();
    let mut body = String::new();
    for edge in &edges {
        writeln!(body, "{}\t{}\t{}", edge.from, edge.to, edge.kind).unwrap();
    }
    let digest = super::registry_identity::NodeId::from_bytes(body.as_bytes()).to_string();
    let mut output = format!(
        "# graph\t{GRAPH_MARKER}\n# nodes\t{}\n# edges\t{}\n# digest\t{digest}\n# from\tto\tkind\n",
        nodes.len(),
        edges.len()
    );
    output.push_str(&body);
    write_if_changed(&out_dir.join("graph_edges.tsv"), &output)
}

#[cfg(test)]
#[path = "graph_tests.rs"]
mod graph_tests;
