//! The one outlet for a derived fact that rides on a row about something else.
//! "附带事实"的唯一出口：一条**派生**事实搭在一条本来在讲别的事的行上。
//!
//! Three readers do this. The census's branch column attaches the contract above a dead arm; the
//! adoption ledger attaches two fingerprints to an entry; `read --whole` attaches each function's
//! name and contract to the line that starts it. What they share is **not the wording** — each
//! keeps its own. What they share is the two rules an attachment has to obey: it is **bounded**, and
//! **when the bound bites it says where the rest is**. Written twice by hand, one of the two ends up
//! truncating silently, and a silent truncation in an attachment reads as "there was nothing more".
//! 有三个读者这么做：总账的分支栏给一条死臂附上它上方的契约；采信台账给一条条目附上两串指纹；
//! `read --whole` 给每个函数的起始行附上它的名字与契约。它们共享的**不是措辞**——各自保留自己的。
//! 共享的是附带事实必须守的两条规则：**有界**，以及**上限一旦生效就要说出剩下的在哪**。这两条手写两遍，
//! 其中一遍迟早会静默截断，而附带内容里的静默截断读起来就是"没有更多了"。

/// Numbered lines, bounded, with the way to the rest when the bound bites.
/// 带行号的若干行，有界；上限生效时给出剩下的在哪。
///
/// The numbering carried through is the **source** line each entry sits on, not its position here:
/// a reader follows the number back into the file.
/// 带上的是每一条所在的**源码**行号，不是它在这里的位置：读者要拿这个号回到文件里。
pub(crate) fn numbered(lines: &[(usize, String)], budget: usize, pointer: &str) -> String {
    let shown = lines
        .iter()
        .take(budget)
        .map(|(line, text)| format!("{line}: {text}"))
        .collect::<Vec<_>>()
        .join(" / ");
    if lines.len() > budget {
        format!("{shown} (first {budget} lines; {pointer})")
    } else {
        shown
    }
}

/// The first twelve characters of a fingerprint: long enough to tell two apart in a reply, short
/// enough that four of them do not crowd out the facts beside them.
/// 指纹的前十二个字符：长到足以在一次回复里分辨两者，短到四个并排也不会挤掉旁边的事实。
pub(crate) fn brief(value: &str) -> &str {
    value.get(..12).unwrap_or(value)
}

#[cfg(test)]
#[path = "note_tests.rs"]
mod note_tests;
