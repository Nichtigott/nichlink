//! Plugin admission: write one record into this package's plugin lock.
//! 插件准入：把一条记录写进本包的插件锁。
//!
//! Studio's plugin form (`submit_plugin`) is the other writer, and this is the same
//! path: `source` selects `official.lock` or `user.lock` under `.nichlink/plugins/`,
//! the seven fields spell one record line, an `official` record is admitted only when
//! the kernel's `PluginCatalog::contains_record` says the lock already accounts for
//! that package identity, and the lock's own parser decides whether the append is
//! legal — before anything is written. The entry file that imports the crate
//! (`official.rs` / `user.rs`) carries the other half of the same decision, so a failed
//! lock write puts the entry back, exactly as Studio's writer does.
//! Studio 的插件表单（`submit_plugin`）是另一个写入方，而这里走同一条路径：`source` 选择
//! `.nichlink/plugins/` 下的 `official.lock` 或 `user.lock`，七个字段拼出一条记录行，`official`
//! 记录只有在核内的 `PluginCatalog::contains_record` 说锁已经覆盖那个包身份时才被准入，而这次追加是否
//! 合法由锁自己的解析器决定——在写下任何东西之前。导入该 crate 的入口文件（`official.rs` /
//! `user.rs`）承载同一个决定的另一半，因此锁写入失败时会把入口放回去，与 Studio 的写入方完全相同。
//!
//! **Preview is the default, and it is the write.** The preview computes the exact bytes
//! with the same kernel calls the write uses — `parse_plugin_catalog`,
//! `contains_record`, `with_appended_line` — and prints them, so there is no copy to
//! drift from: the text a preview shows is the text an apply stores. Nothing is read
//! into the preview that the apply does not read, and nothing is written either way
//! until `apply: true` **and** `confirm: true` are both in the request, because the lock
//! is the artifact the host admits plugins from.
//! **预览是默认，而且它就是那次写入。** 预览用与写入相同的核内调用（`parse_plugin_catalog`、
//! `contains_record`、`with_appended_line`）算出确切字节并打印出来，因此没有副本可以漂移：预览显示的
//! 文本就是落盘存下的文本。预览读的东西与落盘读的完全相同，而两者都不会在请求同时带上 `apply: true`
//! **与** `confirm: true` 之前写下任何字节，因为锁正是宿主据以准入插件的工件。
//!
//! A virtual workspace root names no package, so this write is refused there with the
//! candidate member directories: the lock lives under one package's root, and no request
//! field names which one.
//! 虚拟工作区根不点名任何包，因此这次写入在那里被拒绝、并列出候选成员目录：锁住在某一个包根之下，而
//! 请求里没有任何字段点名是哪一个。

use std::path::{Path, PathBuf};

use nichlink_kernel::plugin::catalog::{PluginCatalog, PluginRecord};
use nichlink_kernel::{PluginMode, PluginSource};
use serde_json::Value;

/// The directory under a package root that holds the plugin locks.
/// 包根之下存放插件锁的目录。
///
/// The literal `plugin_host::PLUGIN_LOCK_DIRECTORY` names and Studio's `submit_plugin`
/// joins. That constant is not imported because `plugin_host` is gated behind the
/// `plugins` feature, which this bridge's own feature set (`mcp`) does not include, and
/// a bridge that widened its features to read one string would drag a signature backend
/// in with it. The name is the contract either way: the host reads the locks from here.
/// 它就是 `plugin_host::PLUGIN_LOCK_DIRECTORY` 命名、Studio 的 `submit_plugin` 拼接的那个
/// 字面量。之所以不导入那个常量，是因为 `plugin_host` 门控在 `plugins` 特性之后，而本桥自己的特性集
/// （`mcp`）不含它；为读一个字符串而放宽特性会把签名后端一起拖进来。无论如何，这个名字就是契约：宿主
/// 正是从这里读那两份锁。
const PLUGIN_DIRECTORY: &str = ".nichlink/plugins";

/// Run one `nichlink.plugin` request, previewing unless `apply` and `confirm` are true.
/// 执行一次 `nichlink.plugin` 请求；除非 `apply` 与 `confirm` 都为真，否则只预览。
pub(crate) fn plugin(root: &Path, arguments: &Value) -> Result<String, String> {
    let request = Request::parse_request(arguments)?;
    let apply = arguments
        .get("apply")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    // The request says `confirm` itself. A plugin lock is the artifact the host admits
    // plugins from, and the same decision is written into two files, so the one caller
    // who can skip the preview by accident is answered with a refusal rather than a write
    // — the shape `apply delete` uses, for the same reason.
    // 请求自己说出 `confirm`。插件锁是宿主据以准入插件的工件，而同一个决定被写进两个文件，因此那个可能
    // 不小心跳过预览的调用方得到的是一次拒绝而不是一次写入——与 `apply delete` 同一种形状、同一个理由。
    if apply && arguments.get("confirm").and_then(Value::as_bool) != Some(true) {
        return Err(format!(
            "nichlink.plugin requires `confirm: true` to write {}: the lock is the artifact the \
             host admits plugins from, and the same decision also changes the entry file that \
             imports the crate, so the request says it rather than the bridge assuming it. \
             Nothing was written",
            request.lock_name
        ));
    }
    match plan(root, &request)? {
        Plan::Already { lock } => Ok(format!(
            "already selected: {} already carries the record `{}`; no file was written\n",
            lock.display(),
            request.line
        )),
        plan @ Plan::Write { .. } if !apply => Ok(plan.preview(root, &request)),
        plan @ Plan::Write { .. } => plan.write_plan(&request),
    }
}

/// One validated `nichlink.plugin` request: the record, and the line the lock stores.
/// 一次已校验的 `nichlink.plugin` 请求：那条记录，以及锁存下的那一行。
struct Request {
    record: PluginRecord,
    /// `<source>|<framework>|…|<mode>`, exactly the seven-field spelling the lock reads.
    /// `<source>|<framework>|…|<mode>`，正是锁读取的七字段拼法。
    line: String,
    /// `official.lock` or `user.lock`.
    /// `official.lock` 或 `user.lock`。
    lock_name: &'static str,
    /// `official.rs` or `user.rs`, the entry file that imports the crate.
    /// `official.rs` 或 `user.rs`，即导入该 crate 的入口文件。
    entry_name: &'static str,
    /// The crate name, kept for the entry file's import line.
    /// crate 名，供入口文件的导入行使用。
    crate_name: String,
}

impl Request {
    /// Validate the request's seven fields, which are Studio's form's seven rows.
    /// 校验请求的七个字段，也就是 Studio 表单的七行。
    fn parse_request(arguments: &Value) -> Result<Self, String> {
        let source_text = text(arguments, "source")?;
        let source = PluginSource::parse_plugin_source(&source_text)
            .ok_or_else(|| format!("`source` must be `official` or `user`, not `{source_text}`"))?;
        let mode_text = text(arguments, "mode")?;
        let mode = PluginMode::parse_plugin_mode(&mode_text).ok_or_else(|| {
            format!("`mode` must be `extension` or `replacement`, not `{mode_text}`")
        })?;
        let framework = text(arguments, "framework")?;
        let package = text(arguments, "package")?;
        let version = text(arguments, "version")?;
        let crate_name = text(arguments, "crate")?;
        let checksum = text(arguments, "checksum")?;
        // The entry file writes `use <crate> as _;`, so the crate has to be one a Rust
        // `use` can name. Studio's writer asks the same question of the same string.
        // 入口文件写的是 `use <crate> as _;`，因此这个 crate 必须是 Rust 的 `use` 能命名的。
        // Studio 的写入方对同一个字符串问同一个问题。
        if !crate_name
            .chars()
            .all(|character| character == '_' || character.is_ascii_alphanumeric())
        {
            return Err(format!(
                "`crate` must be a Rust identifier, not `{crate_name}`"
            ));
        }
        let line = format!(
            "{source_text}|{framework}|{package}|{version}|{crate_name}|{checksum}|{mode_text}"
        );
        let record = PluginRecord {
            source,
            framework,
            package,
            version,
            crate_name: crate_name.clone(),
            checksum,
            mode,
            // The bridge writes Studio's seven-field spelling. The ten-field form's three
            // provenance columns are `Some("")` ("declared absent") rather than `None`
            // ("not mentioned"), and no field of this request declares either.
            // 桥写的是 Studio 的七字段拼法。十字段形式的那三个来源列是 `Some("")`
            // （"声明此处没有值"）而不是 `None`（"没提到"），而本次请求没有任何字段声明其中任何一个。
            signature: None,
            public_key_fingerprint: None,
            revocation_list: None,
        };
        let (lock_name, entry_name) = match source {
            PluginSource::Official => ("official.lock", "official.rs"),
            PluginSource::User => ("user.lock", "user.rs"),
        };
        Ok(Self {
            record,
            line,
            lock_name,
            entry_name,
            crate_name,
        })
    }

    /// The line the entry file must hold for the linker to see the crate.
    /// 入口文件必须持有的那一行，链接器才会看见这个 crate。
    fn entry_line(&self) -> String {
        format!("use {} as _;", self.crate_name)
    }

    /// What an entry file gains when it does not hold [`Request::entry_line`] yet.
    /// 当入口文件还没有 [`Request::entry_line`] 时它会得到的东西。
    fn anchor(&self) -> String {
        format!("\n#[allow(unused_imports)]\n{}\n", self.entry_line())
    }
}

/// What the write would do, decided before anything is written.
/// 这次写入会做什么——在写下任何东西之前就已决定。
enum Plan {
    /// The lock already carries this exact line, so there is nothing to write.
    /// 锁已经载有这完全一样的一行，因此没有东西要写。
    Already {
        /// The lock file that already carries it.
        /// 已经载有它的那份锁文件。
        lock: PathBuf,
    },
    /// The bytes an apply stores, computed by the kernel's own calls.
    /// 落盘存下的字节，由核内自己的调用算出。
    Write {
        /// The lock file, and its text before and after.
        /// 锁文件，以及它写入前与写入后的文本。
        lock: PathBuf,
        lock_before: String,
        lock_after: String,
        /// The entry file, and the text it gains when it needs one.
        /// 入口文件，以及它需要时得到的那段文本。
        entry: PathBuf,
        entry_before: String,
        entry_after: Option<String>,
        /// The plugin directory, and whether this write creates it.
        /// 插件目录，以及这次写入是否创建它。
        directory: PathBuf,
        directory_existed: bool,
        /// Whether the `contains_record` gate admitted the record (official only).
        /// `contains_record` 闸门是否放行了这条记录（仅官方）。
        gated: bool,
    },
}

/// Decide what the request would write, by reading this package's lock.
/// 通过读取本包的锁，决定这次请求会写什么。
fn plan(root: &Path, request: &Request) -> Result<Plan, String> {
    let directory = root.join(PLUGIN_DIRECTORY);
    let lock = directory.join(request.lock_name);
    let before = std::fs::read_to_string(&lock).unwrap_or_default();
    let catalog = PluginCatalog::parse_plugin_catalog(&before)
        .map_err(|error| format!("{} is not a readable lock: {error}", lock.display()))?;
    // An official record is admitted only when the lock already accounts for that package
    // identity: the trust rule is a property of the lock the host reads, and the kernel is
    // its authority. A seven-field lock record accounts for a candidate whose provenance
    // columns it never mentioned (`accounts_for`), which is why this is not an
    // identical-record test.
    // 官方记录只有在锁已经覆盖那个包身份时才被准入：信任规则是宿主所读那份锁的属性，而核内是它的权威。
    // 一条七字段的锁记录会覆盖一个从未提到来源列的候选（`accounts_for`），这正是它不是"记录完全相同"的
    // 判定的原因。
    let gated = if request.record.source == PluginSource::Official {
        if !catalog.contains_record(&request.record) {
            return Err(format!(
                "REFUSED: `{}` is not present in the trusted lock {}; an official record is \
                 admitted only when the lock already accounts for that package identity (kernel \
                 `PluginCatalog::contains_record`, the rule the runtime reads). Nothing was written",
                request.record.package,
                lock.display()
            ));
        }
        true
    } else {
        false
    };
    if before.lines().any(|line| line.trim() == request.line) {
        return Ok(Plan::Already { lock });
    }
    // The lock is the artifact the host reads, so the parser decides whether this append is
    // legal — before anything is written — and the text that comes back is the text an
    // apply stores, so what was validated is what lands.
    // 锁是宿主读取的工件，因此这次追加是否合法由解析器决定——在写下任何东西之前——而交回的文本就是落盘
    // 存下的文本，因此被校验的就是被存下的。
    let line = format!("{}\n", request.line);
    let after = PluginCatalog::with_appended_line(&before, &line).map_err(|error| {
        format!(
            "REFUSED: {} would not parse with this record: {error}. Nothing was written",
            request.lock_name
        )
    })?;
    let entry = directory.join(request.entry_name);
    let entry_before = std::fs::read_to_string(&entry).unwrap_or_default();
    let imported = entry_before
        .lines()
        .any(|line| line.trim() == request.entry_line());
    let entry_after = (!imported).then(|| format!("{entry_before}{}", request.anchor()));
    Ok(Plan::Write {
        lock,
        lock_before: before,
        lock_after: after,
        entry,
        entry_before,
        entry_after,
        directory_existed: directory.is_dir(),
        directory,
        gated,
    })
}

impl Plan {
    /// The write, at the paths and with the bytes the plan decided.
    /// 按计划决定好的路径与字节执行写入。
    fn write_plan(&self, request: &Request) -> Result<String, String> {
        let Plan::Write {
            lock,
            lock_after,
            entry,
            entry_before,
            entry_after,
            directory,
            directory_existed,
            gated,
            ..
        } = self
        else {
            return Err("already selected: nothing to write".to_owned());
        };
        if !*directory_existed {
            std::fs::create_dir_all(directory)
                .map_err(|error| format!("cannot create {}: {error}", directory.display()))?;
        }
        let entry_existed = entry.is_file();
        // The entry file goes first and the lock second, which is Studio's order: the lock
        // is the artifact the runtime reads, so it is the last thing to change.
        // 入口文件先写、锁后写，这是 Studio 的顺序：锁是运行期读取的工件，因此它是最后改变的东西。
        if let Some(after) = entry_after {
            std::fs::write(entry, after)
                .map_err(|error| format!("cannot update {}: {error}", entry.display()))?;
        }
        if let Err(error) = std::fs::write(lock, lock_after) {
            // One decision in two files: a half-written pair would leave the entry importing a
            // crate the lock does not record, so the entry goes back to its previous bytes —
            // or away, when this write created it — before the error is reported.
            // 一个决定由两个文件承载：写了一半会让入口导入一个锁里没有记录的 crate，因此在报告错误之前把
            // 入口恢复成先前的字节——若这次写入创建了它，则移除它。
            let restored = if entry_existed {
                std::fs::write(entry, entry_before)
            } else {
                std::fs::remove_file(entry)
            };
            let note = if restored.is_ok() {
                "the entry file was restored"
            } else {
                "the entry file could not be restored"
            };
            return Err(format!(
                "cannot update {}: {error}; {note}. Nothing else was written",
                lock.display()
            ));
        }
        let mut report = format!(
            "applied: wrote one {} record to {}\n",
            source_name(request.record.source),
            lock.display()
        );
        report.push_str(&gate_line(*gated));
        report.push_str(&format!("wrote {}\n", lock.display()));
        if entry_after.is_some() {
            report.push_str(&format!("wrote {}\n", entry.display()));
        }
        if !*directory_existed {
            report.push_str(&format!("created {}\n", directory.display()));
        }
        report.push_str(&format!("record: {}\n", request.line));
        Ok(report)
    }

    /// What the write would gain each file, with the exact bytes, and nothing written.
    /// 这次写入会让每个文件得到什么——给出确切字节，而什么都不写。
    fn preview(&self, root: &Path, request: &Request) -> String {
        let Plan::Write {
            lock,
            lock_before,
            lock_after,
            entry,
            entry_before,
            entry_after,
            directory,
            directory_existed,
            gated,
            ..
        } = self
        else {
            return "already selected: nothing to write\n".to_owned();
        };
        let mut report = format!(
            "preview: nichlink.plugin would write one {} record; nothing was written\n",
            source_name(request.record.source)
        );
        report.push_str(&format!("root {}\n", root.display()));
        report.push_str(&format!("record: {}\n", request.line));
        report.push_str(&gate_line(*gated));
        if !*directory_existed {
            report.push_str(&format!("+ {}\n", directory.display()));
        }
        report.push_str(&format!("~ {}\n", lock.display()));
        report.push_str(&appended(lock_before, lock_after));
        if let Some(after) = entry_after {
            report.push_str(&format!("~ {}\n", entry.display()));
            report.push_str(&appended(entry_before, after));
        } else {
            report.push_str(&format!(
                "{} already holds `{}`; it is left alone\n",
                entry.display(),
                request.entry_line()
            ));
        }
        report.push_str(
            "these are the bytes `apply: true` (with `confirm: true`) stores; the preceding \
             bytes of each file are unchanged\n",
        );
        report
    }
}

/// How the official gate answered, or that it does not apply.
/// 官方闸门给出的答案，或它并不适用。
fn gate_line(gated: bool) -> String {
    if gated {
        "gate: the official package identity is accounted for by the lock (kernel \
         `contains_record`), so the runtime's trust rule admits this record\n"
            .to_owned()
    } else {
        String::new()
    }
}

/// The lane's name, as the lock spells it.
/// 通道的名字，按锁的拼法。
fn source_name(source: PluginSource) -> &'static str {
    match source {
        PluginSource::Official => "official",
        PluginSource::User => "user",
    }
}

/// The lines an append adds, as `+` lines, with the one non-line change named.
/// 一次追加新增的那些行，写作 `+` 行，并把唯一不属于"行"的那种变化说出来。
///
/// Both writers here are appends, so the change is a suffix and the diff is exact by
/// construction: the common prefix is `before` itself, and what is shown as added is
/// exactly what `after` has past it. A lock whose last line carried no terminator gains
/// one first, and that byte is named rather than printed as an empty line, which would
/// read as an added blank line.
/// 这里的两个写入方都是追加，因此变化就是一段后缀，而 diff 在构造上就是精确的：公共前缀就是 `before`
/// 本身，显示为新增的部分正是 `after` 越过它的那一截。末行没有终止符的锁会先补一个，而那个字节被点名，
/// 而不是被打印成一个空行——那读起来会像新增了一个空行。
fn appended(before: &str, after: &str) -> String {
    let mut output = String::new();
    let mut added = after.strip_prefix(before).unwrap_or(after);
    if !before.is_empty()
        && !before.ends_with('\n')
        && let Some(rest) = added.strip_prefix('\n')
    {
        output.push_str("+ (adds the missing final newline)\n");
        added = rest;
    }
    for line in added.lines() {
        output.push_str(&format!("+{line}\n"));
    }
    output
}

/// The one required string argument named `key`.
/// 名为 `key` 的那一个必填字符串参数。
fn text(arguments: &Value, key: &str) -> Result<String, String> {
    match arguments.get(key).and_then(Value::as_str) {
        Some(value) if !value.trim().is_empty() => Ok(value.trim().to_owned()),
        Some(_) => Err(format!("`{key}` must not be empty")),
        None => Err(format!("nichlink.plugin requires `{key}`")),
    }
}

#[cfg(test)]
#[path = "plugin_tests.rs"]
mod plugin_tests;
