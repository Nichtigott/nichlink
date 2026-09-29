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
    AdoptionEntry, AdoptionVerdict, adoption_fingerprint, parse_adoption, verdict_of,
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
fn read_files(root: &Path, files: &[String]) -> Result<Vec<(String, String)>, String> {
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
    let mut output = String::from("evidence: adoption ledger (provisional by construction)\n");
    let (mut provisional, mut lapsed) = (0usize, 0usize);
    for entry in &entries {
        let current = read_files(root, &entry.files)?;
        match verdict_of(entry, &current) {
            AdoptionVerdict::Provisional => {
                provisional += 1;
                output.push_str(&format!(
                    "adopted since {} at {} (provisional) — {}: {} [evidence: {}; confirmed by {}; \
                     why: {}]\n",
                    entry.at,
                    entry.fingerprint,
                    entry.anchor,
                    entry.certifies,
                    entry.evidence,
                    entry.verifier,
                    entry.reason,
                ));
            }
            AdoptionVerdict::Lapsed { file } => {
                lapsed += 1;
                output.push_str(&format!(
                    "adoption lapsed at {file}; needs confirmation — {}: {} [adopted at {} by {}; \
                     why: {}]\n",
                    entry.anchor, entry.certifies, entry.at, entry.verifier, entry.reason,
                ));
            }
        }
    }
    output.push_str(&format!("provisional {provisional}  lapsed {lapsed}\n"));
    output.push_str(
        "note: nothing here renews or revokes an adoption — a lapsed one needs a person to \
         confirm the new state, and a confirmation is one more line.\n",
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
