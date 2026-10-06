//! The crate-shape rule: which registration subtrees a host asks to become crates.
//! crate 形状规则：宿主要求把哪些注册子树加成 crate。
//!
//! A host declares its shape in one file at the package root (`add_crates.rs`), as ordinary Rust:
//! a [`Shape`](https://docs.rs) value whose crates name subtrees by the generated modules' markers.
//! The **rule** about what a valid declaration is lives here, once, because two readers need it and
//! they see different things: the host's own crate compiles the declaration (so a wrong path is a
//! compile error), and the build script reads the same file as text (so a wrong shape is a build
//! error even before anything compiles). Two copies of "overlap" would drift; one cannot.
//! 宿主在包根一个文件（`add_crates.rs`）里、用普通 Rust 声明它的形状：一个 [`Shape`] 值，其中的 crate 用
//! 生成模块的标记来点名子树。**规则**——什么算一份合法声明——只住在这里一次，因为有两个读者、而他们看到的
//! 东西不同：宿主自己的 crate 会**编译**这份声明（路径写错就是编译错误），而构建脚本把同一个文件当**文本**
//! 读（形状写错在编译之前就是构建错误）。"重叠"这条规则有两份副本就会漂，一份不会。
//!
//! Paths are `::`-separated **module paths** (`control::object`), not file paths and not logical
//! registration paths: the marker a declaration names comes from `module_path!()`, and that is what
//! both readers can agree on without guessing.
//! 路径是 `::` 分隔的**模块路径**（`control::object`），不是文件路径、也不是注册逻辑路径：声明点名的标记来自
//! `module_path!()`，而那是两个读者不必猜就能一致的东西。

/// One crate a host declared, as the rule sees it: a name and the subtrees it claims.
/// 宿主声明的一个 crate，按规则所见：一个名字，以及它认领的子树。
#[derive(Clone, Copy, Debug)]
pub struct DeclaredCrate<'a> {
    /// The crate's name, used as the suffix of its published package name.
    /// 这个 crate 的名字，用作它发布包名的后缀。
    pub name: &'a str,
    /// The registration subtrees it claims, as `::`-separated module paths.
    /// 它认领的注册子树，`::` 分隔的模块路径。
    pub subtrees: &'a [&'a str],
}

/// Whether one subtree contains or equals the other.
/// 一棵子树是否包含另一棵、或与它相同。
///
/// The boundary is a whole `::` segment, exactly like the scope's own subtree selector: `control`
/// contains `control::object` and never `control_extra`.
/// 边界是整个 `::` 段，与作用域自己的子树选择器完全一致：`control` 包含 `control::object`，绝不包含
/// `control_extra`。
pub fn subtree_overlaps(left: &str, right: &str) -> bool {
    let (short, long) = if left.len() <= right.len() {
        (left, right)
    } else {
        (right, left)
    };
    long == short
        || (long.len() > short.len()
            && long.starts_with(short)
            && long.as_bytes().get(short.len()) == Some(&b':'))
}

/// Why a declared shape is not one this framework can build, ready to print.
/// 一份声明为什么不是本框架能构建的形状，可直接打印。
///
/// Every refusal names the crate and the subtree it came from: a shape is edited by hand or by a
/// tool, and "invalid shape" without which line is a message the caller has to reverse-engineer.
/// 每条拒绝都点名它来自哪个 crate、哪棵子树：形状是手写或工具写的，而一句不说是哪一行的"形状无效"，会让
/// 调用方不得不反推。
pub fn validate_shape(package_prefix: &str, crates: &[DeclaredCrate<'_>]) -> Result<(), String> {
    if package_prefix.trim().is_empty() {
        return Err(
            "add_crates: `package_prefix` is empty; it is the prefix every published crate name \
             is built from"
                .to_owned(),
        );
    }
    for (index, krate) in crates.iter().enumerate() {
        if krate.name.trim().is_empty() {
            return Err("add_crates: a crate needs a name".to_owned());
        }
        if krate.subtrees.is_empty() {
            return Err(format!(
                "add_crates: `{}` names no subtree, so it would be an empty crate",
                krate.name
            ));
        }
        for subtree in krate.subtrees {
            if subtree.trim().is_empty() {
                return Err(format!(
                    "add_crates: `{}` names an empty subtree",
                    krate.name
                ));
            }
        }
        for other in &crates[index + 1..] {
            if krate.name == other.name {
                return Err(format!(
                    "add_crates: `{}` is declared twice; a shape has one line per crate",
                    krate.name
                ));
            }
        }
        for (position, subtree) in krate.subtrees.iter().enumerate() {
            for nested in &krate.subtrees[position + 1..] {
                if subtree_overlaps(subtree, nested) {
                    return Err(format!(
                        "add_crates: `{}` names `{subtree}` and `{nested}`, which overlap; a face \
                         belongs to exactly one crate",
                        krate.name
                    ));
                }
            }
            for other in &crates[index + 1..] {
                for claimed in other.subtrees {
                    if subtree_overlaps(subtree, claimed) {
                        return Err(format!(
                            "add_crates: `{}` and `{}` both claim `{subtree}`/`{claimed}`; a face \
                             belongs to exactly one crate",
                            krate.name, other.name
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "shape_tests.rs"]
mod shape_tests;
