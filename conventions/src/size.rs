//! The file-size gate, as a ratchet rather than a wish.
//! 文件尺寸门禁，以棘轮而不是愿望的形式。
//!
//! `docs/roadmap-1.0.md` recorded the size pass as done and stated that
//! non-test source files stay at or under a fixed line count. The fourth audit
//! round measured the tree and found fifteen files over the count then documented,
//! the largest at 751 lines. A ceiling nothing measures is not a ceiling, so this
//! module measures it and pins the current exceptions explicitly.
//! `docs/roadmap-1.0.md` 把尺寸收口记为完成，并声明非测试源码文件不超过一个固定行数。第四轮
//! 审计实测源码树，发现 15 个文件超过当时声明的那个数字，最大 751 行。没有东西去量的上限不是
//! 上限，因此本模块去量它，并把当前的例外显式钉住。
//!
//! The maintainer raised the ceiling from 450 to 600 on 2026-09-28. Which file a
//! piece of code belongs in is a judgement about cohesion, and a line count tight
//! enough to pre-empt that judgement pushes code out of the file it belongs in.
//! The ratchet still bounds growth; it no longer decides placement.
//! 维护者于 2026-09-28 把上限从 450 提到 600。一段代码该待在哪个文件是有关内聚性的判断，而一个
//! 紧到能抢在那个判断之前生效的行数，会把代码挤出它本该在的文件。棘轮仍然约束增长，但不再替归属
//! 做决定。
//!
//! The list is short and only shrinks: moving the shared JSON encoder out of
//! `diagnostic/build.rs` brought that file back under the ceiling, and the ratchet made
//! removing its entry mandatory rather than optional. The count is deliberately not
//! restated here — it is `BASELINE.len()`, and prose that repeats a number drifts from
//! it (this sentence claimed twelve while the list held nine).
//! 这份清单很短，而且只会变短：把共用的 JSON 编码器移出 `diagnostic/build.rs` 让该文件缩回
//! 上限之内，而棘轮让删除对应项成为必然而不是可选。条目数刻意不在这里复述——它就是
//! `BASELINE.len()`，而复述数字的散文终会与它漂移（这句话曾声称十二项，而清单只有九项）。
//!
//! The ratchet has two teeth, and both matter. A newly oversized file fails the
//! gate, and a baseline entry that has shrunk back under the ceiling also fails
//! it, because a stale exception list is how a ratchet quietly becomes
//! permission. The list can therefore only shrink, and shrinking it is a
//! one-line change with a visible diff.
//! 这道棘轮有两齿，两齿都重要。新增超标文件会让门禁失败；某个基线项已缩回上限之内同样会
//! 让门禁失败，因为过期的例外清单正是一道棘轮悄悄变成许可证的方式。因此这份清单只能变短，
//! 而缩短它是一行改动，diff 可见。
//!
//! Boundary: test-only files are exempt, because the ceiling is about the code a
//! maintainer reads to understand behaviour. A file under a `tests/` directory is
//! test-only by where it sits; a file whose *name* looks test-only has to be mounted
//! behind `#[cfg(test)]` as well, because a name is not proof — renaming real code to
//! `x_tests.rs` used to move it out of the measurement silently. The crate-root
//! `build.rs` is measured like `src/`: it is source a maintainer reads.
//! 边界：仅测试文件豁免，因为上限针对的是维护者为了理解行为而要读的代码。`tests/` 目录下的文件
//! 按位置就是仅测试的；而**名字**看起来像测试的文件还必须被挂在 `#[cfg(test)]` 之后，因为名字不是
//! 证明——把真实代码改名成 `x_tests.rs` 过去就能静默地把它移出度量。crate 根的 `build.rs` 与
//! `src/` 一同度量：它是维护者要读的源码。

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::{crate_directories, lines, relative, rust_sources};

/// The documented ceiling, in lines, for a non-test source file.
/// 文档声明的非测试源码文件行数上限。
pub const CEILING: usize = 600;

/// Files that were already over [`CEILING`] when the gate was added.
/// 门禁加入时就已经超过 [`CEILING`] 的文件。
///
/// Each entry is a debt with a measured size, not a permission. Remove an entry
/// the moment the file comes back under the ceiling; the gate fails on a stale
/// entry precisely so that removal cannot be forgotten.
/// 每一项都是带实测大小的欠账，不是许可。文件缩回上限之内的那一刻就删掉对应项；门禁会在
/// 过期项上失败，正是为了让"忘记删除"不可能发生。
///
/// How many entries the list holds is deliberately not restated in prose: it is
/// [`BASELINE.len()`], and a number written here drifts from the list. Audit `G-17` found the
/// roadmap's version of this sentence claiming a count ten times the list's own.
/// 清单有几项刻意不在散文里复述：它就是 [`BASELINE.len()`]，而写在这里的数字终会与清单漂移。
/// 审计 `G-17` 发现路线图那句自称的项数是清单实际项数的十倍。
/// The list is empty: `contracts.rs` was the last entry, and splitting its test module into
/// `contracts_tests.rs` (the sanctioned `#[path]` mount, which the ceiling exempts) brought it
/// back under the ceiling — so the ratchet makes removing the entry mandatory rather than
/// optional, exactly as its own rule says.
/// 清单已空：`contracts.rs` 是最后一项，而把它的测试模块拆进 `contracts_tests.rs`（受认可的
/// `#[path]` 挂载，上限对其豁免）让它缩回上限之内——因此棘轮让删除该项成为必然而不是可选，
/// 这正是它自己的规则所说的。
pub const BASELINE: &[(&str, usize)] = &[];

/// Whether a path sits in a `tests/` directory.
/// 该路径是否位于 `tests/` 目录中。
///
/// Location is proof on its own: a file there is a test whatever it is called, and
/// nothing else in the tree can be mistaken for it.
/// 位置本身就是证明：那里的文件无论叫什么都是测试，树里也没有别的东西会被误认成它。
pub fn is_test_by_location(path: &Path) -> bool {
    path.components()
        .any(|component| component.as_os_str() == "tests")
}

/// Whether a path's *name* looks test-only.
/// 该路径的**名字**是否看起来是仅测试的。
///
/// This is a hint, not proof: it is used together with the private
/// `is_mounted_as_test`, because a file can be renamed.
/// 这是提示而不是证明：它与私有的 `is_mounted_as_test` 配合使用，因为文件可以改名。
pub fn is_test_shaped(path: &Path) -> bool {
    let stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    stem == "test" || stem == "tests" || stem.starts_with("test_") || stem.ends_with("_tests")
}

/// Whether a path looks test-only by location or by name.
/// 该路径是否按位置或名字看起来是仅测试的。
///
/// The weaker of the two predicates, for a caller that only has to decide "do not
/// treat this as documentation a reader is shown". [`oversized`] uses the stricter
/// rule — a name plus a `#[cfg(test)]` mount — because the ceiling is a budget, and a
/// name can lie.
/// 两个谓词中较弱的一个，供只需要判断"别把这里当成给读者看的文档"的调用方使用。[`oversized`]
/// 用的是更严格的规则——名字加 `#[cfg(test)]` 挂载——因为上限是预算，而名字会说谎。
pub fn looks_test_only(path: &Path) -> bool {
    is_test_by_location(path) || is_test_shaped(path)
}

/// Whether a crate mounts `path` behind `#[cfg(test)]`, directly or through an ancestor.
/// crate 是否把 `path` 挂在 `#[cfg(test)]` 之后——直接挂，或经由某个祖先。
///
/// The two spellings in this tree are `#[path = "x_tests.rs"] mod x_tests;` — what
/// the mounting convention writes — and the bare `mod tests;` that resolves to a
/// sibling `tests.rs`. `#[cfg(test)]` may sit above other attributes, and rustfmt
/// keeps the group adjacent, so the walk back crosses attributes and comments.
/// 本树里的两种拼法是 `#[path = "x_tests.rs"] mod x_tests;`（挂载约定所写）与解析到同级
/// `tests.rs` 的裸 `mod tests;`。`#[cfg(test)]` 可能位于其他属性之上，而 rustfmt 让这一组保持
/// 相邻，因此回溯会跨过属性与注释。
///
/// The attribute is inherited down the chain: `app.rs` mounts `tests` behind `#[cfg(test)]`,
/// and `tests.rs` then mounts `call_tree.rs` without repeating it. A file is therefore
/// test-only when the *chain* reaches one, which is why this recurses. The bound is there
/// because a cycle in a hand-written declaration list is not worth hanging a gate for.
/// 该属性沿链继承：`app.rs` 把 `tests` 挂在 `#[cfg(test)]` 之后，`tests.rs` 再挂
/// `call_tree.rs` 时不必重复它。因此只要**链**上有一处，文件就是仅测试的——这正是递归的原因。
/// 有界，是因为手工写的声明列表若成环，不值得让门禁挂住。上限是**带余量的防环，不是对树深度的
/// 描述**：2026-09-28 实测本树出厂的最深挂载链有 **6 个文件**（也就是 5 条挂载边）：`kernel/src/lib.rs`
/// → `registry_core.rs` → `authoring/authoring.rs` → `authoring/parse/parse.rs` →
/// `authoring/parse/flow.rs` → `authoring/parse/flow_tests.rs`。这里说的"层"是**链上的文件数**，
/// 边数比它少一——旧文只列了链尾的四个文件却声称 6 层，读者无法判断该以哪个为准。当时取的是 8
/// ——只剩两层的余量，再嵌两层就会让某个测试文件不再豁免（门禁会报出那个文件：响亮，但不是我们
/// 想要的失败）。因此取 16，并由 `a_deep_mount_chain_is_still_test_only` 用一条十一层的链钉住这份
/// 余量。**`doc_blocks` 也用它**：同一个"这个文件是不是仅测试"的身份问题，两道门禁必须给同一条规则
/// （`tree/graft_ops/fixtures.rs` 就是名字不像测试、却挂在 `#[cfg(test)]` 之后的真实例子）。
pub(crate) fn is_mounted_as_test(crate_root: &Path, path: &Path) -> bool {
    is_mounted_as_test_within(crate_root, path, 16)
}

fn is_mounted_as_test_within(crate_root: &Path, path: &Path, depth: usize) -> bool {
    if depth == 0 {
        return false;
    }
    let Some((declarer, cfg_test)) = declaration_of(crate_root, path) else {
        return false;
    };
    cfg_test || is_mounted_as_test_within(crate_root, &declarer, depth - 1)
}

/// The file that declares `path`, with whether that declaration carries `#[cfg(test)]`.
/// 声明 `path` 的那个文件，以及该声明是否带着 `#[cfg(test)]`。
fn declaration_of(crate_root: &Path, path: &Path) -> Option<(std::path::PathBuf, bool)> {
    let file_name = path.file_name().and_then(|name| name.to_str())?;
    let stem = path.file_stem().and_then(|stem| stem.to_str())?;
    let attribute = format!("\"{file_name}\"");
    let declaration = format!("mod {stem};");
    for candidate in rust_sources(crate_root) {
        let Ok(text) = std::fs::read_to_string(&candidate) else {
            continue;
        };
        let source: Vec<&str> = text.lines().collect();
        for (index, line) in source.iter().enumerate() {
            let trimmed = line.trim();
            let declares = (trimmed.starts_with("#[path") && line.contains(&attribute))
                || declares_module(trimmed, &declaration);
            if !declares {
                continue;
            }
            let mut cursor = index;
            while cursor > 0 {
                cursor -= 1;
                let previous = source[cursor].trim();
                if is_cfg_test(previous) {
                    return Some((candidate.clone(), true));
                }
                if previous.starts_with("//") || previous.starts_with("#[") || previous.is_empty() {
                    continue;
                }
                break;
            }
            return Some((candidate.clone(), false));
        }
    }
    None
}

/// Whether a line declares `stem` as a module, whatever visibility it carries.
/// 该行是否把 `stem` 声明为模块，无论它带着什么可见性限定。
///
/// Comparing the trimmed line against `mod x;` and `pub mod x;` read every other spelling as
/// *not* a declaration: `pub(crate) mod x;`, `pub(super) mod x;`, `pub(in crate::a) mod x;`
/// all mount the same file, and a test module mounted that way was then measured as source —
/// a debt report on a file this gate's own rule calls exempt. Audit `G-04`.
/// 把去掉缩进的行与 `mod x;`、`pub mod x;` 比较，会把其余每一种拼法读成**不是**声明：
/// `pub(crate) mod x;`、`pub(super) mod x;`、`pub(in crate::a) mod x;` 挂载的是同一个文件，而
/// 以这些方式挂载的测试模块会被当作源码度量——在一份本门禁自己的规则已判为豁免的文件上报出欠账。
/// 审计 `G-04`。
fn declares_module(trimmed: &str, declaration: &str) -> bool {
    let rest = without_visibility(trimmed);
    // The two spellings this tree writes, compared without building anything. This runs for
    // every line of every file the walk reads, so the cheap path is the one that matters.
    // 本树书写的两种拼法，直接比较、不构造任何东西。这段代码会对遍历读到的每个文件的每一行运行，
    // 因此重要的是这条便宜路径。
    if rest == declaration {
        return true;
    }
    // `mod` has to end at an identifier boundary: `modx;` shares the keyword's first three
    // characters without being the keyword, and a bare prefix test is what lets it through.
    // `mod` 必须结束在标识符边界上：`modx;` 与关键字共用前三个字符，却并不是这个关键字，而只比
    // 前缀正是让它通过的原因。
    let Some(after_keyword) = rest.strip_prefix("mod") else {
        return false;
    };
    if !after_keyword.starts_with(char::is_whitespace) {
        return false;
    }
    // Whitespace is not part of the declaration: `mod  x ;` is the same declaration to rustc.
    // The earlier token join did not deliver that — it left the space before the `;` in place, so
    // a test module mounted that way was still measured as source (audit `G-07`, finding `F-2`).
    // 空白不属于声明：对 rustc 来说 `mod  x ;` 是同一条声明。先前那次词元连接没做到这一点——
    // 它把 `;` 前的空格留在了原地，于是那样挂载的测试模块仍会被当作源码度量（审计 `G-07`，
    // 发现 `F-2`）。
    without_whitespace(rest) == without_whitespace(declaration)
}

/// `text` without any whitespace, so the comparison ignores where the whitespace sits.
/// `text` 去掉所有空白，使比较不关心空白落在哪里。
fn without_whitespace(text: &str) -> String {
    text.chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

/// `line` without a leading `pub` / `pub(crate)` / `pub(super)` / `pub(self)` / `pub(in …)`.
/// `line` 去掉开头的 `pub` / `pub(crate)` / `pub(super)` / `pub(self)` / `pub(in …)`。
fn without_visibility(line: &str) -> &str {
    let Some(rest) = line.strip_prefix("pub") else {
        return line;
    };
    if let Some(rest) = rest.strip_prefix('(') {
        let Some(close) = rest.find(')') else {
            // `publish = …` is a key, not a visibility with a broken qualifier.
            // `publish = …` 是一个键，而不是限定符写坏的可见性。
            return line;
        };
        return rest[close + 1..].trim_start();
    }
    // `pub` has to end at an identifier boundary: `publish = false` is not a visibility.
    // `pub` 必须结束在标识符边界上：`publish = false` 不是可见性。
    if rest
        .chars()
        .next()
        .is_some_and(|next| next.is_alphanumeric() || next == '_')
    {
        return line;
    }
    rest.trim_start()
}

/// Whether an attribute line is a `#[cfg(…)]` that is only true in a test build.
/// 该属性行是否是"只在测试构建里为真"的 `#[cfg(…)]`。
///
/// `#[cfg(all(test))]` is the same condition as `#[cfg(test)]`, and a literal comparison
/// against the second spelling read the first as no test mount at all. The expression is read
/// as an expression rather than as text, because the operators are not interchangeable:
/// `all(test, feature = "x")` and `any(test, feature = "x")` differ in exactly the way this
/// predicate has to care about, and a `test` inside `not(...)` or inside a string is not a
/// test build. Audit `G-04`.
/// `#[cfg(all(test))]` 与 `#[cfg(test)]` 是同一个条件，而拿第二种拼法做字面比较会把第一种读成
/// 根本没有测试挂载。这里把表达式当表达式读而不是当文本读，因为算子不可互换：
/// `all(test, feature = "x")` 与 `any(test, feature = "x")` 的差别正是本判定必须关心的，而
/// `not(...)` 里或字符串里的 `test` 并不表示测试构建。审计 `G-04`。
fn is_cfg_test(line: &str) -> bool {
    let Some(expression) = cfg_expression(line) else {
        return false;
    };
    cfg_is_test_only(expression)
}

/// The expression inside `#[cfg(…)]`, when the line is such an attribute.
/// 该行是 `#[cfg(…)]` 时，括号里的表达式。
fn cfg_expression(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("#[cfg")?.trim_start();
    let rest = rest.strip_prefix('(')?;
    let mut depth = 1usize;
    for (offset, character) in rest.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&rest[..offset]);
                }
            }
            _ => {}
        }
    }
    None
}

/// Whether a `cfg` expression is true only in a test build.
/// 该 `cfg` 表达式是否只在测试构建里为真。
fn cfg_is_test_only(expression: &str) -> bool {
    let expression = expression.trim();
    match cfg_call(expression) {
        // Every argument has to hold, so one test-only argument makes the whole thing
        // test-only — and an empty list is true in no build at all.
        // 每个实参都必须成立，因此只要有一个仅测试的实参，整体就是仅测试的——而空列表在任何构建里都不成立。
        Some(("all", arguments)) => arguments.iter().any(|argument| cfg_is_test_only(argument)),
        // An empty `any()` is never true, and anything else has to be test-only in *every*
        // branch, or an ordinary build reaches the module too.
        // 空的 `any()` 永远不成立；其余情况下每个分支都必须仅测试，否则普通构建也能到达该模块。
        Some(("any", arguments)) => {
            !arguments.is_empty() && arguments.iter().all(|argument| cfg_is_test_only(argument))
        }
        Some(("not", _)) => false,
        _ => expression == "test",
    }
}

/// A `cfg` call's name and its top-level arguments, when the expression is a call.
/// 表达式是一次调用时，`cfg` 调用的名字与它的顶层实参。
fn cfg_call(expression: &str) -> Option<(&str, Vec<&str>)> {
    let open = expression.find('(')?;
    let name = expression[..open].trim();
    if !expression.ends_with(')') {
        return None;
    }
    let inner = &expression[open + 1..expression.len() - 1];
    let mut arguments = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (offset, character) in inner.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                arguments.push(inner[start..offset].trim());
                start = offset + 1;
            }
            _ => {}
        }
    }
    arguments.push(inner[start..].trim());
    Some((name, arguments))
}

/// Every non-test source file over [`CEILING`], as `(relative path, lines)`.
/// 每个超过 [`CEILING`] 的非测试源码文件，形式为 `(相对路径, 行数)`。
///
/// The count is the one rustfmt writes, not the one on disk. The disk count is the one that
/// drifts with formatting rather than with code, and an unformatted file is, in the case that
/// matters, the shorter of the two — so measuring disk bytes hid debt instead of bounding it.
/// The formatted text is also the artifact this repository maintains: `cargo fmt --all -- --check`
/// is the first gate, so the file rustfmt writes is the file the tree is supposed to contain.
/// See [`rewritten_line_counts`] for what happens when rustfmt cannot be asked.
/// 行数是 rustfmt 写出的那个，而不是磁盘上的那个。磁盘上的行数会随**格式**而不是随代码漂移，而未
/// 格式化的文件在真正要命的那种情形下是两者中较短的那个——因此量磁盘字节是藏住欠账而不是约束它。
/// 格式化的文本也正是本仓库维护的产物：`cargo fmt --all -- --check` 是第一道门禁，因此 rustfmt
/// 写出的文件才是这棵树本该包含的文件。问不出 rustfmt 时的行为见 [`rewritten_line_counts`]。
pub fn oversized(root: &Path) -> Vec<(String, usize)> {
    let mut measured = Vec::new();
    for directory in crate_directories(root) {
        let mut candidates = rust_sources(&directory.join("src"));
        // The crate-root `build.rs` is source a maintainer reads, and it was the one
        // file the ceiling never saw: the walk covered `src/` and nothing else.
        // crate 根的 `build.rs` 是维护者要读的源码，而它正是上限从未量到的那一个文件：遍历只覆盖
        // `src/`。
        let build = directory.join("build.rs");
        if build.is_file() {
            candidates.push(build);
        }
        for path in candidates {
            // A file is test-only when the tree *says* so: the declaration that mounts it
            // carries `#[cfg(test)]`. Location alone is not proof, and treating it as proof
            // exempted a real module that merely sat under a `tests/` directory inside
            // `src/` — the same file elsewhere was measured (audit G-05). The weak
            // location-or-name predicate still serves the documentation gate, which only has
            // to decide "is this shown to a reader".
            // 文件是否仅测试，由树自己说了算：挂载它的那条声明带着 `#[cfg(test)]`。位置本身不是
            // 证明，把它当证明会让"只是位于 `src/` 下某个 `tests/` 目录里"的真实模块免于度量——同一
            // 个文件换个位置就会被量到（审计 G-05）。那个"位置或名字"的弱判定仍服务于文档门禁，
            // 它只需判断"这是不是给读者看的东西"。
            if is_mounted_as_test(&directory, &path) {
                continue;
            }
            measured.push(path);
        }
    }
    let rewritten = workspace_edition(root).map_or_else(BTreeMap::new, |edition| {
        rewritten_line_counts(&measured, &edition)
    });
    let mut found = Vec::new();
    for path in &measured {
        let count = rewritten
            .get(path)
            .copied()
            .unwrap_or_else(|| lines(path).len());
        if count > CEILING {
            found.push((relative(root, path), count));
        }
    }
    found.sort();
    found
}

/// The edition rustfmt has to be told, read from the root manifest's `[workspace.package]`.
/// 必须告诉 rustfmt 的 edition，读自根清单的 `[workspace.package]`。
///
/// rustfmt assumes the 2015 edition when it is not told, and `cargo fmt` passes the manifest's
/// edition; asking with the wrong one measures a file the build never produces. A root that
/// declares no edition is a root rustfmt cannot be asked about, and the measurement then stays
/// on disk — which is what this gate did before the probe existed.
/// 不告诉 rustfmt 时它假定 2015 edition，而 `cargo fmt` 传的是清单的 edition；问错一个就会量到
/// 构建从不产出的文件。没有声明 edition 的根是问不出 rustfmt 的根，度量因此留在磁盘形态上——这与
/// 本探针出现之前的行为一致。
fn workspace_edition(root: &Path) -> Option<String> {
    let text = std::fs::read_to_string(root.join("Cargo.toml")).ok()?;
    let mut section = String::new();
    for line in text.lines() {
        let trimmed = line.split('#').next().unwrap_or("").trim();
        if let Some(header) = trimmed.strip_prefix('[') {
            section = header.trim_end_matches(']').trim().to_owned();
            continue;
        }
        if section != "workspace.package" {
            continue;
        }
        let Some(value) = trimmed.strip_prefix("edition") else {
            continue;
        };
        let Some(value) = value.trim_start().strip_prefix('=') else {
            continue;
        };
        return Some(value.trim_start().trim_matches(['"', '\'']).to_owned());
    }
    None
}

/// The line count rustfmt writes for each file it would rewrite.
/// rustfmt 会为它要重写的每个文件写出的行数。
///
/// The ratchet's promise is about the code a maintainer reads, and an unformatted file is
/// *shorter* than the file rustfmt writes: the B1 round measured `mutations.rs` at 586 lines
/// while rustfmt makes it 616 — under the 600-line ceiling as stored, over it as read. The
/// measurement therefore asks rustfmt. Normalizing is not one-directional in general (rustfmt
/// also joins a `fn f() {\n}` into one line), and it does not have to be: what is measured is the
/// formatted file, which is the file this repository requires to be on disk anyway.
/// 棘轮的承诺针对维护者要读的代码，而未格式化的文件比 rustfmt 写出的文件**更短**：B1 轮实测
/// `mutations.rs` 是 586 行，而 rustfmt 会写成 616 行——按存储形态在上限之内，按阅读形态在上限
/// 之上。因此度量去问 rustfmt。归一化在一般情况下并不是单向的（rustfmt 也会把 `fn f() {\n}` 并成
/// 一行），而这不必是问题：量的是**格式化的文件**，而那正是本仓库要求落在磁盘上的那个文件。
///
/// Every path that cannot ask rustfmt (no `rustfmt` on the machine, no edition in the root
/// manifest, a file rustfmt refuses) falls back to the on-disk count, which can only report *less*
/// debt than the truth — the direction that was already there, so a machine without rustfmt loses
/// the probe rather than the tree, and `cargo fmt --all -- --check` remains the gate that refuses
/// an unformatted tree.
/// 每一条问不成 rustfmt 的路径（机器上没有 `rustfmt`、根清单没有 edition、rustfmt 拒绝某个文件）
/// 都退回磁盘行数，那只会比事实报得**更少**——与原先一样的方向，因此没有 rustfmt 的机器失去的是
/// 这次探针，而不是整棵树；而拒绝未格式化树的仍然是 `cargo fmt --all -- --check` 那道门禁。
fn rewritten_line_counts(files: &[PathBuf], edition: &str) -> BTreeMap<PathBuf, usize> {
    let mut counts = BTreeMap::new();
    let Some(rewritten) = rustfmt_rewrites(files, edition) else {
        return counts;
    };
    if rewritten.is_empty() {
        return counts;
    }
    for path in rewritten {
        if let Some(count) = formatted_lines(&path, edition) {
            counts.insert(path, count);
        }
    }
    counts
}

/// The files rustfmt says it would rewrite, or `None` when rustfmt cannot be asked.
/// rustfmt 说它会重写的文件；问不出 rustfmt 时为 `None`。
///
/// One run decides the whole tree: a formatted tree costs one rustfmt and no rewriting, which
/// is the only case the shipped tree is in.
/// 一次运行判定整棵树：已格式化的树只花一次 rustfmt、不做任何重写，而这正是出厂树所处的情形。
fn rustfmt_rewrites(files: &[PathBuf], edition: &str) -> Option<Vec<PathBuf>> {
    let output = Command::new("rustfmt")
        .args(["--files-with-diff", "--edition", edition])
        .args(files)
        .output()
        .ok()?;
    let listed = String::from_utf8_lossy(&output.stdout);
    let listed: BTreeSet<&str> = listed
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    // A run that failed without naming a file is a rustfmt this gate cannot use, not a
    // formatted tree — and saying "formatted" there would silently return the old measurement.
    // 一次失败且没有点名任何文件的运行是本门禁用不了的 rustfmt，而不是一棵已格式化的树——在那里
    // 说"已格式化"会静默退回旧的度量。
    if !output.status.success() && listed.is_empty() {
        return None;
    }
    Some(
        files
            .iter()
            .filter(|path| listed.contains(path.to_string_lossy().as_ref()))
            .cloned()
            .collect(),
    )
}

/// The number of lines rustfmt writes for `path`, when it can be asked.
/// rustfmt 为 `path` 写出的行数（问得出来时）。
///
/// The text goes in over stdin rather than through a copy beside the original: rustfmt resolves
/// a `mod x;` against the file's own directory, so a copy in a scratch directory fails to
/// resolve one and reports an error instead of a rewrite — which for this tree, whose gate
/// files all mount their test halves, would have meant "cannot be measured". On stdin it has no
/// directory to resolve against, and it still reads its configuration from the working directory
/// upwards.
/// 文本经 stdin 送入，而不是在原文件旁边放一份副本：rustfmt 会相对文件自身所在目录解析 `mod x;`，
/// 因此放在临时目录里的副本解析不到，报的是错误而不是重写——对本树而言（每个门禁文件都挂载着自己的
/// 测试半边）那等于"无法度量"。走 stdin 时它没有目录可解析，而它仍会从工作目录向上读配置。
fn formatted_lines(path: &Path, edition: &str) -> Option<usize> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut child = Command::new("rustfmt")
        .args(["--emit", "stdout", "--edition", edition])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut stdin = child.stdin.take()?;
    let _ = stdin.write_all(text.as_bytes());
    drop(stdin);
    let output = child.wait_with_output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8(output.stdout).ok())
        .flatten()
        .map(|formatted| formatted.lines().count())
}

#[cfg(test)]
#[path = "size_tests.rs"]
mod size_tests;
