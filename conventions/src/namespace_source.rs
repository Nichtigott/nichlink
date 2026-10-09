//! The namespace-source gate: a face file reads the crate-root constant, not the package name.
//! 命名空间来源门禁：面文件读 crate 根的常量，不读包名。
//!
//! Identity is `hash(namespace, source path, declared name)`, and since audit `M7` / P3.3 the
//! declaration macros read `crate::XIRANG_NAMESPACE` — a constant the crate root owns — instead of
//! `env!("CARGO_PKG_NAME")` at each declaration site. Why it matters is exactly what a partition
//! does: a crate that mounts one face file through another `#[path]` would make that file read the
//! **ghost crate's** package name, and every face it declares would be silently renamed.
//! 身份是 `hash(命名空间, 源码路径, 声明名)`，而自审计 `M7` / P3.3 起，声明宏读的是 crate 根持有的
//! `crate::XIRANG_NAMESPACE` 常量，而不是在每个声明处读 `env!("CARGO_PKG_NAME")`。为什么这件事要紧，
//! 正是分区所做的事：一个经另一个 `#[path]` 挂载同一份面文件的 crate，会让那份文件读到**幽灵 crate 的**
//! 包名，于是它声明的每一个面都被静默改名。
//!
//! Scope, stated because a gate that cries wolf gets switched off. Judged: **code lines of a crate's
//! `src/` tree that invoke a declaration macro**, and inside those, every spelling of the package name
//! except the one that *defines* the constant. Not judged, each for a reason: a path under `tests/`
//! (integration tests and the checkout-only fixture are test crates that own their own constant, and
//! the parser's string fixtures are not compiled at all) · comment and doc lines (a document may and
//! must talk *about* the old spelling) · a spelling that is not an invocation (the name inside a
//! string or a const array).
//! 作用域写在这里，因为乱叫的门禁会被关掉。被判定的是：**某个 crate `src/` 树里调用声明宏的代码行**，而在
//! 这些行里，包名的每一种拼法——**定义**常量那一行除外。不被判定的，各有理由：`tests/` 下的路径（集成测试
//! 与仅检出可用的夹具是给自己常量下定义的测试 crate，而解析器的字符串夹具根本不编译）· 注释与文档行（文档
//! 可以、而且必须**谈论**旧拼法）· 不是调用的拼法（字符串里或常量数组里的名字）。

use std::fs;
use std::path::{Path, PathBuf};

use crate::{crate_directories, relative, rust_sources};

/// The spellings that declare a registration face — the hidden primitives and the aliases a generated
/// tree emits for a host (`<parent>_object!`).
/// 声明注册面的那些拼法——隐藏原语，以及生成树为宿主发出的别名（`<parent>_object!`）。
const DECLARATIONS: &[&str] = &[
    "__xirang_object!",
    "__external_object!",
    "__registration_face!",
    "root_object!",
    "external_object!",
];

/// The spelling a face file has to use instead.
/// 面文件必须改用的那个拼法。
const CONSTANT: &str = "crate::XIRANG_NAMESPACE";

/// The retired spelling.
/// 已退役的那个拼法。
const PACKAGE_NAME: &str = "env!(\"CARGO_PKG_NAME\")";

/// One face file that still reads the package name for its identity.
/// 一份仍然为身份读包名的面文件。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Finding {
    /// File relative to the workspace root, with `/` separators.
    /// 以工作区根为基准的文件路径，使用 `/` 分隔符。
    pub file: String,
    /// One-based line the spelling sits on.
    /// 该拼法所在行（从 1 开始）。
    pub line: usize,
}

impl Finding {
    /// The line a reader has to change, and why.
    /// 读者必须改的那一行，以及理由。
    pub fn reason(&self) -> String {
        format!(
            "{}:{} declares a registration face and reads `{PACKAGE_NAME}` for its identity; a \
             partitioned crate mounts this file through another `#[path]`, where that reads the \
             ghost crate's name and silently renames every face. Read `{CONSTANT}` instead (the \
             constant `host!()` defines at the crate root, audit `M7`, P3.3).",
            self.file, self.line
        )
    }
}

/// Every face file that reads the package name where it must read the constant.
/// 每一份在本该读常量的地方读了包名的面文件。
pub fn findings(root: &Path) -> Vec<Finding> {
    let sources: Vec<PathBuf> = crate_directories(root)
        .into_iter()
        .flat_map(|directory| rust_sources(&directory.join("src")))
        .filter(|path| !under_tests(path))
        .collect();
    findings_in(root, &sources)
}

/// The same rule over an explicit list of sources, so it can be judged on fixtures.
/// 同一条规则作用于一份显式的源码清单，因此可以在夹具上判定。
pub fn findings_in(root: &Path, sources: &[PathBuf]) -> Vec<Finding> {
    let mut found = Vec::new();
    for path in sources {
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        if !declares_a_face(&text) {
            continue;
        }
        for (index, line) in text.lines().enumerate() {
            if !line.contains(PACKAGE_NAME) || is_comment(line) {
                continue;
            }
            // The constant's own definition is where the value legitimately comes from.
            // 常量自身的定义正是那个值正当的来源。
            if line.contains("XIRANG_NAMESPACE") {
                continue;
            }
            found.push(Finding {
                file: relative(root, path),
                line: index + 1,
            });
        }
    }
    found.sort();
    found.dedup();
    found
}

/// Whether a path is a test file rather than a compiled face file: under a `tests` directory
/// (crate-root targets and in-crate test trees), or the sibling `<name>_tests.rs` shape every module
/// in this workspace uses for its tests.
/// 路径是否是测试文件而不是被编译的面文件：位于 `tests` 目录之下（crate 根目标与 crate 内的测试树），
/// 或者是本工作区每个模块给测试用的 `<name>_tests.rs` 同级形状。
///
/// Tests are where the old spelling belongs: a parser fixture *quotes* a declaration, and an assertion
/// computes the value with the package name to prove the constant is what the macro read.
/// 测试正是旧拼法该待的地方：解析器夹具**引用**一条声明，而断言用包名算出那个值，以证明宏读的是常量。
fn under_tests(path: &Path) -> bool {
    if path
        .components()
        .any(|component| component.as_os_str() == "tests")
    {
        return true;
    }
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with("_tests.rs"))
}

/// Whether this line is comment or documentation text.
/// 这一行是否是注释或文档文本。
fn is_comment(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("//") || trimmed.starts_with('*')
}

/// Whether this source **invokes** a declaration macro: the spelling followed by an opening brace or
/// parenthesis. A name inside a string (this rule's own list, a parser fixture) is not an invocation.
/// 这份源码是否**调用**了声明宏：那个拼法后面跟着花括号或圆括号。字符串里的名字（本规则自己的清单、
/// 解析器的夹具）不是调用。
fn declares_a_face(text: &str) -> bool {
    for line in text.lines() {
        if is_comment(line) {
            continue;
        }
        for spelling in DECLARATIONS {
            let Some(at) = line.find(spelling) else {
                continue;
            };
            if let Some(next) = line[at + spelling.len()..].chars().next()
                && (next == '{' || next == '(' || next.is_whitespace())
                && line[at + spelling.len()..]
                    .trim_start()
                    .starts_with(['{', '('])
            {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
#[path = "namespace_source_tests.rs"]
mod namespace_source_tests;
