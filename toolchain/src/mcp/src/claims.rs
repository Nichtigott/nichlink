//! The tree-wide half of a `check`: what the tree declares and how that declaration can drift,
//! in the two forms that are decidable without a model.
//! `check` 的"全树"那一半：这棵树声明了什么、以及这些声明会怎么漂移——只做两种**不需要模型**就能
//! 判定的形态。
//!
//! A symptom narrows the scope, and that is the right shape for a symptom (the caller names the failing
//! assertion, the face and the call sites do the rest). An **open** question — "are there other
//! problems in this tree" — has no such narrowing, and the measured failure was that nothing answered
//! it: one arm hand-swept every public doc to find a re-spelled constant, the other only compared the
//! contracts on the symptom's own path and never saw it.
//! 症状会自己收窄范围，那对症状是对的（调用方给出失败断言，面与调用点做完剩下的事）。**开放式**的问题
// ——"这棵树里还有别的问题吗"——没有这种收窄，而量到的失败正是没有任何东西回答它：一个臂手工扫遍所有
// 公开文档才找到那处被重拼的常量，另一个只比对了症状路径上的契约、因此从没看见它。
//!
//! Every line here is a **static fact about the tree**, not a verdict about the code, and the section
//! ends by saying what it does not cover — a census that claimed completeness would be the same
//! false green this repository keeps deleting.
//! 这里的每一行都是**关于这棵树的静态事实**，不是对代码的裁定，而且这一节末尾会说明它**不覆盖**什么
// ——一份自称完备的总账，就是这个仓库一直在删的那种假绿。

use std::path::Path;

const SAMPLE: usize = 5;

/// One declared numeric constant, as the tree spells it.
/// 一条被声明的数值常量，按这棵树里的写法。
struct Declared {
    name: String,
    value: String,
    relative: String,
    line: usize,
}

/// Whether `text` uses `value` as a whole number, with no word character or `.` next to it.
/// `text` 是否把 `value` 当作一个完整的数字使用——两侧既不是词字符、也不是 `.`。
fn uses_whole_number(text: &str, value: &str) -> bool {
    let bytes = text.as_bytes();
    let mut from = 0;
    while let Some(at) = text[from..].find(value) {
        let start = from + at;
        let end = start + value.len();
        let before_ok = start == 0 || {
            let before = bytes[start - 1] as char;
            !before.is_alphanumeric() && before != '_' && before != '.'
        };
        let after_ok = end >= bytes.len() || {
            let after = bytes[end] as char;
            !after.is_alphanumeric() && after != '_' && after != '.'
        };
        if before_ok && after_ok {
            return true;
        }
        from = end;
    }
    false
}

/// The census, as answer lines: the header, one line per finding, and the coverage sentence.
/// 总账，按答案行给出：表头、每条发现一行、以及覆盖范围那一句。
pub(crate) fn census(root: &Path) -> Result<Vec<String>, String> {
    let sources = crate::mcp::source_index::load_sources(root)?;
    let mut declared: Vec<Declared> = Vec::new();
    for file in &sources {
        for (index, line) in file.source.lines().enumerate() {
            let Some(rest) = line.trim().strip_prefix("pub const ") else {
                continue;
            };
            let (Some(name), Some(value)) = (rest.split(':').next(), rest.split('=').nth(1)) else {
                continue;
            };
            let value = value.trim().trim_end_matches(';').trim().replace('_', "");
            if !value.is_empty() && value.chars().all(|digit| digit.is_ascii_digit()) {
                declared.push(Declared {
                    name: name.trim().to_owned(),
                    value,
                    relative: file.relative.clone(),
                    line: index + 1,
                });
            }
        }
    }
    let mut lines = vec![format!(
        "census: {} named numeric constant(s); a static fact about this tree, not a verdict",
        declared.len()
    )];
    let mut found: Vec<String> = Vec::new();
    for constant in &declared {
        for file in &sources {
            for (index, text) in file.source.lines().enumerate() {
                let at = format!("{}:{}", file.relative, index + 1);
                if at == format!("{}:{}", constant.relative, constant.line) {
                    continue;
                }
                if uses_whole_number(text, &constant.value) {
                    found.push(format!(
                        "  respelled {} is declared as `{}` ({}:{}) and written again at {at}",
                        constant.value, constant.name, constant.relative, constant.line
                    ));
                }
            }
        }
        let production = sources
            .iter()
            .filter(|file| !crate::mcp::callgraph::looks_like_a_test(&file.relative, &file.source))
            .filter(|file| file.relative != constant.relative)
            .filter(|file| file.source.contains(&constant.name))
            .count();
        if production == 0 {
            found.push(format!(
                "  unreferenced `{}` ({}:{}) is not read anywhere outside tests",
                constant.name, constant.relative, constant.line
            ));
        }
    }
    let total = found.len();
    for line in found.iter().take(SAMPLE) {
        lines.push(line.clone());
    }
    if total > SAMPLE {
        lines.push(crate::mcp::truncation::withheld(
            total - SAMPLE,
            total,
            SAMPLE,
            "census items",
            "ask per directory to see its own items",
        ));
    }
    // The third column: what the entry plan declares, as a static count. Families a and b both spent
    // real effort working out that "declaring one more cut" is what changes what the application
    // ships — and both arms derived it by experiment. The count itself is a fact this tree already
    // contains, so the census reports it and lets the caller draw the conclusion.
    // 第三栏：入口计划声明了什么，按静态计数给出。a/b 两族都花了真实力气才弄清"多声明一条 cut"才是
    // 改变"这个应用发布什么"的那一步——而两臂都是靠实验推出来的。计数本身是这棵树已有的事实，总账把它
    // 报出来，结论留给调用方。
    let cuts = sources
        .iter()
        .map(|file| file.source.matches("cut(").count())
        .sum::<usize>();
    let grafts = sources
        .iter()
        .map(|file| file.source.matches("graft(").count())
        .sum::<usize>();
    lines.push(format!(
        "  entry plan: {cuts} `cut(` site(s) and {grafts} `graft(` site(s) across this tree's sources \
         (a static count; which of them this application ships is the plan's own business)"
    ));
    lines.push(
        "  not covered: only named numeric constants; string constants, structural duplication \
         and claims written in prose are outside this census"
            .to_owned(),
    );
    Ok(lines)
}
