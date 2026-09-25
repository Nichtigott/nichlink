//! Deleting a generated registration module into the trash.
//! 把生成的注册模块删除进回收目录。

use std::fs;

use super::super::*;
use super::generated_paths;
use super::trash::trash_root;

/// Move one generated module subtree into the recoverable NichLink trash.
/// 将一个生成模块子树移动到可恢复的 NichLink 回收目录。
pub fn delete_module(registry: &Registry, spec: &str) -> Result<AuthoringChange, String> {
    let mut fields = spec.split_whitespace();
    let id = fields
        .next()
        .ok_or_else(|| "usage: delete <node> confirm".to_owned())?
        .parse::<NodeId>()
        .map_err(|_| "delete requires a 32-digit node identity".to_owned())?;
    if fields.next() != Some("confirm") || fields.next().is_some() {
        return Err("usage: delete <node> confirm".to_owned());
    }

    let (name, source) = generated_paths(registry, id)?;
    let module_dir = source
        .parent()
        .ok_or_else(|| "generated source has no module directory".to_owned())?;
    // The trash move takes the whole directory, so the directory has to be *this*
    // module's own: `<module>/<module>.rs`. A marker-carrying face that sits directly
    // under `src/` — a hand-made or flattened module — is listed as an ordinary
    // deletable face, and renaming its parent would take all of `src/` with it. The
    // rename path already refuses this shape for the same reason; deleting needs the
    // same guard because it moves a directory, not a file.
    // 回收搬走的是整个目录，因此该目录必须是**这个**模块自己的：`<module>/<module>.rs`。
    // 带标记却直接位于 `src/` 下的面——手写的或已被拍平的模块——会像普通面一样被列为可删除，而
    // 重命名它的父目录会把整个 `src/` 一起带走。改名路径出于同样的理由已经拒绝这种形状；删除同样
    // 需要这道守卫，因为它搬的是目录而不是文件。
    let stem = source
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if module_dir.file_name().and_then(|name| name.to_str()) != Some(stem) {
        return Err("only standard `<module>/<module>.rs` faces can be deleted".to_owned());
    }
    let trash = trash_root().join(format!("{name}-{id}-{}", super::trash::stamp()?));
    fs::create_dir_all(trash.parent().expect("trash has a parent"))
        .map_err(|error| format!("cannot create NichLink trash: {error}"))?;
    fs::rename(module_dir, &trash)
        .map_err(|error| format!("cannot move module to trash: {error}"))?;

    Ok(AuthoringChange {
        message: format!("moved `{name}` to {}", trash.display()),
        source: trash,
    })
}
