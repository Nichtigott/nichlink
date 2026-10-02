//! The adoption ledger at `<root>/.nichlink/adopted/entries`: which routes this tree trusts
//! **provisionally**, and whether the bytes each decision was taken on are still here.
//! `<root>/.nichlink/adopted/entries` 上的采信台账：这棵树**暂时**采信哪些路线，以及每次决定据以
//! 做出的那些字节是否还在。
//!
//! The maintainer's rule for this surface: 「采信是可以改的松动的，但是要强制人去确认可修改，因为肯定
//! 是稳步发展的，只是暂时采信」. So reading never renews anything, a lapsed entry is reported as a
//! fact with the file to look at, and the only way a line is added is a request that says `apply`,
//! says `confirm`, and names who confirmed it and why. The word this module never prints is
//! "verified": an adoption is a lease, and the honest level is `(provisional)`.
//! 维护者对这个面的规矩：「采信是可以改的松动的，但是要强制人去确认可修改，因为肯定是稳步发展的，只是暂时
//! 采信」。因此读取永不续期，失效条目被当作一个事实报出并点名该去看的文件，而唯一的加行途径是一个说出
//! `apply`、说出 `confirm`、并点名"谁确认的、为什么"的请求。本模块永不打印的词是"已验证"：采信是租约，
//! 诚实的等级就是 `(provisional)`。

use std::path::{Path, PathBuf};

use nichlink_kernel::adoption::{
    AdoptionEntry, AdoptionVerdict, adoption_fingerprint, parse_adoption, state_of,
};
use nichlink_kernel::lexicon::{ADOPTION_DIR, ADOPTION_FILE, NICHLINK_DIR};
use serde_json::Value;

use crate::mcp::freshness::wall_clock;

/// The ledger path for one package root.
/// 一个包根的台账路径。
fn ledger_path(root: &Path) -> PathBuf {
    root.join(NICHLINK_DIR)
        .join(ADOPTION_DIR)
        .join(ADOPTION_FILE)
}

/// Read the files a decision rests on, refusing to leave the root.
/// 读取一次决定依托的文件，拒绝离开根。
///
/// Shared with `consistency --specimen`, which compares the shape those same bytes declare: one
/// reader for "the bytes a lease rests on", so a containment rule cannot drift between two copies.
/// 与 `consistency --specimen` 共用——它比较的正是同一批字节所声明的形状：租约依托哪些字节只有一个读法，
/// 因此"不许离开根"这条规则不会在两份副本之间漂移。
pub(crate) fn read_files(root: &Path, files: &[String]) -> Result<Vec<(String, String)>, String> {
    let mut read = Vec::new();
    for file in files {
        let relative = Path::new(file);
        if relative.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir | std::path::Component::RootDir
            )
        }) {
            return Err(format!(
                "`{file}` must stay inside the configured source root"
            ));
        }
        let path = root.join(relative);
        let contents = std::fs::read_to_string(&path)
            .map_err(|error| format!("{} is not readable: {error}", path.display()))?;
        read.push((file.clone(), contents));
    }
    Ok(read)
}

/// Report every ledger entry, or add one when the request asks to.
/// 报出台账里的每条条目；当请求要求写入时追加一条。
/// The ledger's parsed entries, for readers that need the rows rather than a rendered verdict.
/// 台账解析后的条目，供需要"行"而不是"渲染结论"的读者使用。
///
/// The parser is the kernel's, so a second reading of the same format cannot drift from it.
/// 解析器是内核那一个，因此同一格式不会出现第二份会漂移的读法。
pub(crate) fn entries(root: &Path) -> Result<Vec<AdoptionEntry>, String> {
    let path = ledger_path(root);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    parse_adoption(&text)
        .map_err(|error| format!("{}:{}: {}", path.display(), error.line, error.message))
}

/// What an anchor's claim is right now: the entry in force, its verdict, and the ledger's history.
/// 某个 anchor 的声明现在处于什么状态：生效的那一条、它的判定，以及台账里的修订次数。
///
/// The question is the ledger's own, so it is answered here rather than in a second module that
/// would have to re-read the same file. The lease semantics are the kernel's: the **newest** line
/// for an anchor is the one in force.
/// 这个问题是台账自己的，因此在这里回答，而不是另开一个模块再读一遍同一个文件。租约语义来自内核：
/// 同一个 anchor 的**最后一条**生效。
pub(crate) fn conformance(root: &Path, arguments: &Value) -> Result<String, String> {
    let entries = entries(root)?;
    let requested = arguments
        .get("anchor")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|anchor| !anchor.is_empty());
    // No anchor means **every** anchor, one line each. Measured (T-21): asking about two entries cost
    // two calls, because the argument was required — and the fact a caller wants is usually "which of
    // these still hold", not "the third one's history". The per-anchor view below is unchanged.
    // 不带 anchor 就是**每个**锚点一行。量到的（T-21）：问两条条目要两次调用，因为这个参数是必填的——而调用方
    // 通常要的是"这些里面哪些还成立"，不是"第三条的历史"。下面的单锚点视图一个字没改。
    let Some(anchor) = requested else {
        if entries.is_empty() {
            return Ok(format!(
                "no adoption ledger at {}\nAn adoption is provisional and needs a person to \
                 confirm it: write one with `anchor`, `certifies`, `evidence`, `verifier`, `reason` \
                 and `files`, plus `apply` and `confirm`.\n",
                ledger_path(root).display()
            ));
        }
        let mut output = format!(
            "evidence: adoption ledger at {} (an adoption is a lease: the newest line for an anchor \
             is the one in force)\n",
            ledger_path(root).display()
        );
        let mut provisional = 0usize;
        let mut lapsed = 0usize;
        let mut seen: Vec<&str> = Vec::new();
        for entry in &entries {
            if seen.contains(&entry.anchor.as_str()) {
                continue;
            }
            seen.push(&entry.anchor);
            let in_force = entries
                .iter()
                .rfind(|candidate| candidate.anchor == entry.anchor)
                .expect("the anchor came from this list");
            let current = read_files(root, &in_force.files)?;
            let state = state_of(in_force, &current);
            match &state.verdict {
                AdoptionVerdict::Lapsed { file } => {
                    lapsed += 1;
                    output.push_str(&format!("  lapsed      {} ({file})\n", in_force.anchor));
                }
                _ => {
                    provisional += 1;
                    output.push_str(&format!("  in force    {}\n", in_force.anchor));
                }
            }
        }
        output.push_str(&format!(
            "provisional {provisional}  lapsed {lapsed}\nnext   `conformance {{anchor: \"<one of \
             the above>\"}}` for one anchor's history, `adopted` for the entries in full\n"
        ));
        return Ok(output);
    };

    let history: Vec<&AdoptionEntry> = entries
        .iter()
        .filter(|entry| entry.anchor == anchor)
        .collect();
    if history.is_empty() {
        return Ok(format!(
            "no ledger entry names `{anchor}` in {} — the ledger holds {} entry(ies){}\n",
            root.display(),
            entries.len(),
            if entries.is_empty() {
                " (there is no ledger here)"
            } else {
                "; `adopted` lists the anchors"
            }
        ));
    }
    let effective = history.last().expect("non-empty history");
    let current = read_files(root, &effective.files)?;
    let mut lines = vec![format!(
        "anchor `{anchor}` — {} revision(s) in the ledger; the last is in force (an adoption is a \
         lease: the newest line wins)",
        history.len()
    )];
    let state = state_of(effective, &current);
    match &state.verdict {
        AdoptionVerdict::Provisional => lines.push(format!(
            "  in force   provisional — certifies: {} (adopted {} by {})",
            effective.certifies, effective.at, effective.verifier
        )),
        AdoptionVerdict::Lapsed { file } => lines.push(format!(
            "  in force   lapsed at {file}: the bytes moved after the confirmation, so this needs a \
             **person**, not an edit (adopted {} by {})",
            effective.at, effective.verifier
        )),
    }
    lines.push(format!(
        "  bytes      {}",
        lease(
            &effective.fingerprint,
            &state.current,
            &effective.files,
            false
        )
    ));
    lines.push(format!("  covers     {}", effective.files.join(", ")));
    lines.push(
        "not covered here: whether the siblings of this anchor's object follow the same shape (ask \
         `consistency --specimen <anchor>`, which reads the declared fields from the files this \
         entry covers and compares the siblings against them), and the shape of the ledger's own \
         history beyond the newest line"
            .to_owned(),
    );
    lines.push(
        format!(
            "next   `adopted` for every entry's verdict, `consistency --specimen {anchor}` for \
             whether the siblings follow this shape"
        )
        .to_owned(),
    );
    Ok(format!("{}\n", lines.join("\n")))
}

/// The two fingerprints a reader needs to judge a lease without re-deriving the rule.
/// 读者判断一条租约所需的两串指纹——不必再把规则重推一遍。
///
/// Measured need (W8, h2): the client wanted to know whether a lapsed entry's recorded fingerprint
/// described some *other* file's current bytes, and the reply carried neither value, so it read this
/// tree's sources and rebuilt the kernel's fingerprint composition by hand — 18,177 characters of
/// reasoning, 11.5% of that arm's chain.
/// 量出来的需求（W8 的 h2）：客户端想知道一条失效条目的记录指纹是否描述了**别的文件**的当前字节，而
/// 回复里两个值都没有，于是它去读本树的源码、手工把内核的指纹组合重新拼出来——18,177 字符的推理，占
/// 该臂整条链的 11.5%。
/// What the bytes say, in the reader's terms: they are the ones the lease was taken on, or they are
/// not — and **which file** is not.
/// 字节在读者眼里是什么：它们还是这份租约当初针对的那一份，或者不是——以及**哪个文件**不是。
///
/// The recorded value is a print (a hash over the covered set), and printing it was a mistake: a
/// twelve-character hex string is a **comparison key**, not information — a reader can neither find a
/// file with it nor change a line with it, and its only content appears when two of them sit side by
/// side. What a reader needs is the verdict per file, so that is what this renders. Where the record
/// cannot support that verdict, it says so instead of hiding it: one print for a set of files can say
/// "something in here moved" and never "this one did", and that limit belongs to the record's shape.
/// 记录值是一个指纹（对被覆盖集合取的散列），而把它印出来是错的：一串十二位十六进制是**比较键**，不是信息
/// ——读者既不能拿它去找文件，也不能拿它去改某一行，而它唯一的内容只在两串并排时才出现。读者要的是**逐文件
/// 的判定**，所以这里印的是判定。记录撑不住这个判定的地方，就如实说出来而不是藏起来：对一组文件只留一个指纹，
/// 能说"这里面有东西动了"，永远说不出"是这一个"——那是**记录形状**的限制。
fn lease(recorded: &str, current: &str, files: &[String], file_already_named: bool) -> String {
    if recorded == current {
        return "unchanged since the confirmation".to_owned();
    }
    match files {
        [only] if file_already_named => "changed since the confirmation".to_owned(),
        [only] => format!("changed since the confirmation ({only})"),
        many => format!(
            "changed since the confirmation — one or more of the {} covered files did, and which \
             one is **not in the record**: a single print covers the set, so it cannot say. A \
             per-file record would",
            many.len()
        ),
    }
}

pub(crate) fn adopted(root: &Path, arguments: &Value) -> Result<String, String> {
    let ledger = ledger_path(root);
    if arguments.get("anchor").and_then(Value::as_str).is_some() {
        return renew(root, arguments, &ledger);
    }
    let text = std::fs::read_to_string(&ledger).unwrap_or_default();
    if text.trim().is_empty() {
        return Ok(format!(
            "no adoption ledger at {}\nAn adoption is provisional and needs a person to confirm \
             it: write one with `anchor`, `certifies`, `evidence`, `verifier`, `reason` and \
             `files`, plus `apply` and `confirm`.\n",
            ledger.display()
        ));
    }
    let entries = parse_adoption(&text)
        .map_err(|error| format!("{}:{} {}", ledger.display(), error.line, error.message))?;
    // The ledger's **path** rides on the first line in both cases. It used to appear only when the
    // ledger was empty, so a reader with a populated ledger had to go looking for the file — measured
    // in T-21: the arm spent shell calls on `find`/`ls` before it could read the two lines it was
    // asking about, and a reader that has to find the record is a reader one call poorer.
    // 台账**路径**在两种情形下都上第一行。它过去只在台账为空时出现，于是面对一份有内容的台账的读者得自己去找
    // 那个文件——T-21 实测：那一臂先花了若干 shell 调用 `find`/`ls`，才能读它在问的那两行；而要自己去找记录的
    // 读者，就是少了一次调用的读者。
    let mut output = format!(
        "evidence: adoption ledger at {} (provisional by construction)\n",
        ledger.display()
    );
    let (mut provisional, mut lapsed) = (0usize, 0usize);
    let mut first_lapsed: Option<String> = None;
    // Every entry's current print, computed once, so a lapsed line can say **whose** print its
    // recorded one is. Measured (T-21, h2): the arm spent ~15,000 characters working out that
    // `edc72845…` in a line naming `panel.rs` is `button.rs`'s print — the ledger's two hex strings
    // say what the bytes are, never whose, and the bridge already holds every covered file.
    // 每条的当前指纹先算一遍，好让失效的那一行说出它记录的那串**是谁的**。量到的（T-21 的 h2）：那一臂花了
    // 约 15,000 字符才弄清"点名 `panel.rs` 的那一行里的 `edc72845…` 是 `button.rs` 的"——台账给的两串十六进制
    // 说了"是什么"，从不说"是谁的"，而桥本来就握着每个被覆盖的文件。
    let mut currents: Vec<(String, String)> = Vec::new();
    for entry in &entries {
        let current = read_files(root, &entry.files)?;
        currents.push((entry.anchor.clone(), adoption_fingerprint(&current)));
    }
    for entry in &entries {
        let current = read_files(root, &entry.files)?;
        let state = state_of(entry, &current);
        // The list row already says `lapsed at <file>`, so the bytes clause does not name it twice.
        // 列表那一行已经说了 `lapsed at <文件>`，因此字节那句不再重复点名。
        let bytes = lease(&entry.fingerprint, &state.current, &entry.files, true);
        match &state.verdict {
            AdoptionVerdict::Provisional => {
                provisional += 1;
                output.push_str(&format!(
                    "adopted since {} ({bytes}) (provisional) — {}: {} [evidence: {}; confirmed by \
                     {}; why: {}]\n",
                    entry.at,
                    entry.anchor,
                    entry.certifies,
                    entry.evidence,
                    entry.verifier,
                    entry.reason,
                ));
            }
            AdoptionVerdict::Lapsed { file } => {
                lapsed += 1;
                first_lapsed.get_or_insert_with(|| entry.anchor.clone());
                // Whose print the recorded one is, when it is another entry's: that is the difference
                // between "the bytes moved" and "this line carries a copy of another file's print",
                // and only the second one explains a ledger that names one file and prints another.
                // 记录的那串是谁的（当它是别的条目的时）：这就是"字节动了"与"这一行带着另一个文件的指纹副本"
                // 之间的差别，而只有后者能解释"点名一个文件、却印着另一个文件的指纹"的台账。
                let borrowed = currents
                    .iter()
                    .find(|(anchor, print)| anchor != &entry.anchor && print == &entry.fingerprint)
                    .map(|(anchor, _)| anchor.clone());
                let whose = match &borrowed {
                    Some(anchor) => format!(
                        "; the recorded print is `{anchor}`'s current print, not this file's — a \
                         copy of another entry's bytes"
                    ),
                    None => String::new(),
                };
                output.push_str(&format!(
                    "adoption lapsed at {file} ({bytes}){whose}; needs confirmation — {}: {} [adopted \
                     at {} by {}; why: {}]\n",
                    entry.anchor, entry.certifies, entry.at, entry.verifier, entry.reason,
                ));
            }
        }
    }
    output.push_str(&format!("provisional {provisional}  lapsed {lapsed}\n"));
    output.push_str(
        "note: nothing here renews or revokes an adoption — a lapsed one needs a person to \
         confirm the new state, and a confirmation is one more line. A later line for the same \
         anchor is the one in force; an earlier line stays as its history, which is why a lapsed \
         line can sit above a provisional one and still be the truth about the bytes it named.\n",
    );
    // The state was complete and the action was missing: the round-5 evaluation's other arm read
    // this ledger's rules, concluded that a confirmation belongs to a person, and therefore changed
    // nothing — while the question was what to do about a route the ledger does not name. Naming
    // that action here is not a licence to forge a confirmation: the request still has to say
    // `apply` and `confirm`, and the line still names who confirmed it.
    // 状态是完整的，缺的是**动作**：第五轮评测的另一臂读懂了这台账的规矩，推出"确认属于人"，于是
    // 什么都没做——而它面对的问题恰恰是"台账没点名的路线该怎么办"。这里把那个动作点出来，不是给伪造
    // 确认发许可证：请求仍然必须说出 `apply` 与 `confirm`，那一行仍然点名是谁确认的。
    output.push_str(
        // Instantiated from this ledger's own state: when an entry has lapsed, the next call is the
        // one that asks about **that** anchor, named. The prose below stays for the other case —
        // a route the ledger does not carry at all.
        // 由这份台账自己的状态实例化：有条目失效时，下一次调用就是问**那一个** anchor 的调用，且点名它。
        // 下面那段散文留给另一种情形——台账里根本没有这条路线。
        &match &first_lapsed {
            Some(anchor) => format!(
                "next   `conformance {{anchor: \"{anchor}\"}}` says whether that lease still holds \
                 and where it lapsed; a route the ledger does not name is a **new anchor** — a first \
                 confirmation, not a renewal: pass `anchor`, `certifies`, `evidence`, `verifier`, \
                 `reason` and `files` with `apply: true` and `confirm: true`, and this tool appends \
                 one line whose fingerprint it computes from those files\n"
            ),
            None => "next   a route this ledger does not name is a **new anchor** — a first \
                     confirmation, not a renewal: pass `anchor`, `certifies`, `evidence`, \
                     `verifier`, `reason` and `files` together with `apply: true` and \
                     `confirm: true`, and this tool appends one line whose fingerprint it computes \
                     from those files\n"
                .to_owned(),
        },
    );
    Ok(output)
}

/// The append-only renewal: a preview unless the request says `apply` and `confirm`.
/// 只追加的续期：除非请求说出 `apply` 与 `confirm`，否则只是预览。
fn renew(root: &Path, arguments: &Value, ledger: &Path) -> Result<String, String> {
    let text = |key: &str| -> Result<String, String> {
        arguments
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .ok_or_else(|| format!("a renewal requires `{key}`"))
    };
    let anchor = text("anchor")?;
    let certifies = text("certifies")?;
    let evidence = text("evidence")?;
    let verifier = text("verifier")?;
    let reason = text("reason")?;
    let files = arguments
        .get("files")
        .and_then(Value::as_array)
        .ok_or_else(|| "a renewal requires `files`, the paths the decision rests on".to_owned())?
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if files.is_empty() {
        return Err("`files` must name at least one path".to_owned());
    }
    let current = read_files(root, &files)?;
    let fingerprint = adoption_fingerprint(&current);
    let entry = AdoptionEntry {
        anchor: anchor.clone(),
        certifies,
        evidence,
        verifier: verifier.clone(),
        at: wall_clock(),
        files: files.clone(),
        fingerprint: fingerprint.clone(),
        reason: reason.clone(),
    };
    let line = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}\n",
        entry.anchor,
        entry.certifies,
        entry.evidence,
        entry.verifier,
        entry.at,
        entry.files.join(","),
        entry.fingerprint,
        entry.reason,
    );
    let confirmed = arguments.get("apply").and_then(Value::as_bool) == Some(true)
        && arguments.get("confirm").and_then(Value::as_bool) == Some(true);
    if !confirmed {
        return Ok(format!(
            "preview: would append one adoption line to {}\n  {line}nothing was written; \
             `apply: true` and `confirm: true` write it, because a confirmation is a person's\n",
            ledger.display()
        ));
    }
    if let Some(parent) = ledger.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("{} is not creatable: {error}", parent.display()))?;
    }
    let existing = std::fs::read_to_string(ledger).unwrap_or_default();
    let mut next = existing;
    if !next.is_empty() && !next.ends_with('\n') {
        next.push('\n');
    }
    next.push_str(&line);
    std::fs::write(ledger, next)
        .map_err(|error| format!("{} is not writable: {error}", ledger.display()))?;
    Ok(format!(
        "appended one adoption line to {}\n  {line}confirmed by {verifier}; the ledger is \
         append-only, so this confirmation is one more line rather than a rewrite\n",
        ledger.display()
    ))
}

#[cfg(test)]
#[path = "adopted_tests.rs"]
mod adopted_tests;
