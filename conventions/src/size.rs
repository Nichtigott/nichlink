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

use std::path::Path;

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
pub const BASELINE: &[(&str, usize)] =
    &[("core/src/registry_core/plugin/contracts/contracts.rs", 639)];

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
/// 描述**：2026-09-28 实测本树出厂的最深挂载链是 6 层（`authoring.rs` → `parse/parse.rs` →
/// `parse/flow.rs` → `parse/flow_tests.rs`），而当时取的是 8——只剩两层的余量，再嵌两层就会让
/// 某个测试文件不再豁免（门禁会报出那个文件：响亮，但不是我们想要的失败）。因此取 16，并由
/// `a_deep_mount_chain_is_still_test_only` 用一条十层链钉住这份余量。**`doc_blocks` 也用它**：
/// 同一个"这个文件是不是仅测试"的身份问题，两道门禁必须给同一条规则（`tree/graft_ops/fixtures.rs`
/// 就是名字不像测试、却挂在 `#[cfg(test)]` 之后的真实例子）。
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
    let bare = format!("mod {stem};");
    let bare_public = format!("pub mod {stem};");
    for candidate in rust_sources(crate_root) {
        let Ok(text) = std::fs::read_to_string(&candidate) else {
            continue;
        };
        let source: Vec<&str> = text.lines().collect();
        for (index, line) in source.iter().enumerate() {
            let trimmed = line.trim();
            let declares = (trimmed.starts_with("#[path") && line.contains(&attribute))
                || trimmed == bare
                || trimmed == bare_public;
            if !declares {
                continue;
            }
            let mut cursor = index;
            while cursor > 0 {
                cursor -= 1;
                let previous = source[cursor].trim();
                if previous.starts_with("#[cfg(test)") {
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

/// Every non-test source file over [`CEILING`], as `(relative path, lines)`.
/// 每个超过 [`CEILING`] 的非测试源码文件，形式为 `(相对路径, 行数)`。
pub fn oversized(root: &Path) -> Vec<(String, usize)> {
    let mut found = Vec::new();
    for directory in crate_directories(root) {
        let mut measured = rust_sources(&directory.join("src"));
        // The crate-root `build.rs` is source a maintainer reads, and it was the one
        // file the ceiling never saw: the walk covered `src/` and nothing else.
        // crate 根的 `build.rs` 是维护者要读的源码，而它正是上限从未量到的那一个文件：遍历只覆盖
        // `src/`。
        let build = directory.join("build.rs");
        if build.is_file() {
            measured.push(build);
        }
        for path in measured {
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
            let count = lines(&path).len();
            if count > CEILING {
                found.push((relative(root, &path), count));
            }
        }
    }
    found.sort();
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace_root;
    use std::fs;
    use std::path::PathBuf;

    /// The measured set of oversized files equals the pinned debt exactly.
    /// 实测的超标文件集合与钉住的欠账完全一致。
    #[test]
    fn the_size_ceiling_holds_except_for_the_pinned_debt() {
        let root = workspace_root();
        let actual = oversized(&root);
        let mut expected: Vec<(String, usize)> = BASELINE
            .iter()
            .map(|(path, count)| ((*path).to_owned(), *count))
            .collect();
        expected.sort();
        let new: Vec<_> = actual
            .iter()
            .filter(|entry| !expected.contains(entry))
            .collect();
        let stale: Vec<_> = expected
            .iter()
            .filter(|entry| !actual.contains(entry))
            .collect();
        assert!(
            new.is_empty(),
            "new files over the {CEILING}-line ceiling; split them: {new:#?}"
        );
        assert!(
            stale.is_empty(),
            "these entries are stale or their file changed size; update BASELINE \
             (an entry that shrank back under the ceiling must be removed): {stale:#?}"
        );
    }

    /// A throwaway workspace with one member, `probe`, holding the given files.
    /// 一个只含一个成员 `probe` 的一次性工作区，成员下含给定文件。
    ///
    /// The manifest deliberately carries a `[workspace.package] name` right after the
    /// members array: a reader that took that section for members would build a crate
    /// directory out of a borrowed name.
    /// 清单刻意在 members 数组之后紧跟一个 `[workspace.package] name`：把那一节当成成员的读者
    /// 会用一个借来的名字造出一个 crate 目录。
    fn synthetic(files: &[(&str, &str)]) -> PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("nichlink-size-{}-{sequence}", std::process::id()));
        fs::create_dir_all(root.join("probe/src")).expect("fixture src");
        fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"probe\"]\nresolver = \"2\"\n\n\
             [workspace.package]\nname = \"borrowed\"\n",
        )
        .expect("fixture manifest");
        for (relative, contents) in files {
            let path = root.join("probe").join(relative);
            fs::create_dir_all(path.parent().expect("parent")).expect("fixture dir");
            fs::write(&path, contents).expect("fixture file");
        }
        root
    }

    /// Filler that pushes a fixture file past [`CEILING`].
    /// 把夹具文件推过 [`CEILING`] 的填充内容。
    fn over_ceiling() -> String {
        "// filler\n".repeat(CEILING + 10)
    }

    /// Location is not proof of testhood: a real module that merely sits under a `tests/`
    /// directory inside `src/` was exempt from the ceiling while the same file elsewhere was
    /// measured. The declaration that mounts it has to carry `#[cfg(test)]` — or inherit one
    /// from an ancestor, which is how this tree's nested test modules are written.
    /// 位置本身不是"仅测试"的证明：只是位于 `src/` 下某个 `tests/` 目录里的真实模块过去免于度量，
    /// 而同一个文件换个位置就会被量。挂载它的声明必须带 `#[cfg(test)]`——或者从祖先继承一个，本树
    /// 的嵌套测试模块正是这么写的。
    #[test]
    fn a_module_under_src_tests_needs_a_test_declaration_to_be_exempt() {
        let root = synthetic(&[
            ("src/lib.rs", "pub mod tests;\n"),
            (
                "src/tests.rs",
                "#[path = \"tests/zz_big.rs\"]\npub mod zz_big;\n",
            ),
            ("src/tests/zz_big.rs", &over_ceiling()),
        ]);
        assert!(
            oversized(&root)
                .iter()
                .any(|(path, _)| path.ends_with("tests/zz_big.rs")),
            "nothing says this file is a test, so the ceiling measures it"
        );
        let _ = fs::remove_dir_all(&root);

        let root = synthetic(&[
            ("src/lib.rs", "pub mod tests;\n"),
            (
                "src/tests.rs",
                "#[cfg(test)]\n#[path = \"tests/zz_big.rs\"]\npub mod zz_big;\n",
            ),
            ("src/tests/zz_big.rs", &over_ceiling()),
        ]);
        assert!(
            !oversized(&root)
                .iter()
                .any(|(path, _)| path.ends_with("tests/zz_big.rs")),
            "a `#[cfg(test)]` declaration makes it a test file"
        );
        let _ = fs::remove_dir_all(&root);

        let root = synthetic(&[
            ("src/lib.rs", "#[cfg(test)]\npub mod tests;\n"),
            (
                "src/tests.rs",
                "#[path = \"tests/zz_big.rs\"]\npub mod zz_big;\n",
            ),
            ("src/tests/zz_big.rs", &over_ceiling()),
        ]);
        assert!(
            !oversized(&root)
                .iter()
                .any(|(path, _)| path.ends_with("tests/zz_big.rs")),
            "the attribute is inherited down the mount chain"
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// The recursion bound is a cycle stop with headroom, not a description of this tree's depth.
    /// The deepest chain the tree ships is six levels (measured: `authoring.rs` → `parse/parse.rs`
    /// → `parse/flow.rs` → `parse/flow_tests.rs`); a ten-level chain below one `#[cfg(test)]` mount
    /// must still be exempt, because a bound that merely fits today's tree reclassifies a test file
    /// as measured source the next time someone nests two more modules.
    /// 递归上限是带余量的防环，而不是对本树深度的描述。本树出厂的最深链是 6 层（实测：
    /// `authoring.rs` → `parse/parse.rs` → `parse/flow.rs` → `parse/flow_tests.rs`）；一条挂在
    /// 单个 `#[cfg(test)]` 之下的十层链必须仍然豁免，因为"刚好装下今天的树"的上限会在下次有人再嵌
    /// 两层模块时，把一个测试文件重新归类成被测源码。
    #[test]
    fn a_deep_mount_chain_is_still_test_only() {
        let mut files: Vec<(String, String)> = vec![(
            "src/lib.rs".to_owned(),
            "#[cfg(test)]\npub mod a;\n".to_owned(),
        )];
        for letter in b'a'..=b'j' {
            let name = (letter as char).to_string();
            let body = if letter == b'j' {
                over_ceiling()
            } else {
                let next = (letter + 1) as char;
                format!("#[path = \"{next}.rs\"]\npub mod {next};\n")
            };
            files.push((format!("src/{name}.rs"), body));
        }
        let borrowed: Vec<(&str, &str)> = files
            .iter()
            .map(|(path, body)| (path.as_str(), body.as_str()))
            .collect();
        let root = synthetic(&borrowed);
        assert!(
            !oversized(&root)
                .iter()
                .any(|(path, _)| path.ends_with("j.rs")),
            "a ten-level chain below a `#[cfg(test)]` mount is still a test file"
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// Test-only files are recognised by where they sit, or by their name together
    /// with a `#[cfg(test)]` mount.
    /// 仅测试文件按所在位置识别，或按名字加 `#[cfg(test)]` 挂载识别。
    #[test]
    fn test_only_files_are_recognised() {
        assert!(is_test_shaped(Path::new("cli/src/lib_tests.rs")));
        assert!(is_test_shaped(Path::new(
            "core/src/registry_core/syntax/face_tests.rs"
        )));
        assert!(is_test_shaped(Path::new("studio/src/studio/app/tests.rs")));
        assert!(!is_test_shaped(Path::new(
            "core/src/registry_core/syntax/face.rs"
        )));
        assert!(is_test_by_location(Path::new("studio/tests/graph.rs")));
        assert!(!is_test_by_location(Path::new(
            "studio/src/studio/app/tests.rs"
        )));
    }

    /// A name is not proof: real code renamed to `x_tests.rs` and never mounted is
    /// measured like anything else, which is the hole this closed.
    /// 名字不是证明：把真实代码改名成 `x_tests.rs` 却从不挂载，会像别的文件一样被度量——这正是
    /// 被堵上的那个洞。
    #[test]
    fn an_unmounted_test_shaped_file_is_measured() {
        let root = synthetic(&[("src/probe_tests.rs", &over_ceiling())]);
        let found = oversized(&root);
        assert_eq!(
            found.len(),
            1,
            "an unmounted `_tests` file is source like any other: {found:#?}"
        );
        assert!(found[0].0.ends_with("src/probe_tests.rs"), "{found:#?}");
        let _ = fs::remove_dir_all(&root);
    }

    /// Mounted behind `#[cfg(test)]`, the same file is exempt — under both spellings
    /// this tree uses.
    /// 挂在 `#[cfg(test)]` 之后，同一个文件重新豁免——本树使用的两种拼法都算。
    #[test]
    fn a_mounted_test_file_is_exempt() {
        let root = synthetic(&[
            (
                "src/probe.rs",
                "#[cfg(test)]\n#[path = \"probe_tests.rs\"]\nmod probe_tests;\n",
            ),
            ("src/probe_tests.rs", &over_ceiling()),
        ]);
        assert_eq!(
            oversized(&root),
            Vec::new(),
            "a mounted `#[path]` test module is exempt"
        );
        let _ = fs::remove_dir_all(&root);

        let root = synthetic(&[
            ("src/tests.rs", &over_ceiling()),
            ("src/lib.rs", "#[cfg(test)]\nmod tests;\n"),
        ]);
        assert_eq!(
            oversized(&root),
            Vec::new(),
            "the bare `mod tests;` form is exempt too"
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// The crate-root `build.rs` is measured like `src/`.
    /// crate 根的 `build.rs` 与 `src/` 一同被度量。
    #[test]
    fn the_crate_root_build_script_is_measured() {
        let root = synthetic(&[("build.rs", &over_ceiling())]);
        let found = oversized(&root);
        assert_eq!(
            found.len(),
            1,
            "an oversized build script is debt like any other: {found:#?}"
        );
        assert!(found[0].0.ends_with("build.rs"), "{found:#?}");
        let _ = fs::remove_dir_all(&root);
    }

    /// The ceiling is the maintainer's decision, not a drift target: a file at 500
    /// lines is accepted and one past 600 is still debt. Pinned so the 2026-09-28
    /// relaxation from 450 to 600 cannot be undone or exceeded silently.
    /// 上限是维护者的决定，不是可以漂移的目标：500 行的文件被接受，超过 600 的仍是欠账。钉住它，
    /// 使 2026-09-28 从 450 到 600 的放宽不会被静默撤销或被静默越界。
    #[test]
    fn the_ceiling_is_the_number_the_maintainer_set() {
        assert_eq!(
            CEILING, 600,
            "the ratchet's ceiling is the maintainer's decision of 2026-09-28"
        );
        let accepted = "// filler\n".repeat(500);
        let root = synthetic(&[("src/probe.rs", &accepted)]);
        assert_eq!(
            oversized(&root),
            Vec::new(),
            "500 lines sits under a 600-line ceiling"
        );
        let _ = fs::remove_dir_all(&root);

        let root = synthetic(&[("src/probe.rs", &over_ceiling())]);
        assert_eq!(
            oversized(&root).len(),
            1,
            "past the ceiling is still measured debt"
        );
        let _ = fs::remove_dir_all(&root);
    }
}
