//! Admission field rendering and parsing for authored faces.
//! 注册面 admission 字段的渲染与解析。
//!
//! Admission is written either as the compact clause form or as the
//! `Admission::new(&[…], &[…])` constructor an editor emits. Both spellings
//! describe the same gate, so both are read here.
//!
//! The compact form is the `registration_rule` family's clause grammar, one
//! clause per list: `allow:ui,controls`, `deny:ui/experimental`, `ANY` for the
//! unconstrained gate, and both clauses (`allow:ui;deny:ui/experimental`) when a
//! declaration names both lists. A declaration that names both lists is read,
//! rendered, and written back **without losing either one**: the deny list is
//! the veto `Admission::accepts` gives priority to, so a read that kept only the
//! allow list widened the gate, and the widened form is what got written back to
//! the source. `;` is the clause separator now, so a path containing `;` is
//! refused instead of being read as one long path.
//! admission 要么写成紧凑的子句形式，要么写成编辑器发出的 `Admission::new(&[…], &[…])`
//! 构造形式。两种写法描述同一道门禁，因此都在此读取。
//!
//! 紧凑形式沿用 `registration_rule` 家族的“子句”语法，每张列表一个子句：`allow:ui,controls`、
//! `deny:ui/experimental`、表示不受限门禁的 `ANY`，以及声明同时给出两张列表时的
//! `allow:ui;deny:ui/experimental`。同时给出两张列表的声明在读取、渲染与写回的全过程中
//! **两张都不会丢**：deny 列表是 `Admission::accepts` 优先给出的否决权，只保留 allow 列表的
//! 读取会放宽门禁，而写回源码的正是被放宽的那一份。`;` 现在是子句分隔符，因此含 `;` 的
//! 路径会被拒绝，而不是被读成一条很长的路径。
//!
//! The compact spelling is rendered by [`compact_admission`], which is **public
//! API**: a surface that shows or saves the compact form calls it instead of
//! spelling the grammar a second time. Studio had to spell it itself while this was
//! private, and its copy widened the gate the kernel had just fixed (`FIXR-01`).
//! 紧凑拼法由 [`compact_admission`] 渲染，而它是**公开 API**：展示或保存紧凑形式的执行面调用它，
//! 而不是把语法再拼一遍。本函数私有时 Studio 只能自己拼，而它那份副本放宽了内核刚修好的门禁
//! （`FIXR-01`）。

use crate::registry_core::authoring::validation::rust_string;
use crate::registry_core::declaration::OwnedAdmission;

use super::{FaceParseError, quoted_list_field, split_csv_owned};

/// Render one admission policy as the compact clause the editor carries.
/// 将一条 admission 策略渲染成编辑器所携带的紧凑子句。
///
/// This is the kernel's only renderer for the compact spelling, and it is public
/// because the alternative was a second implementation outside the kernel: Studio
/// needed the compact form for its Edit-form prefill and, with this function
/// private, spelled the same grammar itself — including the same deny-list
/// widening the kernel had just fixed (`FIXR-01`). A surface that shows or saves
/// the compact form calls this function; it must not decide the spelling itself.
/// 这是内核唯一的紧凑拼法渲染器，之所以公开，是因为另一条路是内核之外的第二份实现：Studio 的
/// Edit 表单预填需要紧凑形式，而本函数私有时，它只能自己拼同一套语法——连内核刚修好的 deny
/// 列表被放宽的问题也一并复制（`FIXR-01`）。展示或保存紧凑形式的执行面调用本函数，不得自行决定
/// 拼法。
///
/// One clause per list and `;` between them, in the historical order
/// (`allow:ui,controls`, `deny:ui/experimental`, `ANY` for the unconstrained gate,
/// `allow:ui;deny:ui/experimental` when a declaration names both lists). A
/// declaration that names one list keeps the spelling consumers already read, so
/// routing the historical single-list branches through this function changed no
/// bytes; a declaration that names both renders both, because the deny list is the
/// veto `Admission::accepts` gives priority to and a rendering that dropped it
/// would widen the gate it shows.
/// 每张列表一个子句，之间用 `;`，顺序沿用历史（`allow:ui,controls`、
/// `deny:ui/experimental`、表示不受限门禁的 `ANY`，同时点名两张列表时为
/// `allow:ui;deny:ui/experimental`）。只点名一张列表的声明保持消费方已在读的拼法，因此把历史
/// 单列表分支改走本函数没有改动任何字节；同时点名两张的声明把两张都渲染出来，因为 deny 列表是
/// `Admission::accepts` 优先采用的否决权，丢掉它的渲染会放宽它所展示的那道门禁。
///
/// The output is exactly what [`parse_admission_owned`] reads back, and the two are
/// pinned against each other from outside the crate by
/// `core/tests/compact_admission_entry.rs`.
/// 输出正是 [`parse_admission_owned`] 读得回的那一份，两者由 crate 之外的
/// `core/tests/compact_admission_entry.rs` 互钉。
pub fn compact_admission(admission: &OwnedAdmission) -> String {
    let (allow, deny) = (&admission.allowed_paths, &admission.denied_paths);
    match (allow.is_empty(), deny.is_empty()) {
        (true, true) => "ANY".to_owned(),
        (false, true) => format!("allow:{}", allow.join(",")),
        (true, false) => format!("deny:{}", deny.join(",")),
        (false, false) => format!("allow:{};deny:{}", allow.join(","), deny.join(",")),
    }
}

/// Read an admission expression, accepting both the compact and constructor
/// spellings.
/// 读取 admission 表达式，同时接受紧凑写法与构造形式。
///
/// A constructor that names both lists is read as both clauses, so the deny list
/// reaches the editor and the next write instead of being dropped
/// (`both_lists_survive_the_read_and_write_round_trip`).
/// 同时点名两张列表的构造函数会读成两个子句，因此 deny 列表会抵达编辑器与下一次写盘，
/// 而不是被丢掉（见 `both_lists_survive_the_read_and_write_round_trip`）。
///
/// Every spelling is spelled by [`compact_admission`] and by nothing else: the
/// historical `allow_paths(`/`deny_paths(` branches are a policy with one list,
/// and the constructor form is a policy with two. That is the whole reason the
/// function is public — the kernel and any surface that shows this value render it
/// in one place, and the bytes of the historical spellings are pinned by
/// `core/tests/compact_admission_entry.rs`.
/// 每一种拼法都只由 [`compact_admission`] 拼出：历史的 `allow_paths(`/`deny_paths(` 分支就是
/// 只有一张列表的策略，构造形式则是两张列表的策略。这正是本函数公开的全部原因——内核与任何
/// 展示该值的执行面只有一处渲染它，而历史拼法的字节由 `core/tests/compact_admission_entry.rs`
/// 钉住。
pub fn parse_admission_expression(expression: &str) -> Result<String, FaceParseError> {
    let expression = expression.trim().trim_end_matches(',');
    let policy = |allowed_paths, denied_paths| {
        compact_admission(&OwnedAdmission {
            allowed_paths,
            denied_paths,
        })
    };
    if expression.is_empty() || expression.contains("Admission::ANY") {
        return Ok(policy(Vec::new(), Vec::new()));
    }
    let paths = quoted_list_field(expression, "allow_paths(");
    if !paths.is_empty() {
        return Ok(policy(paths, Vec::new()));
    }
    let paths = quoted_list_field(expression, "deny_paths(");
    if !paths.is_empty() {
        return Ok(policy(Vec::new(), paths));
    }
    // The editor writes the constructor form, because a face holds real Rust.
    // Reading only the `allow_paths(`/`deny_paths(` spellings meant the editor
    // could write a declaration it then refused to read back.
    // 编辑器写下的是构造函数形式，因为注册面里放的是真实 Rust。只认
    // `allow_paths(`/`deny_paths(` 会让编辑器写出自己读不回来的声明。
    if let Some((_, rest)) = expression.split_once("Admission::new(") {
        let mut parts = rest.split(']');
        let allow = parts.next().map(quoted_strings).unwrap_or_default();
        let deny = parts.next().map(quoted_strings).unwrap_or_default();
        return Ok(policy(allow, deny));
    }
    Err("generated face has an invalid admission expression"
        .to_owned()
        .into())
}

/// The quoted strings of one bracketed fragment, e.g. `&["a", "b"`.
/// 一个方括号片段里的字符串，例如 `&["a", "b"`。
fn quoted_strings(fragment: &str) -> Vec<String> {
    let fragment = fragment.rsplit_once('[').map_or(fragment, |(_, rest)| rest);
    fragment
        .split(',')
        .filter_map(|value| {
            let value = value.trim().trim_end_matches(']').trim();
            value
                .strip_prefix('"')
                .and_then(|value| value.strip_suffix('"'))
                .map(str::to_owned)
        })
        .collect()
}

/// One compact admission value split into the lists it names.
/// 一个紧凑 admission 值切分成它点名的那些列表。
///
/// `parse_admission_owned` and `render_admission` each used to trim, split at
/// the first `:`, check the empty list, and choose between `allow` and `deny` on
/// their own, including three separately written error strings. A message or an
/// empty-list rule changed on one side would let a value the snapshot parser
/// accepted be refused by the renderer. This is the family's only parser; the
/// renderer formats what it returns and the snapshot parser converts it. Clause
/// keys are read case-insensitively and with surrounding whitespace trimmed, so
/// both sides treat `ALLOW : a` as a path list.
/// `admission_render_and_parse_read_one_parser` pins the two entry points, and
/// `both_lists_survive_the_read_and_write_round_trip` pins that a two-clause
/// value keeps both lists.
/// `parse_admission_owned` 与 `render_admission` 过去各自去空白、在第一个 `:` 处切分、
/// 检查空列表、在 `allow` 与 `deny` 之间选择，连三条错误字符串都是各写一遍。只改一侧
/// 的消息或空列表规则，会让快照解析器接受的值被渲染器拒绝。这是该字段族唯一的解析器；
/// 渲染器格式化它的返回值，快照解析器转换它。子句键按大小写不敏感读取并去掉两侧空白，
/// 因此两侧都把 `ALLOW : a` 当作路径列表。
/// `admission_render_and_parse_read_one_parser` 钉住两个入口，
/// `both_lists_survive_the_read_and_write_round_trip` 钉住双子句值两张列表都不丢。
struct CompactAdmission {
    allowed_paths: Vec<String>,
    denied_paths: Vec<String>,
}

/// The one message a compact value the grammar does not define is refused with.
/// 语法未定义的紧凑值统一的拒绝文本。
fn admission_syntax_error() -> FaceParseError {
    "admission must be ANY, allow:path/prefix, deny:path/prefix, or allow:…;deny:…"
        .to_owned()
        .into()
}

/// Split a compact admission value, or `None` for the unconstrained gate.
/// 切分紧凑 admission 值；空值或 `ANY` 表示不受限门禁时返回 `None`。
///
/// Every clause names one list, and a value may carry one clause or both. An
/// unknown clause, a repeated clause, and a clause whose path list is empty are
/// all refused: silently dropping or doubling one of the two lists is exactly
/// what this parser exists to prevent.
/// 每个子句点名一张列表，一个值可以只带一个子句，也可以两个都带。未知子句、重复子句、
/// 路径列表为空的子句一律拒绝：静默丢掉或重复两张列表中的一张，正是本解析器存在要防的事。
fn parse_compact_admission(value: &str) -> Result<Option<CompactAdmission>, FaceParseError> {
    let value = value.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("any") {
        return Ok(None);
    }
    let clauses = value
        .split(';')
        .map(str::trim)
        .filter(|clause| !clause.is_empty())
        .collect::<Vec<_>>();
    if clauses.is_empty() {
        return Err(admission_syntax_error());
    }
    let mut allowed_paths = Vec::new();
    let mut denied_paths = Vec::new();
    for clause in clauses {
        let (key, paths) = clause.split_once(':').ok_or_else(admission_syntax_error)?;
        let paths = split_csv_owned(paths);
        if paths.is_empty() {
            return Err("admission must list at least one path".to_owned().into());
        }
        match key.trim().to_ascii_lowercase().as_str() {
            "allow" if !allowed_paths.is_empty() => {
                return Err("admission names `allow` twice".to_owned().into());
            }
            "deny" if !denied_paths.is_empty() => {
                return Err("admission names `deny` twice".to_owned().into());
            }
            "allow" => allowed_paths = paths,
            "deny" => denied_paths = paths,
            _ => return Err(admission_syntax_error()),
        }
    }
    Ok(Some(CompactAdmission {
        allowed_paths,
        denied_paths,
    }))
}

/// Parse a compact admission value into its owned snapshot form.
/// 将紧凑 admission 值解析为快照使用的拥有型形式。
///
/// Both lists survive: a value that names an allow list and a deny list produces
/// an `OwnedAdmission` carrying both, which is what the registration tree and the
/// connector read.
/// 两张列表都会保留：同时点名 allow 与 deny 的值产出一份携带两张列表的
/// `OwnedAdmission`，注册树与连接器读的正是它。
pub fn parse_admission_owned(value: &str) -> Result<OwnedAdmission, FaceParseError> {
    Ok(match parse_compact_admission(value)? {
        None => OwnedAdmission {
            allowed_paths: Vec::new(),
            denied_paths: Vec::new(),
        },
        Some(admission) => OwnedAdmission {
            allowed_paths: admission.allowed_paths,
            denied_paths: admission.denied_paths,
        },
    })
}

/// Render a compact admission value as the `Admission` constructor expression.
/// 将紧凑 admission 值渲染成 `Admission` 构造表达式。
///
/// A value that names both lists renders both: the deny list `Admission::accepts`
/// gives priority to has to reach the source, or the next build reads back a
/// wider gate than the author wrote.
/// 同时点名两张列表的值会把两张都渲染出来：`Admission::accepts` 优先采用的 deny 列表
/// 必须落到源码里，否则下一次构建读回的是一道比作者所写更宽的门禁。
pub fn render_admission(value: &str) -> Result<String, FaceParseError> {
    let Some(admission) = parse_compact_admission(value)? else {
        return Ok("crate::Admission::ANY".to_owned());
    };
    let quoted = |paths: &[String]| {
        paths
            .iter()
            .map(|path| format!("\"{}\"", rust_string(path)))
            .collect::<Vec<_>>()
            .join(", ")
    };
    Ok(format!(
        "crate::Admission::new(&[{}], &[{}])",
        quoted(&admission.allowed_paths),
        quoted(&admission.denied_paths)
    ))
}

#[cfg(test)]
mod tests {
    use super::{parse_admission_expression, parse_admission_owned, render_admission};
    use crate::registry_core::declaration::OwnedAdmission;

    /// The editor writes the constructor form and must be able to read it back;
    /// both spellings describe the same admission.
    /// 编辑器写下构造函数形式，也必须能读回来；两种写法描述同一个 admission。
    #[test]
    fn admission_round_trips_through_the_constructor_form() {
        assert_eq!(
            parse_admission_expression("crate::Admission::new(&[\"a\", \"b\"], &[])"),
            Ok("allow:a,b".to_owned())
        );
        assert_eq!(
            parse_admission_expression("crate::Admission::new(&[], &[\"c\"])"),
            Ok("deny:c".to_owned())
        );
        assert_eq!(
            parse_admission_expression("crate::Admission::new(&[], &[])"),
            Ok("ANY".to_owned())
        );
        assert_eq!(
            parse_admission_expression("crate::Admission::allow_paths(&[\"a\"])"),
            Ok("allow:a".to_owned())
        );
    }

    /// The renderer and the snapshot parser now share one parser, so a malformed
    /// value is refused with identical text on both sides and a well-formed one
    /// names the same lists and paths. Before the extraction each split the line,
    /// checked the empty list, and wrote the three messages separately.
    /// 渲染器与快照解析器现在共用同一个解析器，因此畸形值在两侧得到完全相同的拒绝文本，
    /// 合法值命名同样的列表与路径。抽取之前两者各自切分该行、检查空列表，并把三条消息
    /// 各写一遍。
    #[test]
    fn admission_render_and_parse_read_one_parser() {
        for malformed in ["allow", "allow:", "deny:", "veto:a", "ANY:extra"] {
            let rendered = render_admission(malformed).unwrap_err();
            let parsed = parse_admission_owned(malformed).unwrap_err();
            assert_eq!(rendered, parsed, "value `{malformed}`");
        }
        assert_eq!(
            render_admission("allow: ui, controls ").unwrap(),
            "crate::Admission::new(&[\"ui\", \"controls\"], &[])"
        );
        assert_eq!(
            parse_admission_owned("allow: ui, controls ").unwrap(),
            OwnedAdmission {
                allowed_paths: vec!["ui".to_owned(), "controls".to_owned()],
                denied_paths: Vec::new(),
            }
        );
        assert_eq!(
            render_admission("ALLOW : ui").unwrap(),
            "crate::Admission::new(&[\"ui\"], &[])"
        );
        assert_eq!(
            render_admission("deny:c").unwrap(),
            "crate::Admission::new(&[], &[\"c\"])"
        );
        assert_eq!(render_admission("ANY").unwrap(), "crate::Admission::ANY");
        assert_eq!(
            parse_admission_owned("").unwrap(),
            OwnedAdmission {
                allowed_paths: Vec::new(),
                denied_paths: Vec::new(),
            }
        );
    }

    /// A declaration that names both an allow list and a deny list must survive
    /// the read-and-write round trip intact. `Admission::accepts` gives the deny
    /// list priority, so a read that keeps only the allow list silently widens
    /// the gate — and the widened form is what gets written back to the source.
    /// 同时声明 allow 与 deny 两张列表的声明必须在“读一次再写回”的往返中完整存活。
    /// `Admission::accepts` 把否决权交给 deny 列表，因此只保留 allow 列表的读取会静默
    /// 放宽门禁——而写回源码的正是被放宽的那一份。
    #[test]
    fn both_lists_survive_the_read_and_write_round_trip() {
        let declaration = "crate::Admission::new(&[\"ui\"], &[\"ui/experimental\"])";
        let expected = OwnedAdmission {
            allowed_paths: vec!["ui".to_owned()],
            denied_paths: vec!["ui/experimental".to_owned()],
        };
        let compact = parse_admission_expression(declaration)
            .expect("the constructor form a real face carries is readable");
        assert_eq!(
            parse_admission_owned(&compact).expect("the compact form is readable"),
            expected,
            "reading `{declaration}` dropped the deny list: `{compact}`"
        );
        let rendered = render_admission(&compact).expect("the compact form renders");
        assert!(
            rendered.contains("ui/experimental"),
            "the rendered declaration must still carry the deny list: {rendered}"
        );
        assert_eq!(
            parse_admission_expression(&rendered).expect("the rendered form is readable"),
            compact,
            "read → render → read must be a fixed point"
        );
    }

    /// The compact spelling carries both lists, and a clause the grammar does
    /// not define — or the same clause twice — is refused instead of silently
    /// winning.
    /// 紧凑拼法同时承载两张列表；语法未定义的子句、以及同一子句出现两次，一律拒绝，
    /// 而不是静默胜出。
    #[test]
    fn the_compact_form_carries_both_lists_and_refuses_duplicate_clauses() {
        assert_eq!(
            parse_admission_owned("allow:ui;deny:ui/experimental").unwrap(),
            OwnedAdmission {
                allowed_paths: vec!["ui".to_owned()],
                denied_paths: vec!["ui/experimental".to_owned()],
            }
        );
        assert_eq!(
            render_admission("allow:ui;deny:ui/experimental").unwrap(),
            "crate::Admission::new(&[\"ui\"], &[\"ui/experimental\"])"
        );
        for malformed in ["allow:a;allow:b", "allow:a;veto:b", "allow:a;deny:"] {
            assert!(
                parse_admission_owned(malformed).is_err(),
                "parse accepted `{malformed}`"
            );
            assert!(
                render_admission(malformed).is_err(),
                "render accepted `{malformed}`"
            );
        }
    }
}
