//! What a workflow's `if:` conditions and inputs *say*, read as shapes rather than text.
//! 工作流的 `if:` 条件与输入**说了什么**——把两者当形状读，而不是当文本读。
//!
//! Split out of `release_workflow.rs`, which had one line of headroom left under the 600-line
//! ratchet once its tests moved to `release_workflow_tests.rs` (`N-2`, measured in the
//! B4-conventions independent verification). The seam is *reading* the text versus *walking*
//! the workflow: this module decides whether a condition is a positive conjunction containing
//! the tag test, and whether the text can read an input, while the parent keeps the traversal
//! (steps, delegation, upload detection) and the findings it produces. The two readers share
//! one lexical layer — `split_top_level`, `strip_outer_parentheses`, `starts_word`,
//! `is_word_character`, `is_quoted_literal` — which is why they are one file rather than two.
//! 从 `release_workflow.rs` 拆出：那个文件在测试移到 `release_workflow_tests.rs` 之后，在 600 行
//! 棘轮下只剩一行头寸（`N-2`，在 B4-conventions 的独立验证里实测）。切缝是"**读**文本"与
//! "**走**工作流"：本模块判定一个条件是否为含 tag 判断的肯定式合取、以及文本是否能读到输入，而父
//! 模块保留遍历（步骤、委托、上传判定）与它产出的发现。两个读法共用同一层词法
//! （`split_top_level`、`strip_outer_parentheses`、`starts_word`、`is_word_character`、
//! `is_quoted_literal`），这正是它们放同一个文件而不是两个的原因。

use super::TAG_GUARD;

/// How a step's own `if:` relates to the tag requirement.
/// 步骤自己的 `if:` 与 tag 要求的关系。
pub(super) enum GuardShape {
    /// Positive tests in conjunction, one of them the tag test itself.
    /// 肯定式测试的合取，其中之一就是 tag 判断本身。
    Tag,
    /// Positive tests in conjunction, with no tag test among them.
    /// 肯定式测试的合取，但不含 tag 判断。
    NoTagTest,
    /// Off the whitelist — a negation, a `||`, an `!=`, or an unenumerated operator.
    /// 白名单之外——取反、`||`、`!=`，或门禁没有列举的运算符。
    Other,
}

/// Which of the three shapes a step's `if:` is written in.
/// 某个步骤的 `if:` 写成了这三种形状里的哪一种。
pub(super) fn guard_shape(condition: &str) -> GuardShape {
    // `${{ … }}` is the same shape with the explicit expression wrapper around it.
    // `${{ … }}` 只是同一形状外面套了显式表达式外壳。
    let condition = condition.trim();
    let unwrapped = condition
        .strip_prefix("${{")
        .and_then(|rest| rest.strip_suffix("}}"))
        .map(str::trim)
        .unwrap_or(condition);
    let Some(tests) = positive_tests(unwrapped) else {
        return GuardShape::Other;
    };
    let tag = compact_shape(TAG_GUARD);
    if tests.iter().any(|test| compact_shape(test) == tag) {
        GuardShape::Tag
    } else {
        GuardShape::NoTagTest
    }
}

/// The tests of a whitelisted condition, or `None` if any part of it is off the list: the
/// list holds only the tag test itself and a `path == 'literal'` comparison. A shape a
/// maintainer wants that is not on it — `!cancelled()`, an `!=`, a `>` — is added here, not
/// worked around in the workflow.
/// 白名单条件下的一组测试；其中任何一部分不在名单上就返回 `None`——名单上只有 tag 判断本身和
/// `path == 'literal'` 比较。维护者想要而名单上没有的形状——`!cancelled()`、`!=`、`>`——在这里
/// 添加，而不是在工作流里绕开它。
fn positive_tests(condition: &str) -> Option<Vec<String>> {
    let mut tests = Vec::new();
    for part in split_top_level(condition, "&&") {
        let part = part.trim();
        let inner = strip_outer_parentheses(part);
        if inner == part {
            if !is_positive_test(part) {
                return None;
            }
            tests.push(part.to_owned());
            continue;
        }
        // A parenthesized group is one test of its own, judged by the same measure.
        // 括号里的组自身就是一个测试，用同一把尺子判。
        tests.extend(positive_tests(inner)?);
    }
    Some(tests)
}

/// Whether one test is one of the two whitelisted shapes. `!=` cannot be one: there is no
/// `==` to split on, so it drops out with every operator this gate does not enumerate.
/// 某个测试是否为白名单上那两种形状之一。`!=` 不可能是：没有 `==` 可切，因此它与门禁没有列举的
/// 每个运算符一样被排除。
fn is_positive_test(test: &str) -> bool {
    if test.contains("||") {
        return false;
    }
    if compact_shape(test) == compact_shape(TAG_GUARD) {
        return true;
    }
    let Some((left, right)) = test.split_once("==") else {
        return false;
    };
    is_path(left.trim()) && is_quoted_literal(right.trim())
}

/// Whether `text` is a dotted path of words: `github.event_name`, `github.ref`; the
/// bracket spelling (`github['ref']`) is not one.
/// `text` 是否为由词组成的点分路径：`github.event_name`、`github.ref`；方括号拼法不算。
fn is_path(text: &str) -> bool {
    text.split('.').all(|word| {
        let mut characters = word.chars();
        characters.next().is_some_and(starts_word) && characters.all(is_word_character)
    })
}

/// Whether `character` can start a word.
/// `character` 是否可以作为词的开头。
fn starts_word(character: char) -> bool {
    character.is_ascii_alphabetic() || character == '_'
}

/// Whether `character` can be part of a word, so `inputs` is not matched inside a longer
/// one.
/// `character` 是否可以是词的一部分——这样 `inputs` 不会在更长的词里被匹配到。
fn is_word_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

/// Whether `text` is one quoted literal, in either quote style GitHub accepts.
/// `text` 是否为单个引号字面量，两种 GitHub 接受的引号都算。
fn is_quoted_literal(text: &str) -> bool {
    let Some(quote) = text.chars().next() else {
        return false;
    };
    if !matches!(quote, '\'' | '"') || text.len() < 2 || !text.ends_with(quote) {
        return false;
    }
    !text[1..text.len() - 1].contains(quote)
}

/// `text` normalized for whitespace and quote style, so two spellings of one shape compare.
/// 把 `text` 的空白与引号风格归一，使同一形状的两种写法可以比较。
fn compact_shape(text: &str) -> String {
    text.chars()
        .filter(|character| !character.is_whitespace())
        .map(|character| if character == '"' { '\'' } else { character })
        .collect()
}

/// `text` without one balanced pair of parentheses around the *whole* of it: anything after
/// the closing parenthesis means the parentheses are part of the expression, not a wrapper.
/// 去掉包住**整段**文本的那一对配平括号之后的 `text`：右括号之后还有内容，说明这对括号是表达式
/// 本身的一部分，而不是外壳。
fn strip_outer_parentheses(text: &str) -> &str {
    let trimmed = text.trim();
    if !trimmed.starts_with('(') {
        return trimmed;
    }
    let mut depth = 0usize;
    let mut quote: Option<char> = None;
    for (offset, character) in trimmed.char_indices() {
        if let Some(open) = quote {
            quote = (character != open).then_some(open);
            continue;
        }
        match character {
            '\'' | '"' => quote = Some(character),
            '(' => depth += 1,
            ')' if depth == 1 => {
                return if trimmed[offset + 1..].trim().is_empty() {
                    trimmed[1..offset].trim()
                } else {
                    trimmed
                };
            }
            ')' => depth -= 1,
            _ => {}
        }
    }
    trimmed
}

/// `text` split on every separator outside parentheses and quoted literals.
/// 把 `text` 在每个不在括号内、也不在引号字面量内的分隔符处切开。
fn split_top_level<'a>(text: &'a str, separator: &str) -> Vec<&'a str> {
    let mut parts = Vec::new();
    let (mut start, mut at, mut depth, mut quote) = (0usize, 0usize, 0usize, None::<char>);
    while at < text.len() {
        let rest = &text[at..];
        let character = rest.chars().next().expect("at is a character boundary");
        if let Some(open) = quote {
            quote = (character != open).then_some(open);
        } else if depth == 0 && rest.starts_with(separator) {
            parts.push(&text[start..at]);
            start = at + separator.len();
            at = start;
            continue;
        } else {
            match character {
                '\'' | '"' => quote = Some(character),
                '(' => depth += 1,
                ')' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
        at += character.len_utf8();
    }
    parts.push(&text[start..]);
    parts
}

/// Whether the workflow reads an input, declared or referenced, in either spelling.
/// 该工作流是否读取输入——声明或引用都算，两种拼法都算。
///
/// The reference halves are read as references, not as text: GitHub accepts `inputs.name`
/// and `inputs['name']`, and a substring test for `inputs.` found only the first — a
/// workflow reading `inputs['publish']` produced no finding at all. A comment is prose and
/// cannot read an input, which is the boundary the upload test already draws.
/// 引用那两半按"引用"读，而不按文本读：GitHub 接受的拼法只有 `inputs.name` 与
/// `inputs['name']` 两种，而对 `inputs.` 做子串判定只能找到第一种——读 `inputs['publish']`
/// 的工作流完全不产生发现。注释是散文，读不了输入，这也是上传判定早已画出的边界。
pub(super) fn takes_input(text: &str) -> bool {
    text.lines()
        .any(|line| line.trim() == "inputs:" || line_reads_input(line))
}

/// Whether one line names an input reference: `inputs.name` or `inputs['name']`.
/// 某一行是否命名了一处输入引用：`inputs.name` 或 `inputs['name']`。
fn line_reads_input(line: &str) -> bool {
    if line.trim_start().starts_with('#') {
        return false;
    }
    let mut rest = line;
    while let Some(offset) = rest.find("inputs") {
        let after = &rest[offset + "inputs".len()..];
        // The word has to be whole: `myinputs.name` names something else.
        // 这个词必须是完整的：`myinputs.name` 命名的是别的东西。
        let whole_word = !rest[..offset]
            .chars()
            .next_back()
            .is_some_and(is_word_character);
        if whole_word && names_input_key(after) {
            return true;
        }
        rest = after;
    }
    false
}

/// Whether `after` continues an `inputs` word as a key: `.name` or `['name']`.
/// `after` 是否把 `inputs` 这个词接成一个键：`.name` 或 `['name']`。
fn names_input_key(after: &str) -> bool {
    let after = after.trim_start();
    if let Some(dotted) = after.strip_prefix('.') {
        return dotted.trim_start().chars().next().is_some_and(starts_word);
    }
    let Some(bracketed) = after.strip_prefix('[').map(str::trim_start) else {
        return false;
    };
    let Some(close) = bracketed.find(']') else {
        return false;
    };
    is_quoted_literal(bracketed[..close].trim_end())
}
