use super::*;

/// Return the part of `value` after `prefix`, or `None` when it is not one.
/// 返回 `value` 中 `prefix` 之后的部分；`prefix` 不匹配时返回 `None`。
///
/// Written with scalar indexing and `split_at` because range indexing and
/// `slice::get` are not usable in constant functions on the supported
/// toolchain. 使用标量索引与 `split_at`：在受支持的工具链上，范围索引和
/// `slice::get` 无法用于常量函数。
///
/// Internal to the identity module: the two helpers that need it from outside
/// (`manifest_relative_source`, `strip_path_prefix`) are exported instead.
/// 身份模块内部使用：真正需要它对外的两个辅助函数（`manifest_relative_source`、
/// `strip_path_prefix`）已经导出。
pub(crate) const fn strip_prefix<'a>(value: &'a str, prefix: &str) -> Option<&'a str> {
    let value_bytes = value.as_bytes();
    let prefix_bytes = prefix.as_bytes();
    if prefix_bytes.len() > value_bytes.len() {
        return None;
    }
    let mut index = 0;
    while index < prefix_bytes.len() {
        if value_bytes[index] != prefix_bytes[index] {
            return None;
        }
        index += 1;
    }
    let (_, rest) = value_bytes.split_at(index);
    match core::str::from_utf8(rest) {
        Ok(text) => Some(text),
        Err(_) => None,
    }
}

/// Like `strip_prefix`, but `/` and `\` count as the same separator.
/// 与 `strip_prefix` 相同，但 `/` 与 `\` 视为同一分隔符。
///
/// Cargo reports `CARGO_MANIFEST_DIR` with backslashes on Windows while the
/// build step writes `#[path]` literals with forward slashes, so an exact
/// comparison would fail there and leave the absolute path as the identity.
/// Cargo 在 Windows 上以反斜杠给出 `CARGO_MANIFEST_DIR`，而构建步骤写出的
/// `#[path]` 字面量用正斜杠；精确比较在那边会失败，身份会退化成绝对路径。
pub const fn strip_path_prefix<'a>(value: &'a str, prefix: &str) -> Option<&'a str> {
    let value_bytes = value.as_bytes();
    let prefix_bytes = prefix.as_bytes();
    if prefix_bytes.len() > value_bytes.len() {
        return None;
    }
    let mut index = 0;
    while index < prefix_bytes.len() {
        let left = value_bytes[index];
        let right = prefix_bytes[index];
        if left != right && !(is_separator(left) && is_separator(right)) {
            return None;
        }
        index += 1;
    }
    let (_, rest) = value_bytes.split_at(index);
    // The prefix has to end on a component boundary: `/work/app` is not a path
    // prefix of `/work/application/src/a.rs`, and stripping it there would invent
    // the identity `lication/src/a.rs` for a file that was never inside the
    // project.
    // 前缀必须结束在组件边界上：`/work/app` 不是 `/work/application/src/a.rs` 的路径
    // 前缀，在那里剥离会为一个从来不在项目里的文件凭空造出身份
    // `lication/src/a.rs`。
    // A prefix that already ends with a separator is on a boundary by itself;
    // otherwise the character after the prefix has to be one.
    // 前缀本身以分隔符结尾时它已经落在边界上；否则前缀之后必须是分隔符。
    // `slice::last`/`Option::is_some_and` are not const-stable here, so the last
    // byte is read by index like the loop above does.
    // `slice::last`/`Option::is_some_and` 在本工具链上不是 const 稳定的，因此像上面的
    // 循环一样用下标读取最后一个字节。
    let prefix_ends_on_separator = match prefix_bytes.len() {
        0 => false,
        length => is_separator(prefix_bytes[length - 1]),
    };
    if !prefix_ends_on_separator
        && let Some(next) = rest.first()
        && !is_separator(*next)
    {
        return None;
    }

    match core::str::from_utf8(rest) {
        Ok(text) => Some(text),
        Err(_) => None,
    }
}

/// Last `::`-separated segment of a module path.
/// 模块路径中最后一段 `::` 分隔的分量。
///
/// Public because the declaration macros call it from the host crate to derive
/// a face's registry name from `module_path!()`.
/// 公开是因为注册面宏在宿主 crate 里调用它，从 `module_path!()` 推导注册机名。
pub const fn last_path_segment(path: &str) -> &str {
    let bytes = path.as_bytes();
    let mut index = bytes.len();
    while index > 0 {
        index -= 1;
        if bytes[index] == b':' {
            let (_, rest) = bytes.split_at(index + 1);
            return match core::str::from_utf8(rest) {
                Ok(text) => text,
                Err(_) => path,
            };
        }
    }
    path
}

/// Derive a declaration's `source` identity input from `file!()`.
/// 从 `file!()` 推导声明的 `source` 身份输入。
///
/// Drops `manifest_dir` and one leading `src/` component so the value matches
/// the repository-relative path the build step used to inject, which keeps
/// `NodeId` stable across the generated and derived forms. The original value
/// is returned when `file` is not under `manifest_dir` (an external crate, a
/// `tests/` target, or a differently laid out package).
/// 去掉 `manifest_dir` 与其后的一个 `src/` 分量，使结果与构建步骤原本注入的
/// 仓库相对路径一致，从而让生成式与推导式产生相同的 `NodeId`。当 `file`
/// 不在 `manifest_dir` 下（外部 crate、`tests/` 目标或其它布局）时返回原值。
pub const fn manifest_relative_source<'a>(manifest_dir: &str, file: &'a str) -> &'a str {
    let after_manifest = match strip_path_prefix(file, manifest_dir) {
        Some(rest) => rest,
        None => return file,
    };
    let after_separator = strip_leading_separator(after_manifest);
    match strip_prefix(after_separator, "src/") {
        Some(rest) => rest,
        None => match strip_prefix(after_separator, "src\\") {
            Some(rest) => rest,
            None => after_separator,
        },
    }
}
