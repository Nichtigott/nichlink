//! The `move` action: relocate a face — and the module subtree under it — to a new parent.
//! `move` 动作：把一个面（连同它下面的模块子树）搬到新的父级之下。
//!
//! Why this is its own module: `apply.rs` is a ratcheted file (600 code lines) and every action that
//! grows lives beside it, the same reason `apply_cut.rs` and `apply_promote.rs` do.
//! 为什么单独一个模块：`apply.rs` 是受棘轮约束的文件（600 代码行），而每个会长大的动作都住在它旁边
//! ——与 `apply_cut.rs`、`apply_promote.rs` 同一个理由。
//!
//! **A move is an identity change.** `NodeId = hash(namespace, source path, name)` takes the source
//! path as an input, so the face comes back under a new id and everything booked against the old one
//! stops resolving. The record under `.nichlink/moves/` is a compatibility note for the next reader,
//! not an audit ledger, and only the most recent five are kept.
//! **一次搬动就是一次身份变化。** `NodeId = hash(命名空间, 源码路径, 名字)` 把源码路径当输入，因此面会以
//! 新 id 回来，而按旧 id 记账的一切都不再解析。`.nichlink/moves/` 下的记录是留给下一个读者的兼容提示，
//! 不是审计账本，只保留最近五条。
//!
//! **What it writes, and why it is not the executor** (the maintainer's three premises, decided
//! 2026-10-09): it **may** rewrite hand-written face files (P1) but **previews unless `apply` is true**;
//! a move across a crate boundary is **refused**, asking for the declaration first (P2); and the alias
//! macro **is** rewritten automatically (P3). The executor cannot be reused: it only rewrites files it
//! generated, and `AuthoredFace` has no `parent` field at all — so `move` is a new write path that has
//! to keep its own atomicity and rollback.
//! **它写什么，以及为什么不用执行器**（维护者 2026-10-09 拍板的三个前提）：它**可以**改写手写的面文件
//! （P1），但**除非 `apply` 为真否则只预览**；跨 crate 边界的搬动**被拒绝**，要求先改声明（P2）；别名宏
//! **自动替换**（P3）。这里用不了执行器：它只改写它生成过的文件，而 `AuthoredFace` 根本没有 `parent`
//! 字段——因此 `move` 是一块新的写入路径，必须自己保证原子性与回滚。
//!
//! **Line-level, and it refuses rather than guesses** (audit `M7`, §M7.68). The front end gives no
//! source offsets — `nichlink-macro` is a proc-macro, so its spans serve diagnostics, and the kernel's
//! syntax face carries `location.line` and nothing finer — so the `parent:` field is found by **line**
//! and a file that does not spell it exactly once is refused by name.
//! **行级，而且拒绝而不是猜**（审计 `M7`，§M7.68）。前端不给源码偏移——`nichlink-macro` 是 proc-macro，
//! 它的 span 服务诊断，而 kernel 的 syntax 面只有 `location.line`——因此 `parent:` 是按行找的，拼不出
//! 恰好一次的文本会被点名拒绝。

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::build_method::FaceView;
use crate::mcp::apply::Outcome;
use nichlink_kernel::identity::NodeId;
use nichlink_kernel::lexicon;

/// Relocate one face under a new parent, previewing unless the caller is applying.
/// 把一个面搬到新的父级之下；除非调用方在落盘，否则只预览。
///
/// It takes **both** roots for the reason the caller cannot explain in one line: the claim ledger
/// (`add_crates.rs`) and the host entry are read from the **project** root, because a preview's copy skips
/// `.nichlink/` by design, while every write lands in the work directory — which is also why it takes the
/// `applying` flag rather than inferring it.
/// 它要**两个**根，理由没法在调用处一行说清：认领台账（`add_crates.rs`）与宿主入口都从**项目**根读，因为
/// 预览副本按设计跳过 `.nichlink/`；而每一处写入都落在工作目录里——这也是它接 `applying` 标志而不是自己
/// 推断的原因。
pub(crate) fn run_move(
    root: &Path,
    work: &Path,
    applying: bool,
    arguments: &Value,
) -> Result<Outcome, String> {
    let node =
        text(arguments, "node").map_err(|error| format!("{error}\n{}", move_example(root)))?;
    let to = text(arguments, "to").map_err(|error| format!("{error}\n{}", move_example(root)))?;
    let namespace = crate::mcp::registry::namespace(root)?;
    let views = crate::build_method::face_views(work, &namespace)?;
    // A `FaceView::source` is relative to the **source root** the build scans (`src/`, or whatever the
    // manifest renames it to), not to the package root — measured while writing this action: reading
    // `work.join(&face.source)` asked for `<copy>/panel/object/tile/tile.rs`, and the file is under
    // `<copy>/src/`. Every filesystem step below goes through this root, while the **identity** is
    // computed from the same relative path the build hashes (audit `M7`, §M7.69).
    // `FaceView::source` 是相对构建扫描的**源码根**（`src/`，或清单把它改名成的地方），而不是包根——
    // 写这个动作时实测：`work.join(&face.source)` 去读 `<副本>/panel/object/tile/tile.rs`，而文件在
    // `<副本>/src/` 下。下面每一步文件操作都走这个根，而**身份**用构建所哈希的同一条相对路径算
    // （审计 `M7`，§M7.69）。
    let source_root = crate::build_method::source_layout(work)?.scan_root;
    let face = named_face(&views, &node).ok_or_else(|| {
        format!(
            "no face in this tree has the path or identity `{node}`; `nichlink.faces` lists them, and \
             `nichlink.explain <path>` answers for one"
        )
    })?;
    // The destination is a face that owns a registry, or the package root itself — which is not a face,
    // because the root mounts the faces rather than being one of them.
    // 目的地是一个拥有注册机的面，或者包根本身——包根不是面，因为它挂载面而不是成为其中一个。
    let parent = if to == "root" {
        None
    } else {
        Some(named_face(&views, &to).ok_or_else(|| {
            format!(
                "no face in this tree has the path or identity `{to}`; `nichlink.faces` lists them, and \
                 the package root is spelled `root`"
            )
        })?)
    };
    if let Some(parent) = parent
        && !parent.owns_registry
    {
        return Err(format!(
            "`{}` does not own a registry, so nothing may be mounted under it; pick a face whose \
             declaration carries `needs_registry: true` (the ones this tool would accept are in \
             `nichlink.faces`)",
            parent.path
        ));
    }
    let name = match arguments.get("name").and_then(Value::as_str) {
        Some(name) => name.to_owned(),
        None => leaf_of(&face.path).to_owned(),
    };
    let new_path = match parent {
        Some(parent) => format!("{}/{name}", parent.path),
        None => format!("root/{name}"),
    };
    if new_path == face.path {
        return Err(format!(
            "`{}` is already there: the same parent and the same name would change nothing",
            face.path
        ));
    }
    if let Some(existing) = views.iter().find(|view| view.path == new_path) {
        return Err(format!(
            "`{new_path}` is already `{}`; moving onto an occupied position is a different operation — \
             pick another name (`name`) or another parent",
            existing.kind
        ));
    }
    refuse_a_claimed_subtree(work, face, parent, &new_path)?;
    refuse_entry_references(work, face)?;

    let source = source_root.join(&face.source);
    let text = std::fs::read_to_string(&source)
        .map_err(|error| format!("{} is not readable: {error}", source.display()))?;
    // The two spellings the tree writes: the root's own, and a typed module path for every face.
    // 这棵树写的两种拼法：根自己的那条，以及每个面用的类型化模块路径。
    let spelling = parent.map_or_else(
        || ROOT_PARENT_SPELLING.to_owned(),
        |parent| format!("crate::{}::NODE_ID", parent.module),
    );
    let parent_edit = rewrite_parent_line(&text, &spelling)?;
    let alias_edit = rewrite_alias(&text, &views, face, parent)?;
    let moved = moved_source(face, parent, &name);
    // The faces mounted **directly** under the one being moved spell its Rust path in their own
    // `parent:` lines, so they move with it: `crate::panel::NODE_ID` becomes `crate::shelf::panel::NODE_ID`.
    // Measured on a real move — the first version of this action left them alone and the tree stopped
    // compiling (`parent declaration cannot be resolved`, then `cannot find \`panel\` in \`crate\``),
    // while the **alias** rule the spec records still held: the child kept `panel_object!` because its
    // parent is the same registry owner it always was (audit `M7`, §M7.69).
    // 直接挂在这个面上的那些面，在自己的 `parent:` 行里拼着它的 Rust 路径，因此它们跟着一起改：
    // `crate::panel::NODE_ID` 变成 `crate::shelf::panel::NODE_ID`。在一次真搬动上实测——本动作的第一版
    // 没动它们，树就不编译了（先是 `parent declaration cannot be resolved`，随后是
    // `cannot find \`panel\` in \`crate\``），而规格记下的**别名**规则依然成立：那个子面保持
    // `panel_object!`，因为它的父级还是它一直以来的那个注册机拥有者（审计 `M7`，§M7.69）。
    let children = plan_child_edits(&source_root, &views, face, &moved)?;
    let old_id = face.id;
    let new_id = NodeId::from_namespaced_path(&namespace, &moved.source, &face.kind);

    let mut message = String::new();
    message.push_str(&format!("move   {} → {new_path}\n", face.path));
    message.push_str(&format!(
        "       parent: {} → {}\n",
        value_of(&parent_edit.before),
        value_of(&parent_edit.after)
    ));
    if let Some(alias) = &alias_edit {
        message.push_str(&format!("       alias:  {alias}\n"));
    }
    for child in &children {
        message.push_str(&format!(
            "       child {}: {} → {}\n",
            child.path, child.before_value, child.after_value
        ));
    }
    message.push_str(&format!(
        "       identity {old_id} → {new_id}\n       `NodeId` hashes the source path, so every record \
         that names the old identity stops resolving; the note under `.nichlink/moves/` is what the \
         next reader gets instead\n"
    ));

    if !applying {
        return Ok(Outcome {
            message,
            declaration: None,
            source: source_root.join(&moved.source),
            moved: true,
            alternative: false,
            inherited: Vec::new(),
            consumers: vec![format!(
                "records that name `{old_id}` no longer resolve; the move note is written on apply"
            )],
        });
    }

    apply_move(
        &source_root,
        &moved,
        &parent_edit,
        alias_edit.as_deref(),
        &children,
    )?;
    let record = write_record(work, &face.path, &new_path, old_id, new_id)?;
    message.push_str(&format!("record {}\n", record.display()));
    Ok(Outcome {
        message,
        declaration: None,
        source: source_root.join(&moved.source),
        moved: true,
        alternative: false,
        inherited: Vec::new(),
        consumers: Vec::new(),
    })
}

/// Where the face's module subtree goes.
/// 这个面的模块子树搬到哪里。
struct Moved {
    /// The directory to rename, relative to the package root.
    /// 要改名的目录，相对包根。
    from_dir: String,
    /// Its new name, relative to the package root.
    /// 它的新名字，相对包根。
    to_dir: String,
    /// The face file's new path, relative to the source root.
    /// 面文件的新路径，相对源码根。
    source: String,
    /// The moved face's new **module path**, which is what its children must now spell.
    /// 被搬面的新**模块路径**，也就是它的子面现在必须拼出的东西。
    module: String,
}

/// The destination of a move, computed from the parent and the leaf name.
/// 一次搬动的目的地，由父级与末段名字算出。
///
/// A face's module is its **directory** and its source is `<dir>/<leaf>.rs`, and the build refuses a
/// stem that disagrees with its directory — so the directory name, the file stem and the logical leaf
/// all move together.
/// 一个面的模块是它的**目录**，源码是 `<目录>/<末段>.rs`，而构建会拒绝与目录不一致的 stem——因此目录名、
/// 文件 stem 与逻辑末段三者一起搬。
fn moved_source(face: &FaceView, parent: Option<&FaceView>, name: &str) -> Moved {
    let from_dir = directory_of(&face.source);
    let to_dir = match parent {
        Some(parent) => {
            let parent_dir = directory_of(&parent.source);
            if parent_dir.is_empty() {
                name.to_owned()
            } else {
                format!("{parent_dir}/{name}")
            }
        }
        None => name.to_owned(),
    };
    let module = match parent {
        Some(parent) if !parent.module.is_empty() => format!("{}::{name}", parent.module),
        _ => name.to_owned(),
    };
    Moved {
        source: format!("{to_dir}/{name}.rs"),
        module,
        from_dir,
        to_dir,
    }
}

/// The directory part of a relative source path.
/// 相对源码路径的目录部分。
fn directory_of(source: &str) -> String {
    match source.rsplit_once('/') {
        Some((directory, _)) => directory.to_owned(),
        None => String::new(),
    }
}

/// The last segment of a logical path.
/// 逻辑路径的末段。
fn leaf_of(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// The face a path or an identity names.
/// 某个路径或身份指名的面。
///
/// Named for its object rather than `find`: a bare verb is only allowed at an entry position in this
/// workspace, and this is not one (the verb-table gate refused it by name).
/// 名字带着它的宾语，而不是叫 `find`：本工作区只允许裸动词出现在入口位置，而这里不是（动词表门禁点名拒了它）。
fn named_face<'a>(views: &'a [FaceView], target: &str) -> Option<&'a FaceView> {
    views
        .iter()
        .find(|view| view.path == target)
        .or_else(|| views.iter().find(|view| view.id.to_string() == target))
}

/// The alias macro a face uses, from the face its `parent:` points at.
/// 一个面所用的别名宏，由它的 `parent:` 指向的那个面决定。
///
/// `renderer/aliases.rs` emits one `<name>_object!` per registry-owning face and `root_object!` for the
/// root; a face uses the one belonging to the nearest registry owner **at or above its parent**, because
/// `parent:` is what decides the answer (audit `M7`, §1.3).
/// `renderer/aliases.rs` 为每个拥有注册机的面发一条 `<name>_object!`，为根发 `root_object!`；一个面用的是
/// **它父级自身或之上**最近的那个注册机拥有者的那条——因为决定这个答案的正是 `parent:`（审计 `M7`，§1.3）。
fn alias_for(views: &[FaceView], parent: Option<&FaceView>) -> String {
    let mut current = parent;
    while let Some(view) = current {
        if view.owns_registry {
            return format!("{}_object!", leaf_of(&view.path));
        }
        current = views.iter().find(|candidate| candidate.id == view.parent);
    }
    "root_object!".to_owned()
}

/// The `parent:` line, before and after.
/// `parent:` 那一行，改前与改后。
#[derive(Debug)]
struct ParentEdit {
    /// The whole line as it stands, including its indentation and trailing comma.
    /// 该行现状，含缩进与尾逗号。
    before: String,
    /// The replacement line.
    /// 替换后的行。
    after: String,
}

/// Rewrite the `parent:` line to point at the new parent.
/// 把 `parent:` 那一行改写成指向新的父级。
///
/// Found **by line**, because the front end offers no source offsets (see this module's own note), and
/// refused when the file does not spell it exactly once — a second guess here would edit a file the
/// author did not write that way.
/// **按行**找，因为前端不提供源码偏移（见本模块自己的说明），而在文件拼不出恰好一次时拒绝——在这里多猜
/// 一次会去改一份作者并非那样写的文件。
fn rewrite_parent_line(text: &str, spelling: &str) -> Result<ParentEdit, String> {
    let mut found: Vec<String> = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("parent")
            && rest.trim_start().starts_with(':')
        {
            found.push(line.to_owned());
        }
    }
    let [before] = found.as_slice() else {
        return Err(format!(
            "this face file spells `parent:` {} time(s), so this action will not guess which line to \
             rewrite; a face declares exactly one parent",
            found.len()
        ));
    };
    let trimmed = before.trim_start();
    let indent = &before[..before.len() - trimmed.len()];
    let value = trimmed
        .split_once(':')
        .map(|(_, value)| value.trim())
        .unwrap_or_default();
    let comma = if value.ends_with(',') { "," } else { "" };
    let value = value.trim_end_matches(',');
    // Only the two spellings the tree itself writes are accepted: a typed `crate::…::NODE_ID`, and the
    // root's `crate::root_node_id(crate::NICHLINK_NAMESPACE)`. Anything else is a shape this action has
    // never seen, and rewriting it would be a guess (audit `M7`, §M7.68).
    // 只接受这棵树自己写的那两种拼法：类型化的 `crate::…::NODE_ID`，以及根的
    // `crate::root_node_id(crate::NICHLINK_NAMESPACE)`。别的形状这个动作没见过，改写它就是猜
    // （审计 `M7`，§M7.68）。
    let typed = value.starts_with("crate::") && value.ends_with("::NODE_ID");
    let root = value.contains("root_node_id(");
    if !typed && !root {
        return Err(format!(
            "this face's `parent:` is spelled `{value}`, which is neither a typed `crate::…::NODE_ID` \
             nor the root's `crate::root_node_id(…)`; this action rewrites the two spellings the tree \
             itself writes, and guesses at nothing else"
        ));
    }
    let after = format!("{indent}parent: {spelling}{comma}");
    Ok(ParentEdit {
        before: before.clone(),
        after,
    })
}

/// One child's `parent:` line, and what it becomes.
/// 一个子面的 `parent:` 行，以及它变成什么。
#[derive(Debug)]
struct ChildEdit {
    /// The child's source **after** the move, relative to the source root.
    /// 搬动**之后**该子面的源码，相对源码根。
    source: String,
    /// The child's logical path, for the report.
    /// 该子面的逻辑路径，供报告用。
    path: String,
    /// The line to replace, and its replacement.
    /// 要替换的那一行，以及它的替代。
    before: String,
    after: String,
    /// Both values, for the preview.
    /// 两个值，供预览用。
    before_value: String,
    after_value: String,
}

/// Plan the `parent:` rewrite for every face mounted directly under the moved one.
/// 为每一个直接挂在被搬面之下的面，规划它的 `parent:` 改写。
///
/// Planned **before** the rename, so a child whose file does not spell its parent exactly once refuses the
/// whole move before anything on disk changes — and the edits land inside the renamed directory, so
/// rolling the rename back restores them.
/// 在改名**之前**规划，因此某个子面的文件拼不出恰好一次时，整次搬动会在磁盘上任何东西改变之前被拒绝——而
/// 这些改写落在被改名的目录里面，因此把改名回滚就恢复了它们。
fn plan_child_edits(
    source_root: &Path,
    views: &[FaceView],
    face: &FaceView,
    moved: &Moved,
) -> Result<Vec<ChildEdit>, String> {
    let spelling = format!("crate::{}::NODE_ID", moved.module);
    let mut edits = Vec::new();
    for child in views.iter().filter(|view| view.parent == face.id) {
        // Two paths, and mixing them up is what the first version did: the plan runs **before** the
        // rename, so the child is read where it is now and written where it will be (measured:
        // `src/shelf/panel/object/knob/knob.rs is not readable` — the post-move path, asked for too
        // early).
        // 两条路径，而把两者弄混正是第一版干的事：规划跑在改名**之前**，因此子面要按它**现在**的位置读、
        // 按它将来的位置写（实测：`src/shelf/panel/object/knob/knob.rs is not readable`——把搬动之后的
        // 路径问得太早了）。
        let here = source_root.join(&child.source);
        let source = child
            .source
            .strip_prefix(&moved.from_dir)
            .map(|rest| format!("{}{rest}", moved.to_dir))
            .unwrap_or_else(|| child.source.clone());
        let text = std::fs::read_to_string(&here)
            .map_err(|error| format!("{} is not readable: {error}", here.display()))?;
        let edit = rewrite_parent_line(&text, &spelling).map_err(|error| {
            format!(
                "`{}` hangs under the face being moved, and {error}",
                child.path
            )
        })?;
        edits.push(ChildEdit {
            source,
            path: child.path.clone(),
            before_value: value_of(&edit.before),
            after_value: value_of(&edit.after),
            before: edit.before,
            after: edit.after,
        });
    }
    Ok(edits)
}

/// The value half of a `parent:` line, without the key or the trailing comma.
/// `parent:` 行的值那一半，不含键与尾逗号。
fn value_of(line: &str) -> String {
    line.trim()
        .split_once(':')
        .map(|(_, value)| value.trim().trim_end_matches(',').to_owned())
        .unwrap_or_else(|| line.trim().to_owned())
}

/// Rewrite the alias macro when the new parent changes whose registry it is.
/// 当新的父级换了注册机归属时，改写别名宏。
///
/// P3: automatic. It is the one edit that is not required for the tree to **compile** — the face would
/// build either way — and it is required for the file to keep telling the truth about where it hangs.
/// P3：自动。这是唯一一处"不改也能**编译**过"的改写——两种写法都能构建——而不改的话，这个文件就会继续
/// 说错自己挂在哪里。
fn rewrite_alias(
    text: &str,
    views: &[FaceView],
    face: &FaceView,
    parent: Option<&FaceView>,
) -> Result<Option<String>, String> {
    let current_parent = views.iter().find(|view| view.id == face.parent);
    let before = alias_for(views, current_parent);
    let after = alias_for(views, parent);
    if before == after {
        return Ok(None);
    }
    if !text.contains(&before) {
        return Err(format!(
            "this face hangs under `{before}` by its `parent:` line, but its file never spells that \
             alias; this action rewrites the alias the tree derives, and refuses to invent one in a file \
             that does not carry it"
        ));
    }
    Ok(Some(format!("{before} → {after}")))
}

/// Refuse a move that crosses a crate boundary, in either direction.
/// 拒绝任何方向的跨 crate 边界搬动。
///
/// P2: refuse first and ask for the declaration. The claim ledger (`add_crates.rs`) is what decides
/// which crate compiles a subtree, so a move that leaves a claim or lands inside one is a partition
/// change wearing a move's clothes.
/// P2：先拒绝、要求先改声明。认领台账（`add_crates.rs`）决定哪棵子树由哪个 crate 编译，因此一次"离开某个
/// 认领"或"落进某个认领"的搬动，是穿着搬动外衣的分区改动。
fn refuse_a_claimed_subtree(
    work: &Path,
    face: &FaceView,
    parent: Option<&FaceView>,
    new_path: &str,
) -> Result<(), String> {
    let Ok(declaration) = crate::build_method::read_shape_declaration(work) else {
        return Ok(());
    };
    // No declaration is the ordinary case: nothing claims anything, so nothing can be crossed.
    // 没有声明是普通情形：没有东西被认领，也就无从跨越。
    let Some(declaration) = declaration else {
        return Ok(());
    };
    let claims = declaration.cut_subtrees();
    let new_module = match parent {
        Some(parent) => format!("{}::{}", parent.module, leaf_of(new_path)),
        None => leaf_of(new_path).to_owned(),
    };
    for claim in &claims {
        if under(&face.module, claim) {
            return Err(format!(
                "`{}` is claimed by `{claim}` in `add_crates.rs`, so another crate compiles it; a move \
                 across that boundary is a partition change — remove or narrow the claim first, then \
                 move",
                face.path
            ));
        }
        if under(&new_module, claim) {
            return Err(format!(
                "the destination `{new_path}` lands inside `{claim}`, which `add_crates.rs` claims for \
                 another crate; a move across that boundary is a partition change — change the claim \
                 first, then move"
            ));
        }
    }
    Ok(())
}

/// Whether `module` is `claim` or sits under it.
/// `module` 是不是 `claim`、或位于其下。
fn under(module: &str, claim: &str) -> bool {
    module == claim || module.starts_with(&format!("{claim}::"))
}

/// Refuse a move of a face the host entry's graft declarations name.
/// 拒绝搬动一个被宿主入口的 graft 声明点名的面。
///
/// The two spellings both break: a typed `cut(crate::old::module::NODE_ID)` stops compiling, and a
/// string `cut "root/old/path"` silently names a position that no longer exists. Rewriting them is the
/// next slice, so this one refuses by name rather than leaving a stale reference behind.
/// 两种拼法都会坏：类型化的 `cut(crate::旧::模块::NODE_ID)` 编译不过，字符串 `cut "root/旧/路径"` 则静默
/// 指向一个不再存在的位置。改写它们是下一片，因此这一片按名拒绝，而不是留下一条陈旧引用。
fn refuse_entry_references(work: &Path, face: &FaceView) -> Result<(), String> {
    let Ok(declared) = crate::build_method::declared_grafts(work) else {
        return Ok(());
    };
    let naming: Vec<&crate::build_method::DeclaredGraft> = declared
        .cuts
        .iter()
        .filter(|cut| cut.names_face(&face.path, Some(&face.module)))
        .collect();
    if naming.is_empty() {
        return Ok(());
    }
    let lines = naming
        .iter()
        .map(|cut| format!("line {}: `{}`", cut.line, cut.cut))
        .collect::<Vec<_>>()
        .join(", ");
    Err(format!(
        "the host entry declares a graft on `{}` ({lines}), and a move changes both the module path and \
         the identity those declarations spell; rewriting entry declarations is not part of this action \
         yet — move the declaration by hand first (or drop it), then move the face",
        face.path
    ))
}

/// Rename the subtree and rewrite the two lines, rolling the rename back if the rewrite fails.
/// 给子树改名并改写那两行；改写失败时把改名回滚。
fn apply_move(
    source_root: &Path,
    moved: &Moved,
    parent_edit: &ParentEdit,
    alias: Option<&str>,
    children: &[ChildEdit],
) -> Result<(), String> {
    let from = source_root.join(&moved.from_dir);
    let to = source_root.join(&moved.to_dir);
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    std::fs::rename(&from, &to).map_err(|error| {
        format!(
            "cannot move {} to {}: {error}",
            from.display(),
            to.display()
        )
    })?;
    let file = source_root.join(&moved.source);
    let text = match std::fs::read_to_string(&file) {
        Ok(text) => text,
        Err(error) => {
            let _ = std::fs::rename(&to, &from);
            return Err(format!("{} is not readable: {error}", file.display()));
        }
    };
    let edited = text.replace(&parent_edit.before, &parent_edit.after);
    if edited == text {
        let _ = std::fs::rename(&to, &from);
        return Err(format!(
            "the `parent:` line was found but the rewrite changed nothing in {}; refusing to report a \
             move that did not happen",
            file.display()
        ));
    }
    let edited = match alias.and_then(|alias| alias.split_once(" → ")) {
        Some((before, after)) => edited.replace(before, after),
        None => edited,
    };
    if let Err(error) = std::fs::write(&file, &edited) {
        let _ = std::fs::rename(&to, &from);
        return Err(format!(
            "cannot write {}: {error} (the rename was rolled back)",
            file.display()
        ));
    }
    // The children move with it, inside the directory that was just renamed: any failure here is undone
    // by renaming the directory back, because every edit lives inside it.
    // 子面跟着一起搬，落在那次刚改名的目录里：这里任何失败都可以靠把目录改回去撤销，因为每一处改写都在它里面。
    for child in children {
        let path = source_root.join(&child.source);
        let text = std::fs::read_to_string(&path).map_err(|error| {
            let _ = std::fs::rename(&to, &from);
            format!("{} is not readable: {error}", path.display())
        })?;
        let edited = text.replace(&child.before, &child.after);
        if edited == text {
            let _ = std::fs::rename(&to, &from);
            return Err(format!(
                "the `parent:` line of `{}` was found but the rewrite changed nothing; the rename was \
                 rolled back",
                child.path
            ));
        }
        if let Err(error) = std::fs::write(&path, &edited) {
            let _ = std::fs::rename(&to, &from);
            return Err(format!(
                "cannot write {}: {error} (the rename was rolled back)",
                path.display()
            ));
        }
    }
    Ok(())
}

/// Write the move record, keeping the most recent five.
/// 写下搬动记录，只保留最近五条。
fn write_record(
    work: &Path,
    from: &str,
    to: &str,
    old_id: NodeId,
    new_id: NodeId,
) -> Result<PathBuf, String> {
    let directory = work.join(lexicon::NICHLINK_DIR).join(lexicon::MOVES_DIR);
    let next = next_number(&directory);
    let record = directory.join(next.to_string());
    std::fs::create_dir_all(&record)
        .map_err(|error| format!("cannot create {}: {error}", record.display()))?;
    let when = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default();
    let body =
        format!("version=1\nfrom={from}\nto={to}\nfrom_id={old_id}\nto_id={new_id}\nwhen={when}\n");
    let path = record.join(lexicon::MOVE_PLAN_FILE);
    std::fs::write(&path, body)
        .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    prune(&directory, KEEP_RECORDS);
    Ok(path)
}

/// How the root is spelled in a `parent:` line.
/// 根在 `parent:` 行里的拼法。
const ROOT_PARENT_SPELLING: &str = "crate::root_node_id(crate::NICHLINK_NAMESPACE)";

/// How many move records are kept (the maintainer's decision, 2026-10-09).
/// 保留多少条搬动记录（维护者 2026-10-09 的决定）。
const KEEP_RECORDS: u32 = 5;

/// The numbers of the record directories, ascending.
/// 记录目录的编号，升序。
fn numbers(directory: &Path) -> Vec<u32> {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut numbers: Vec<u32> = entries
        .flatten()
        .filter_map(|entry| entry.file_name().to_string_lossy().parse::<u32>().ok())
        .collect();
    numbers.sort_unstable();
    numbers
}

/// The number the next record takes.
/// 下一条记录取的编号。
fn next_number(directory: &Path) -> u32 {
    numbers(directory).last().map_or(1, |last| last + 1)
}

/// Drop the oldest records so that at most `keep` remain.
/// 丢掉最旧的记录，使至多留下 `keep` 条。
fn prune(directory: &Path, keep: u32) {
    let numbers = numbers(directory);
    let excess = numbers.len().saturating_sub(keep as usize);
    for number in numbers.into_iter().take(excess) {
        let _ = std::fs::remove_dir_all(directory.join(number.to_string()));
    }
}

/// One string argument, or the shape this action accepts.
/// 取一个字符串参数，或给出本动作接受的形状。
fn text(arguments: &Value, key: &str) -> Result<String, String> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("this action needs `{key}`"))
}

/// A complete move request, built from the tree this call resolved.
/// 一个完整的搬动请求，由本次调用解析出的那棵树搭出来。
///
/// Same rule as `add`'s and `promote`'s examples: a refusal that offers placeholders makes the reader go
/// and list the tree before the request becomes a call.
/// 与 `add`、`promote` 的示例同一条规则：给出占位符的拒绝，会让读者在请求变成调用之前先去列一遍树。
fn move_example(root: &Path) -> String {
    let namespace = crate::mcp::registry::namespace(root).unwrap_or_default();
    let views = crate::build_method::face_views(root, &namespace).unwrap_or_default();
    let face = views
        .iter()
        .find(|view| !view.owns_registry)
        .or(views.first());
    let parent = views
        .iter()
        .find(|view| view.owns_registry)
        .map(|view| view.path.clone())
        .unwrap_or_else(|| "root".to_owned());
    let node = face
        .map(|view| view.path.clone())
        .unwrap_or_else(|| "root".to_owned());
    format!(
        "accepted shape  {{\"action\":\"move\",\"node\":\"{node}\",\"to\":\"{parent}\",\"apply\":true}}"
    )
}

#[cfg(test)]
#[path = "apply_move_tests.rs"]
mod apply_move_tests;
