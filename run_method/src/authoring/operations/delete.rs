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
