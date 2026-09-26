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

use std::path::Path;

use nichlink::NodeId;
use nichlink_build_method::face_views;
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
        None | Some("") => Ok(nichlink::root_node_id(namespace)),
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
    let mut faces = face_views(root, namespace)?;
    // The registry root has no face row of its own unless something declares it,
    // so `root` is answered from the namespace directly.
    // 注册树根没有自己的面行（除非有东西声明了它），因此 `root` 直接由命名空间作答。
    if wanted == "root" {
        return Ok(nichlink::root_node_id(namespace));
    }
    faces.retain(|face| face.path == wanted);
    match faces.len() {
        0 => Err(format!("no registration face at `{target}`")),
        1 => Ok(faces[0].id),
        _ => Err(format!("`{target}` is ambiguous")),
    }
}
