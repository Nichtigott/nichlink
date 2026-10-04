//! What an agent names, resolved to the identity the executor wants.
//! 代理所命名的东西，解析成执行器需要的身份。
//!
//! Two spellings, one answer: `nichlink.registry` and `nichlink.apply` both report
//! logical paths, so a caller can take a path out of the tree it just read and hand
//! it straight back — no translation into a 32-digit identity, and no chance of
//! translating it wrong. An identity is accepted too, because a caller that already
//! has one should not have to look it up.
//! 两种写法，一个答案：`nichlink.registry` 与 `nichlink.apply` 都报告逻辑路径，因此调用方可以
//! 把它刚读到的树里的路径直接交回来——不必翻译成 32 位身份，也就没有翻译错的机会。身份同样接受，
//! 因为已经持有身份的调用方不该被迫再查一次。
//!
//! online: an agent names what it can see in the sources now, and a name added since the build is not in the record.

use std::path::Path;

use crate::build_time::FaceView;
use nichlink_kernel::NodeId;
use serde_json::Value;

/// The parent identity a request names, by identity or by logical path.
/// 请求命名的父身份，可按身份或按逻辑路径给出。
///
/// A logical path is the agent-friendly spelling, and it is resolved against the
/// same derivation the registry query reports — so an agent can take a path from
/// `nichlink.registry` and use it here without translating it.
/// 逻辑路径是对代理友好的写法，而且它按注册树查询所报告的那份推导解析——因此代理可以把
/// `nichlink.registry` 给出的路径直接用在这里，不必翻译。
pub(crate) fn parent_id(
    root: &Path,
    namespace: &str,
    arguments: &Value,
    fields: &Value,
) -> Result<NodeId, String> {
    // `parent` is a sibling of `fields` in the request, because it names a place in
    // the tree rather than a field of the new face; `fields.parent` is accepted as
    // an alias so a caller that puts every value in one object still works.
    // `parent` 在请求里与 `fields` 平级，因为它命名的是树里的位置而不是新面的一个字段；
    // `fields.parent` 作为别名接受，因此把所有取值放进一个对象的调用方也能用。
    let configured = arguments
        .get("parent")
        .or_else(|| fields.get("parent"))
        .and_then(Value::as_str);
    match configured {
        // The default is the *namespaced* root: the bare `ROOT_NODE_ID` is a
        // different identity, and a face hung under it would report a logical path
        // of `root/...` while living in no tree the host compiled.
        // 默认值是**带命名空间的**根：裸的 `ROOT_NODE_ID` 是另一个身份，挂在它下面的面会报告
        // `root/...` 的逻辑路径，却不住在宿主编译过的任何树里。
        None | Some("") => Ok(nichlink_kernel::root_node_id(namespace)),
        Some(value) => match value.parse::<NodeId>() {
            Ok(id) => Ok(id),
            Err(_) => resolve_node(root, namespace, value),
        },
    }
}

/// Resolve a logical path or an identity to a face identity.
/// 把逻辑路径或身份解析成一个面的身份。
pub(crate) fn resolve_node(root: &Path, namespace: &str, target: &str) -> Result<NodeId, String> {
    if let Ok(id) = target.parse::<NodeId>() {
        return Ok(id);
    }
    let wanted = target.trim_start_matches('/');
    let mut faces = derived_faces(root, namespace)?.0;
    // The count is taken before the retain below: the refusal has to say how big the tree
    // actually is, and after the filter that number is always zero.
    // 计数在下面的 retain 之前取：拒绝文案要说的是这棵树到底有多大，而过滤之后那个数永远是零。
    let derived = faces.len();
    // The registry root has no face row of its own unless something declares it,
    // so `root` is answered from the namespace directly.
    // 注册树根没有自己的面行（除非有东西声明了它），因此 `root` 直接由命名空间作答。
    if wanted == "root" {
        return Ok(nichlink_kernel::root_node_id(namespace));
    }
    faces.retain(|face| face.path == wanted);
    match faces.len() {
        // A bare "no such face" leaves the caller guessing the tree's shape, and the
        // round-13 benchmark measured what that costs: a fresh agent spent five calls
        // trying `control`, `control::`, `crate`, `control::control` and an empty value
        // before it found `root`. Naming the root and the derived count is the way
        // forward, and `registry` is the call that lists the rest.
        // 干巴巴一句"没有这个面"把树的形状留给调用方去猜，第十三轮量出了代价：一个新代理试了
        // `control`、`control::`、`crate`、`control::control` 与空值共五次才摸到 `root`。点名根与
        // 推导出的面数就是那条继续走的路，而列出其余的是 `registry`。
        0 => Err(format!(
            "no registration face at `{target}`; this tree's root is `root` and it derives \
             {derived} face(s) — `registry` lists every face's logical path and kind"
        )),
        1 => Ok(faces[0].id),
        _ => Err(format!("`{target}` is ambiguous")),
    }
}

/// The faces this package derives, plus the one line naming any registration file the
/// derivation could not parse.
/// 本包推导出的面，外加一行：点名推导解析不了的注册面文件。
///
/// `face_views` returned only the first half, so a registration file that does not
/// parse (two macro invocations, a truncated field list) was dropped without a word: a
/// read-only tree query reported a smaller tree as if it were the whole one
/// (`LGC-LG-11`). The producer now hands the list over, and every reporting tool that
/// reads the tree says `unparsable faces N`, so no caller has to guess.
/// `face_views` 只返回前半，于是解析不了的注册面文件（两次宏调用、被截断的字段表）被一声不响地
/// 丢掉：只读的树查询把一棵更小的树当成完整的那棵报了出去（`LGC-LG-11`）。生产者现在把清单交出来，
/// 而每个读这棵树的报告工具都说出 `unparsable faces N`，因此没有调用方需要猜。
pub(crate) fn derived_faces(
    root: &Path,
    namespace: &str,
) -> Result<(Vec<FaceView>, String), String> {
    // Every derivation is counted, so "this answer read the build's published
    // records instead" is a checkable claim rather than a comment
    // (`published::derivations`). Test-only instrumentation: nothing in a shipped
    // bridge reads the count.
    // 每一次推导都被计数，因此"这份答案读的是构建已发布的记录"是可核对的声称而不是一句注释
    // （`published::derivations`）。这是仅供测试的埋点：出厂的桥里没有东西读这个计数。
    #[cfg(test)]
    crate::mcp::published::note_derivation();
    let (faces, unreadable, external) =
        crate::build_time::face_views_with_external(root, namespace)?;
    // External faces are not a defect and are not hidden either: a crate whose faces are
    // `external_object!` declarations is a crate the generated tree deliberately does not
    // contain, and a reader told only "faces 0" would file a bug against a working example.
    // 外部面不是缺陷，但也不该被藏起来：一个用 `external_object!` 声明面的 crate 是本包生成树有意不含的
    // crate，而只被告知"faces 0"的读者会去给一个能工作的示例报 bug。
    let mut line = String::new();
    if !external.is_empty() {
        line.push_str(&format!("external faces {}\n", external.len()));
        for entry in external.iter().take(5) {
            line.push_str(&format!("  {entry}\n"));
        }
    }
    let line = if unreadable.is_empty() {
        line
    } else {
        // The count alone leaves a reader holding a smaller tree with no way to learn why.
        // The reason is in this value already, and a registration file that does not parse is
        // the one thing a caller can act on — so the file and its complaint are printed beside
        // the count, capped, and the cap declares itself.
        // 只有计数会让读者拿到一棵更小的树、却无从得知原因。原因本来就在这个值里，而"注册面文件解析不了"
        // 正是调用方唯一能据以行动的东西——因此文件名与它的报错印在计数旁边，设上限，而上限自己声明自己。
        let mut text = line;
        text.push_str(&format!("unparsable faces {}\n", unreadable.len()));
        const REASONS: usize = 5;
        for reason in unreadable.iter().take(REASONS) {
            text.push_str(&format!("  {reason}\n"));
        }
        if unreadable.len() > REASONS {
            text.push_str(&format!(
                "  {}\n",
                crate::mcp::truncation::withheld(
                    unreadable.len() - REASONS,
                    unreadable.len(),
                    REASONS,
                    "unparsable reasons",
                    "run `nichlink check` for the build's own list"
                )
            ));
        }
        text
    };
    Ok((faces, line))
}

#[cfg(test)]
#[path = "resolve_tests.rs"]
mod resolve_tests;
