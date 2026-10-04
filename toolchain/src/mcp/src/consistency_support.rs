//! The shape comparison's vocabulary and the excerpt reader behind it.
//! 形状比较的词汇表，以及它背后的摘录读取器。
//!
//! Split out of `consistency.rs` because that file is under a 600-code-line ratchet and the ledger
//! comparison is what pushed it over. What lives here is the **decidable half**: which fields a
//! specimen is compared on, how one spelling of a trait field is matched under one key, and how the
//! lines a deviation names are quoted back (audit `F1`, `W4-6`). The comparisons themselves — the
//! sibling set, the ledger verdict, the two signals — stay in the parent, because they are what the
//! tool answers with.
//! 从 `consistency.rs` 拆出来，因为那个文件受 600 代码行棘轮约束，而台账比对正是把它顶过去的东西。这里
//! 住的是**可判定的那一半**：标本按哪些字段比、trait 字段的两种拼写怎么归到一个键下、以及一处偏离点名的
//! 那些行怎么被引回来（审计 `F1`、`W4-6`）。比较本身——同族集合、台账裁定、两种信号——留在父模块，因为
//! 那才是这个工具作答的东西。

use std::path::Path;

use nichlink_kernel::syntax::FaceSyntax;

use crate::build_time::FaceView;
use crate::mcp::source_index::load_sources;

/// How one shape field is compared between a specimen and a sibling.
/// 某个形状字段在标本与兄弟之间怎么比。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ShapeComparison {
    /// Only whether the sibling declares the field at all: its value is this object's own name.
    /// 只比这个兄弟到底有没有声明该字段：它的取值就是这个对象自己的名字。
    Presence,
    /// The labels the specimen states must appear in the sibling's list. Extra labels are that
    /// sibling's own business, like everything else here — the comparison is one-directional.
    /// 标本写下的标签必须出现在兄弟的清单里。多出来的标签是那个兄弟自己的事，与这里其余部分一样——比较
    /// 是单向的。
    Labels,
}

/// One declared-shape field: the **key** a comparison matches on, the spelling the declaration
/// actually wrote, and the value.
/// 一个已声明形状字段：比较据以匹配的**键**、声明**实际写下**的拼写、以及取值。
///
/// The key and the spelling are deliberately two things. The kernel accepts two spellings for a
/// trait field — `handle_contracts` (compiler-checked Rust paths) and `handle_traits` (the plain
/// labels) — and both are editable through `apply`. Reporting the *derived* spelling instead of the
/// one in the file is what audit `F1` caught: a reader was handed a name whose value could not be
/// pasted back where the declaration keeps it. So the comparison still matches on one key (a family
/// is not an outlier for choosing the other spelling), while every line a reader may copy names the
/// spelling that is actually written.
/// 键与拼写是刻意分开的两件事。内核接受两种 trait 字段拼写——`handle_contracts`（参与编译检查的 Rust
/// 路径）与 `handle_traits`（纯标签）——而两者都能经 `apply` 编辑。报**推导**拼写而不是文件里那一个，
/// 正是审计 `F1` 抓到的：读者拿到的名字，其取值放不回声明保存它的位置。因此比较仍只认一个键（同族不会
/// 因为选了另一种拼写而被判离群），而每一行**读者可能照抄**的输出都点名文件里真正写着的那个拼写。
#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) struct DeclaredField {
    /// What comparisons match on; two spellings of one field share it.
    /// 比较据以匹配的东西；同一个字段的两种拼写共用它。
    pub(super) key: &'static str,
    /// The spelling the declaration wrote — the one a reader may paste back into `apply`.
    /// 声明写下的拼写——读者可以照抄回 `apply` 的那一个。
    pub(super) spelling: &'static str,
    /// The rendered value.
    /// 渲染后的取值。
    pub(super) value: String,
}

/// One entry of [`SHAPE_FIELDS`]: what to compare, under which key, and how.
/// [`SHAPE_FIELDS`] 的一项：比什么、用哪个键、怎么比。
pub(super) struct ShapeField {
    /// The key comparisons match on.
    /// 比较据以匹配的键。
    pub(super) key: &'static str,
    /// The spellings a declaration may write, the compiler-checked one first when there are two.
    /// 声明可以写下的拼写；有两个时把参与编译检查的那个放前面。
    pub(super) spellings: &'static [&'static str],
    /// How the value is compared.
    /// 取值怎么比。
    pub(super) comparison: ShapeComparison,
}

/// The declared-shape fields a specimen is compared on, and how each one is compared.
/// 对比一个标本时看哪些已声明形状字段，以及每一项怎么比。
///
/// Every name here is a field name the kernel's own vocabulary carries (`FACE_FIELD_ORDER`), and the
/// values are read through the kernel's parser (`FaceSyntax`), so this list can name fields but
/// cannot invent or re-spell them. The two trait entries resolve the `*_contracts` spelling the same
/// way the build-time contract check does — a face that states a checked path does not state its
/// label a second time.
/// 这里的每个名字都是内核词表（`FACE_FIELD_ORDER`）携带的字段名，取值经内核解析器（`FaceSyntax`）读出，
/// 因此这张清单可以点名字段，却不能发明或改写它们。两条 trait 条目按构建期合同检查同一条规则解析
/// `*_contracts` 写法——写了受检路径的注册面不必再写一遍标签。
///
/// `parts` is presence-only because it names the object's **own** parts type (`ButtonParts`): two
/// siblings stating different ones is the design, not a drift, and comparing the value would make
/// every family an outlier.
/// `parts` 只比存在性，因为它点名的是对象**自己**的零件类型（`ButtonParts`）：两个兄弟写出不同的类型正是
/// 设计而非漂移，比值会让每一个同族都成了离群。
pub(super) const SHAPE_FIELDS: &[ShapeField] = &[
    ShapeField {
        key: "parts",
        spellings: &["parts"],
        comparison: ShapeComparison::Presence,
    },
    ShapeField {
        key: "exports",
        spellings: &["exports"],
        comparison: ShapeComparison::Labels,
    },
    ShapeField {
        key: "handle_traits",
        spellings: &["handle_contracts", "handle_traits"],
        comparison: ShapeComparison::Labels,
    },
    ShapeField {
        key: "part_traits",
        spellings: &["part_contracts", "part_traits"],
        comparison: ShapeComparison::Labels,
    },
];

/// The shape field spellings this comparison reads, for the lines that name the set.
/// 这次比较读哪些形状字段**拼写**，供点名该集合的行使用。
///
/// Every spelling is named, not only the canonical key: this line is read by someone who is about
/// to write one of them, and naming a spelling the file does not carry is the `F1` defect.
/// 每一种拼写都点名，而不只是规范键：读这一行的人正要去写其中之一，而点一个文件里没有的拼写就是
/// `F1` 那个缺陷。
pub(super) fn shape_field_names() -> Vec<&'static str> {
    SHAPE_FIELDS
        .iter()
        .flat_map(|field| field.spellings.iter().copied())
        .collect()
}

/// One field's rendered value as the labels it states.
/// 某个字段渲染后的取值，按它写下的标签拆开。
pub(super) fn shape_labels(value: &str) -> Vec<&str> {
    value
        .split(", ")
        .map(str::trim)
        .filter(|label| !label.is_empty())
        .collect()
}

/// The labels a trait field carries, from either spelling, or `None` when neither is written.
/// 某个 trait 字段携带的标签（两种写法都认），两者都没写时为 `None`。
///
/// The value is always labels — the kernel derives them from the paths — while the returned
/// **spelling** is whichever one the file actually carries, so a reader can paste it back.
/// 取值永远是标签（由内核从路径派生），而返回的**拼写**是文件里真正写着的那个，读者据此可以照抄回去。
pub(super) fn trait_shape(
    face: &FaceSyntax,
    contracts: &'static str,
    labels: &'static str,
) -> Option<(&'static str, String)> {
    let wrote_contracts = face.field(contracts).is_some();
    let wrote_labels = face.field(labels).is_some();
    if !wrote_contracts && !wrote_labels {
        return None;
    }
    let value = face
        .string_list(labels)
        .map(|values| values.join(", "))
        .or_else(|| face.field(labels))
        .or_else(|| face.field(contracts))?;
    // The checked spelling wins when a face wrote both: that is the one the compiler reads, and the
    // labels beside it are the redundant half.
    // 两者都写时以受检的那个拼写为准：编译器读的是它，旁边那份标签才是多余的一半。
    let spelling = if wrote_contracts { contracts } else { labels };
    Some((spelling, value))
}

/// A `read` call for the first quoted line of one excerpt block.
/// 一个摘录块里第一行引用所对应的 `read` 调用。
///
/// The excerpt is already a coordinate (`src/x.rs:12  text`), so the call that opens it is a
/// substring away — and a command a reader has to assemble is the thing `W4-5` measured as an extra
/// step.
/// 摘录本身就是坐标（`src/x.rs:12  text`），因此打开它的调用只差一次截取——而"要读者自己拼的命令"
/// 正是 `W4-5` 量到的那多出来的一步。
pub(super) fn read_command(shown: &[String]) -> Option<String> {
    let (path, line) = shown.first()?.trim().split_once("  ")?.0.split_once(':')?;
    Some(format!("--call read --path {path} --line {line}"))
}

/// The tree census every answer about a tree can open with.
/// 每个关于一棵树的答案都可以用它开头的那行普查。
///
/// Audit `W4-5`/`W1-2`: the round measured sessions opening with `status` just to learn how big the
/// tree is, then asking the real question. The size is a fact every one of these tools already
/// holds, so it rides on the answer instead of costing a call.
/// 审计 `W4-5`/`W1-2`：那一轮量到会话先发一次 `status`、只为知道树有多大，然后再问真正的问题。这个大小是
/// 这些工具**本来就持有**的事实，因此它随答案一起走，而不是花掉一次调用。
pub(crate) fn tree_census(root: &Path) -> Option<String> {
    let files = load_sources(root).ok()?.len();
    let namespace = crate::mcp::registry::namespace(root).ok()?;
    let (faces, _) = crate::mcp::resolve::derived_faces(root, &namespace).ok()?;
    Some(format!("tree: {} face(s), {files} file(s)", faces.len()))
}

/// One parsed face's declared shape: the shape fields it carries, in the order of [`SHAPE_FIELDS`].
/// 一个已解析注册面的已声明形状：它携带的形状字段，按 [`SHAPE_FIELDS`] 的顺序。
pub(super) fn declared_shape(face: &FaceSyntax) -> Vec<DeclaredField> {
    let mut shape = Vec::new();
    for field in SHAPE_FIELDS {
        let declared = match field.spellings {
            [contracts, labels] => {
                trait_shape(face, contracts, labels).map(|(spelling, value)| DeclaredField {
                    key: field.key,
                    spelling,
                    value,
                })
            }
            [name, ..] => face
                .string_list(name)
                .map(|values| values.join(", "))
                .or_else(|| face.field(name))
                .map(|value| DeclaredField {
                    key: field.key,
                    spelling: name,
                    value,
                }),
            [] => None,
        };
        if let Some(declared) = declared {
            shape.push(declared);
        }
    }
    shape
}

/// One sibling's shape against the specimen's: what it lacks, and which stated labels it is missing.
/// 某个兄弟的形状对标本而言缺什么、以及缺了哪些已写下的标签。
///
/// One direction only, and that is the point: the specimen is the baseline, so the question is
/// whether it was followed, not whether a sibling invented something more. A sibling carrying a
/// field, or a label, that the specimen does not is therefore not a row here, and the boundary
/// sentence says so.
/// 只判一个方向，而这就是要点：标本是基准，因此问题是"有没有照它做"，不是"兄弟有没有多发明什么"。因此一个
/// 携带了标本没有的字段或标签的兄弟不在这里成行，而边界句会说清这一点。
///
/// Matching is on the **key**, naming is by the specimen's **spelling**: a sibling that chose the
/// other spelling of a trait field is not lacking anything, while the row for one that lacks it
/// tells the reader the name the baseline actually uses.
/// 匹配按**键**、命名按标本的**拼写**：选了 trait 字段另一种拼写的兄弟并不缺什么，而真的缺的那一个，
/// 行里给出的名字是基准真正在用的那个。
pub(super) fn shape_gaps(specimen: &[DeclaredField], sibling: &[DeclaredField]) -> Vec<String> {
    let mut notes = Vec::new();
    for field in specimen {
        let Some(other) = sibling.iter().find(|candidate| candidate.key == field.key) else {
            notes.push(format!("lacks `{}`", field.spelling));
            continue;
        };
        let comparison = SHAPE_FIELDS
            .iter()
            .find(|candidate| candidate.key == field.key)
            .map(|candidate| candidate.comparison);
        if comparison != Some(ShapeComparison::Labels) {
            continue;
        }
        let stated = shape_labels(&other.value);
        for label in shape_labels(&field.value) {
            if !stated.contains(&label) {
                notes.push(format!("lacks `{}: {label}`", field.spelling));
            }
        }
    }
    notes
}

/// How many source lines one outlier may carry.
/// 一个离群者最多可以带几行源码。
///
/// The round that measured the alternative (codegraph answered the same question with a 6,781-character
/// slab) is why this is a count and not "the file": what decides the answer is the one line that
/// deviates, and ten lines is enough to show it with its neighbours.
/// 量到另一种做法的那一轮（codegraph 对同一个问题给了 6,781 字符的整片）正是这里写"行数"而不是写"整个
/// 文件"的原因：决定答案的是偏离的那一行，而十行足够把它和邻居一起显示出来。
pub(super) const EXCERPT_LIMIT: usize = 10;

/// The field names a specimen's gap notes name.
/// 标本的差异注里点名的字段名。
///
/// `lacks \`handle_contracts: ControlHandle\`` and `lacks \`parts\`` both carry the field before the
/// colon; the label after it is a value, not a name to search the source for.
/// `lacks \`handle_contracts: ControlHandle\`` 与 `lacks \`parts\`` 都在冒号前带字段名；冒号后那个是取值，
/// 不是要在源码里找的名字。
pub(super) fn gap_fields(gaps: &[String]) -> Vec<String> {
    gaps.iter()
        .filter_map(|gap| backticked(gap))
        .map(|named| {
            named
                .split_once(':')
                .map_or(named, |(field, _)| field)
                .trim()
                .to_owned()
        })
        .collect()
}

/// The first backticked word in a note, when it has one.
/// 一条注里第一个反引号包起来的词（有时）。
pub(super) fn backticked(note: &str) -> Option<&str> {
    let start = note.find('`')? + 1;
    let rest = &note[start..];
    let end = rest.find('`')?;
    Some(rest[..end].trim())
}

/// The label one face is reported under: its own path's last segment.
/// 一个面被报告时用的标签：它自己路径的最后一段。
pub(super) fn label_of(face: &FaceView) -> String {
    face.path
        .rsplit('/')
        .next()
        .unwrap_or(&face.path)
        .to_owned()
}

/// Up to `limit` lines of one file's text that mention any needle, as `path:line  text`.
/// 一份文件文本里提到任一 needle 的最多 `limit` 行，形如 `path:line  text`。
///
/// A line is trimmed and cut at [`LINE_LIMIT`] characters: the point is the decisive line, and one
/// generated declaration can be a thousand characters wide — quoting it whole would put back the
/// slab this excerpt exists to replace.
/// 每行去掉首尾空白、并在 [`LINE_LIMIT`] 字符处截断：要点是那一行决定性的内容，而一条生成出来的声
/// 明可以宽达上千字符——整条引用会把这段摘录本要替代掉的整片又装回来。
pub(super) fn lines_naming(
    text: &str,
    path: &str,
    needles: &[String],
    limit: usize,
) -> Vec<String> {
    if limit == 0 || needles.is_empty() {
        return Vec::new();
    }
    let mut rows = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if rows.len() >= limit {
            break;
        }
        if !needles.iter().any(|needle| line.contains(needle.as_str())) {
            continue;
        }
        let trimmed = line.trim();
        let shown = if trimmed.chars().count() > LINE_LIMIT {
            trimmed.chars().take(LINE_LIMIT).collect::<String>() + "…"
        } else {
            trimmed.to_owned()
        };
        rows.push(format!("    {path}:{}  {shown}", index + 1));
    }
    rows
}

/// How wide one quoted source line may be, in characters.
/// 引用的一行源码最多多宽（字符）。
pub(super) const LINE_LIMIT: usize = 200;

/// The registration declaration one file carries, as quoted lines, up to `limit`.
/// 一份文件携带的那条注册声明，按行引用，最多 `limit` 行。
///
/// It exists for the deviation that has **no line of its own**: a field the face does not declare
/// cannot be quoted, so what a reader gets instead is the declaration it does have. The block runs
/// from the macro invocation to its closing brace, which is the whole declaration by this grammar's
/// own shape.
/// 它是为**自己没有行**的那种偏离而存在的：一个面没声明的字段引不出来，于是读者拿到的是它**确实**有的那条
/// 声明。块从宏调用起到它的收尾大括号，按这套语法自己的形状，那就是整条声明。
pub(super) fn declaration_block(text: &str, path: &str, limit: usize) -> Vec<String> {
    if limit == 0 {
        return Vec::new();
    }
    let mut rows = Vec::new();
    let mut inside = false;
    for (index, line) in text.lines().enumerate() {
        if !inside {
            if !line.contains('!') || !line.contains('{') {
                continue;
            }
            inside = true;
        }
        let trimmed = line.trim();
        let shown = if trimmed.chars().count() > LINE_LIMIT {
            trimmed.chars().take(LINE_LIMIT).collect::<String>() + "…"
        } else {
            trimmed.to_owned()
        };
        rows.push(format!("    {path}:{}  {shown}", index + 1));
        if rows.len() >= limit {
            break;
        }
        if trimmed.starts_with('}') {
            break;
        }
    }
    rows
}

/// One rendered shape, or the line that says there was none to read.
/// 渲染后的形状；无可读字段时给出那一行说明。
pub(super) fn shape_line(shape: &[DeclaredField]) -> String {
    if shape.is_empty() {
        return format!(
            "none of the fields this comparison reads ({})",
            shape_field_names().join(", ")
        );
    }
    shape
        .iter()
        .map(|field| format!("{} `{}`", field.spelling, field.value))
        .collect::<Vec<_>>()
        .join(" · ")
}
