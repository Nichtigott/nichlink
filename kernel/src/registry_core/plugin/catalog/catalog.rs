//! Plugin lock records and catalog matching.
//! 插件锁记录与清单匹配。

use crate::registry_core::declaration::{PluginManifest, PluginMode, PluginSource};
use std::fmt;

use crate::registry_core::identity::IDENTITY_SCHEMA;

/// One parsed plugin lock line, kept as typed fields.
/// 一条解析后的插件锁记录，保留为带类型的字段。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginRecord {
    /// Trust lane the lock line declares.
    /// 锁记录声明的信任通道。
    pub source: PluginSource,
    /// Target framework this record was written for.
    /// 本记录针对的目标框架。
    pub framework: String,
    /// Package name of the plugin.
    /// 插件的包名。
    pub package: String,
    /// Plugin version exactly as the lock spelled it.
    /// 锁文件中原样写下的插件版本。
    pub version: String,
    /// Rust crate that carries the plugin implementation.
    /// 承载插件实现的 Rust crate。
    pub crate_name: String,
    /// Digest the plugin bytes must match.
    /// 插件字节必须匹配的摘要。
    pub checksum: String,
    /// Whether the plugin extends or replaces.
    /// 插件是扩展还是替换。
    pub mode: PluginMode,
    /// Recorded signature: `None` when the line carried no provenance column (the
    /// seven-field form) and `Some("")` when the ten-field form carried the column
    /// empty.
    /// 记录的签名：锁记录没带来源列（七字段形式）时为 `None`，十字段形式带了空列时为 `Some("")`。
    ///
    /// The two absences are deliberately different values. The ten-field spelling
    /// declares the provenance extension, so an empty column asserts "there is no
    /// such value" and the trust rule honours that; collapsing it back into `None`
    /// made a spelled-out declaration indistinguishable from a line that never
    /// mentioned the field, while every other empty column in this grammar is
    /// refused (`unknown plugin source`, `contains an empty field`) — audit `X-1` /
    /// `LGC-LG-40`.
    /// 两种"无"刻意是不同的值。十字段拼法声明了来源扩展，因此空列声明的是"此处没有值"，信任规则
    /// 会遵守它；把它塌缩回 `None`，会让一个写明了的声明与一条从未提到该字段的记录无从区分，而
    /// 这套语法里其它空列一律被拒（`unknown plugin source`、`contains an empty field`）——审计
    /// `X-1` / `LGC-LG-40`。
    pub signature: Option<String>,
    /// Recorded signing-key fingerprint; see [`PluginRecord::signature`] for how an
    /// empty ten-field column differs from the seven-field form.
    /// 记录的签名密钥指纹；空十字段列与七字段形式的区别见 [`PluginRecord::signature`]。
    pub public_key_fingerprint: Option<String>,
    /// Recorded revocation-list snapshot; see [`PluginRecord::signature`] for how an
    /// empty ten-field column differs from the seven-field form.
    /// 记录的撤销列表快照；空十字段列与七字段形式的区别见 [`PluginRecord::signature`]。
    pub revocation_list: Option<String>,
}

impl PluginRecord {
    /// The canonical lock line for this record.
    /// 本记录的规范锁行。
    ///
    /// Seven columns, or ten when any provenance column is declared: a record that declares
    /// provenance writes all three columns, spelling an absent one empty, because that is what
    /// "this line declares there is no such value" looks like ([`PluginRecord::signature`]). Two
    /// writers used to spell this line by hand — Studio's form and the bridge — which is how the
    /// seven-column and ten-column forms could drift apart from the parser that reads them.
    /// 七列；只要声明了任何一个来源列就是十列：声明了来源的记录三列都写，缺的那列拼成空，因为
    /// "这一行声明此处没有值"就长这样（见 [`PluginRecord::signature`]）。过去有两个写入方各自
    /// 手写这一行——Studio 的表单与桥——七列与十列两种形式正是在那里可能与读它们的解析器漂移。
    pub fn line(&self) -> String {
        let mut line = format!(
            "{}|{}|{}|{}|{}|{}|{}",
            self.source.text(),
            self.framework,
            self.package,
            self.version,
            self.crate_name,
            self.checksum,
            self.mode.text(),
        );
        let provenance = self.signature.is_some()
            || self.public_key_fingerprint.is_some()
            || self.revocation_list.is_some();
        if provenance {
            line.push_str(&format!(
                "|{}|{}|{}",
                self.signature.as_deref().unwrap_or_default(),
                self.public_key_fingerprint.as_deref().unwrap_or_default(),
                self.revocation_list.as_deref().unwrap_or_default(),
            ));
        }
        line
    }
}

/// Why a plugin lock was refused.
/// 插件锁被拒绝的原因。
///
/// The line is 1-based; it is `0` only for a failure that names no line.
/// 行号从 1 起；只有不指向某行的失败才为 `0`。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginLockError {
    /// 1-based line the failure points at; `0` when it names no line.
    /// 失败指向的行号，从 1 起；不指向某行时为 `0`。
    pub line: usize,
    /// Human-readable description of the failure.
    /// 失败的人类可读描述。
    pub message: String,
}

impl PluginLockError {
    /// A failure tied to one line of the lock.
    /// 与锁中某一行相关的失败。
    pub fn new(line: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            message: message.into(),
        }
    }
}

impl fmt::Display for PluginLockError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            write!(formatter, "{}", self.message)
        } else {
            write!(
                formatter,
                "plugin lock line {}: {}",
                self.line, self.message
            )
        }
    }
}

impl std::error::Error for PluginLockError {}

/// The historical string form, for callers that only render the failure.
/// 历史字符串形式，供只做渲染的调用方使用。
impl From<PluginLockError> for String {
    fn from(error: PluginLockError) -> Self {
        error.to_string()
    }
}

/// Parsed plugin selections kept separate from the Rust linking entries.
/// 与 Rust 链接入口分离的插件选择记录。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PluginCatalog {
    records: Vec<PluginRecord>,
    /// The lock line each record was read from, and the identity it was keyed by. A writer
    /// needs both to replace the line a promotion supersedes: leaving the old record in the
    /// file would put two records for one identity in a lock only a reader who knows the
    /// supersede rule can interpret.
    /// 每条记录读自哪一行，以及它据以做键的身份。写入方需要这两样才能替换掉被升级取代的那一行：把旧记录
    /// 留在文件里，等于让一份锁承载同一身份的两条记录，而只有懂得取代规则的读者才解释得了它。
    lines: Vec<usize>,
    keys: Vec<Identity>,
}

/// One record's identity: the seven fields `accounts_for` compares, with the digest reduced to
/// its body and lowercased.
/// 一条记录的身份：`accounts_for` 比较的那七个字段，摘要已归到本体并转小写。
type Identity = (
    PluginSource,
    String,
    String,
    String,
    String,
    String,
    PluginMode,
);

/// Whether a lock's declared schema is the identity schema this build reads.
/// 锁声明的 schema 是否就是本次构建所读的身份 schema。
///
/// `v{IDENTITY_SCHEMA}` is canonical; the bare version predates the prefix and stays readable
/// only so locks from earlier releases keep working.
/// `v{IDENTITY_SCHEMA}` 是规范写法；裸版本号早于该前缀，保留它只为让更早版本写下的锁继续可用。
///
/// TODO: drop the bare spelling once no supported lock predates the `v` prefix.
/// TODO: 等不再有早于 `v` 前缀的受支持锁文件时，删掉裸写法。
fn schema_matches(version: &str) -> bool {
    version.strip_prefix('v') == Some(IDENTITY_SCHEMA) || version == IDENTITY_SCHEMA
}

impl PluginCatalog {
    /// Parse a plugin lock, refusing unknown schemas and malformed or duplicate lines.
    /// 解析插件锁；未知 schema、格式错误或重复的记录一律拒绝。
    ///
    /// A ten-field line spells the provenance extension, so each of its three extra
    /// columns is read as written: an empty one becomes `Some("")` (declared empty)
    /// rather than being collapsed into the seven-field form's `None` (not
    /// mentioned). Nothing is silently dropped — audit `X-1` / `LGC-LG-40`.
    /// 十字段行写明了来源扩展，因此它多出的三列按原样读取：空列成为 `Some("")`（声明为空），而
    /// 不是被塌缩成七字段形式的 `None`（没提到）。没有任何东西被静默丢掉——审计 `X-1` /
    /// `LGC-LG-40`。
    pub fn parse_plugin_catalog(lock: &str) -> Result<Self, PluginLockError> {
        let mut records = Vec::new();
        let mut identities: std::collections::BTreeMap<_, usize> =
            std::collections::BTreeMap::new();
        // Kept beside the records so [`PluginCatalog::with_record`] can replace the exact line
        // a promotion supersedes.
        // 与记录并列留着，[`PluginCatalog::with_record`] 才能替换掉升级所取代的那一行。
        let mut lines: Vec<usize> = Vec::new();
        let mut keys: Vec<Identity> = Vec::new();
        // The gate belongs to the whole file, not to the line the parser reached: per-record
        // checking let an empty lock and a header written after the records read as "this
        // build's schema" (audit `LGC-LG-04`).
        // 门禁属于整份文件，而不是解析器走到的那一行：逐记录校验会让空锁与写在记录之后的表头都被
        // 读成"本次构建的 schema"（审计 `LGC-LG-04`）。
        let mut schema: Option<(usize, &str)> = None;
        for (line_number, line) in lock.lines().enumerate() {
            let Some(rest) = line.trim().strip_prefix("# nichlink-schema") else {
                continue;
            };
            // A misspelled header is refused, not read as a comment; `# nichlink-schema-related`
            // is prose. 拼错的表头被拒绝、不当注释读掉；`# nichlink-schema-related` 是散文。
            match rest
                .strip_prefix('=')
                .map(str::trim)
                .filter(|v| !v.is_empty())
            {
                Some(version) if schema.replace((line_number + 1, version)).is_none() => {}
                Some(_) => {
                    return Err(PluginLockError::new(
                        line_number + 1,
                        "duplicate schema header",
                    ));
                }
                None if rest.starts_with(char::is_whitespace) || rest.starts_with('=') => {
                    return Err(PluginLockError::new(line_number + 1, "bad schema header"));
                }
                None => {}
            }
        }
        if let Some((line_number, version)) = schema.filter(|(_, version)| !schema_matches(version))
        {
            return Err(PluginLockError::new(
                line_number,
                format!("uses identity schema {version}, expected v{IDENTITY_SCHEMA}"),
            ));
        }
        for (line_number, line) in lock.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if line.starts_with('#') {
                continue;
            }
            let fields = line.split('|').collect::<Vec<_>>();
            if fields.len() != 7 && fields.len() != 10 {
                return Err(PluginLockError::new(
                    line_number + 1,
                    "must contain 7 or 10 fields",
                ));
            }
            let source = PluginSource::parse_plugin_source(fields[0]).ok_or_else(|| {
                PluginLockError::new(
                    line_number + 1,
                    format!("unknown plugin source `{}`", fields[0]),
                )
            })?;
            let mode = PluginMode::parse_plugin_mode(fields[6]).ok_or_else(|| {
                PluginLockError::new(
                    line_number + 1,
                    format!("unknown plugin mode `{}`", fields[6]),
                )
            })?;
            if fields[1..6].iter().any(|field| field.is_empty()) {
                return Err(PluginLockError::new(
                    line_number + 1,
                    "contains an empty field",
                ));
            }
            let record = PluginRecord {
                source,
                framework: fields[1].to_owned(),
                package: fields[2].to_owned(),
                version: fields[3].to_owned(),
                crate_name: fields[4].to_owned(),
                checksum: fields[5].to_owned(),
                mode,
                signature: fields.get(7).map(ToString::to_string),
                public_key_fingerprint: fields.get(8).map(ToString::to_string),
                revocation_list: fields.get(9).map(ToString::to_string),
            };
            // The identity is the seven fields `accounts_for` compares, not the five that
            // name the package: the checksum (by meaning, `LGC-LG-40`) and the mode are part
            // of what makes two records describe the same plugin, and keying on five of the
            // seven made the parser refuse a pair every reader of this lock treats as two
            // identities. One identity has one record; a second record may replace the first
            // only by carrying provenance the first does not (`supersedes`), which is the
            // promotion an official write performs.
            // 身份是 `accounts_for` 比较的那七个字段，而不是点名包的五个：校验和（按含义，`LGC-LG-40`）
            // 与 mode 同样决定两条记录是否在描述同一个插件，而只按五个做键会让解析器拒绝一对在本锁的每个
            // 读取方看来都是两个身份的记录。一个身份只有一条记录；第二条只有在携带第一条没有的来源时才能
            // 取代它（`supersedes`）——那正是官方写入所做的那次升级。
            let identity = (
                record.source,
                record.framework.clone(),
                record.package.clone(),
                record.version.clone(),
                record.crate_name.clone(),
                checksum_body(&record.checksum).to_ascii_lowercase(),
                record.mode,
            );
            match identities.entry(identity.clone()) {
                std::collections::btree_map::Entry::Occupied(slot) => {
                    if !supersedes(&records[*slot.get()], &record) {
                        return Err(PluginLockError::new(
                            line_number + 1,
                            format!(
                                "duplicates package identity `{}` `{}`: a second record for one \
                                 identity must carry provenance the first does not",
                                record.package, record.version
                            ),
                        ));
                    }
                    let index = *slot.get();
                    records[index] = record;
                    lines[index] = line_number + 1;
                    keys[index] = identity;
                }
                std::collections::btree_map::Entry::Vacant(slot) => {
                    records.push(record);
                    lines.push(line_number + 1);
                    keys.push(identity);
                    slot.insert(records.len() - 1);
                }
            }
        }
        Ok(Self {
            records,
            lines,
            keys,
        })
    }

    /// The lock text that carries `line` — appended, or replacing the record it supersedes —
    /// or the parser's refusal of the text that would result.
    /// 承载 `line` 的锁文本——追加它，或让它替换掉被它取代的那条记录——或解析器对将产生文本的拒绝。
    ///
    /// A writer must not decide for itself what a legal lock is: appending a
    /// record the parser rejects produces an artifact the host refuses to read
    /// while the writer reports success (audit `LGC-LG-03`: a second record
    /// reusing the identity five-tuple, and a lock whose last line carried no
    /// terminator, both went through). The rule is [`PluginCatalog::parse`], and
    /// the separator is the one that parser reads — a lock not ending in a
    /// newline is completed rather than glued to the new record, because `a`
    /// followed by `b` written as `ab` is neither record.
    /// 写入方不得自行决定什么是合法锁：追加一条解析器会拒绝的记录，会产出一份宿主拒绝读、
    /// 而写入方却报成功的工件（审计 `LGC-LG-03`：复用身份五元组的第二条记录、以及末行没有
    /// 终止符的锁，两者都曾写进去）。规则就是 [`PluginCatalog::parse`]，分隔符就是它读的
    /// 那一个——不以换行结尾的锁先补一个换行，而不是与新记录粘在一起，因为把 `a` 之后接
    /// `b` 写成 `ab` 后两条都不是。
    ///
    /// The returned text is what the caller must write: validating one text and
    /// writing another is how a lock stops being the one that was checked.
    /// 交回的文本就是调用方必须写下的文本：校验一份、写另一份，是锁不再是被检查过的那一份的
    /// 开始。
    pub fn with_record(lock: &str, line: &str) -> Result<String, PluginLockError> {
        let before = Self::parse_plugin_catalog(lock)?;
        let mut candidate = String::with_capacity(lock.len() + line.len() + 2);
        candidate.push_str(lock);
        if !candidate.is_empty() && !candidate.ends_with('\n') {
            candidate.push('\n');
        }
        candidate.push_str(line);
        if !candidate.ends_with('\n') {
            candidate.push('\n');
        }
        let after = Self::parse_plugin_catalog(&candidate)?;
        // A record the new line superseded is dead, and it leaves the file: a lock carrying two
        // records for one identity is readable only by someone who knows the supersede rule,
        // and it contradicts this parser's own refusal message. The replacement is by line, so
        // comments and the order of every surviving record stay exactly as they were.
        // 被新行取代的记录已经死了，它要离开文件：承载同一身份两条记录的锁，只有懂得取代规则的人才读得懂，
        // 而且与这个解析器自己的拒绝话术相矛盾。替换按行进行，因此注释与每条存活记录的顺序原样不变。
        let dead: Vec<usize> = before
            .keys
            .iter()
            .enumerate()
            .filter_map(|(index, key)| {
                let position = after.keys.iter().position(|other| other == key)?;
                (after.lines[position] != before.lines[index]).then_some(before.lines[index])
            })
            .collect();
        if dead.is_empty() {
            return Ok(candidate);
        }
        let mut text = String::with_capacity(candidate.len());
        for (index, existing) in candidate.lines().enumerate() {
            if dead.contains(&(index + 1)) {
                continue;
            }
            text.push_str(existing);
            text.push('\n');
        }
        Ok(text)
    }

    /// The parsed records, in lock order.
    /// 解析出的记录，按锁文件中的顺序排列。
    pub fn records(&self) -> &[PluginRecord] {
        &self.records
    }

    /// Whether an identical record is already in the catalog.
    /// 目录中是否已存在一条完全相同的记录。
    pub fn contains(&self, candidate: &PluginRecord) -> bool {
        self.records.iter().any(|record| record == candidate)
    }

    /// Check an embedded manifest against the lock record for its source.
    /// 将嵌入注册面的 manifest 与对应来源锁文件中的记录比对。
    ///
    /// The seven identity fields must match exactly. The three provenance fields
    /// the parser accepts as an extension — signature, key fingerprint, and
    /// revocation-list snapshot — are *expectations*: a record that carries one
    /// pins it, and a record written without it (the seven-field form, which is
    /// what a host's own plugin UI writes) leaves it to the signature check.
    /// Comparing them for equality instead made the seven-field official record
    /// unsatisfiable in the only direction that matters: it can equal a manifest
    /// with no signature, and an official manifest with no signature is refused
    /// by every trust policy that has a trust root, so an official plugin could
    /// not be admitted end to end through a lock the host itself wrote.
    /// 七个身份字段必须完全一致。解析器作为扩展接受的三个来源字段——签名、密钥指纹与撤销列表
    /// 快照——是**期望**：记录携带某个值就把它钉住，记录没写（七字段形式，宿主自己的插件界面写下
    /// 的就是这种）就交给签名校验。此前用相等比较它们，会让七字段的官方记录在唯一要紧的方向上无法
    /// 满足：它只能等于一个没有签名的 manifest，而没有签名的官方 manifest 会被任何配置了信任根的
    /// 策略拒绝——于是官方插件根本无法经宿主自己写下的锁端到端准入。
    ///
    /// An empty ten-field column is the opposite direction of that fix: it is an
    /// expectation that the field has *no* value, so a manifest that names one is
    /// refused instead of being waved through as "not mentioned" (audit `X-1`).
    /// 空的十字段列是那次修复的反方向：它期望该字段*没有*值，因此点了值的 manifest 被拒绝，而不是
    /// 被当作"没提到"放行（审计 `X-1`）。
    pub fn contains_manifest(&self, manifest: PluginManifest) -> bool {
        self.contains_record(&PluginRecord {
            source: manifest.source,
            framework: manifest.framework.0.to_owned(),
            package: manifest.name.to_owned(),
            version: manifest.version.to_owned(),
            crate_name: manifest.crate_name.to_owned(),
            checksum: manifest.checksum.to_owned(),
            mode: manifest.mode,
            signature: manifest.signature.map(str::to_owned),
            public_key_fingerprint: manifest.public_key_fingerprint.map(str::to_owned),
            revocation_list: manifest.revocation_list.map(str::to_owned),
        })
    }

    /// Whether an existing record accounts for `candidate`, by the rule above.
    /// 已有记录是否覆盖了 `candidate`，用的是上面那条规则。
    ///
    /// A *record* is asked here, not a manifest, because one writer holds a record and no
    /// manifest: Studio's plugin UI. It used [`contains`](Self::contains), which asks for an
    /// identical record — all ten fields equal — and therefore refused a ten-field official
    /// write whenever the lock already carried the seven-field form, while the runtime
    /// accepts exactly that artifact. One rule, two spellings of the input, and the writer
    /// was on the wrong one (audit `PH-7`).
    /// 这里问的是**记录**而不是 manifest，因为有一个写入方手里只有记录、没有 manifest：Studio
    /// 的插件界面。它过去用 [`contains`](Self::contains)，那要求记录完全相同——十个字段全等——
    /// 于是在锁里已有七字段形式时拒绝一条十字段的官方写入，而运行期恰恰接受同一个工件。同一条规则、
    /// 两种输入拼法，而写入方站错了那一边（审计 `PH-7`）。
    pub fn contains_record(&self, candidate: &PluginRecord) -> bool {
        self.records
            .iter()
            .any(|record| accounts_for(record, candidate))
    }
}

/// Whether `record` accounts for `candidate`: the seven identity fields exactly, the three
/// provenance fields as the *record's* expectations.
/// `record` 是否覆盖了 `candidate`：七个身份字段严格相等，三个来源字段作为**记录侧**的期望。
///
/// The checksum is the one identity field compared by meaning rather than by spelling: the
/// `sha256:` prefix is optional (`PluginManifest::verify_bytes` and `is_sha256` both accept
/// either) and the hex is case-insensitive, so a lock written as `sha256:abc…` and a manifest
/// written as `abc…` name the same bytes. A literal comparison rejected that pair with
/// `LockMismatch` — "the lock does not have this plugin" — while every other reader of the same
/// digest would have accepted it (audit `LGC-LG-40`).
/// 校验和是唯一按含义而不是按拼法比较的身份字段：`sha256:` 前缀可选（`PluginManifest::verify_bytes`
/// 与 `is_sha256` 都接受两种拼法），十六进制也不区分大小写，因此锁里的 `sha256:abc…` 与清单里的
/// `abc…` 指的是同一段字节。字面比较会用 `LockMismatch`——"锁里没有这个插件"——拒绝这一对，而同一份
/// 摘要在其它每个读取点都会被接受（审计 `LGC-LG-40`）。
/// Whether `new` may replace `old` for one identity: its provenance columns are at least
/// the ones `old` carries, and at least one of them is one `old` did not carry.
/// `new` 是否可以取代同一身份的 `old`：它的来源列至少不比 `old` 少，且至少多出一列。
///
/// An empty column is *declared absent* — the ten-field spelling writes `|||` for provenance
/// it does not have — so it counts as not carried. A record that would drop a column is
/// refused rather than accepted as a replacement: silently losing a signature a lock pinned
/// is exactly the downgrade this rule exists to prevent, and a re-signing (the same columns
/// with different values) is a maintainer's edit to the lock, not an append.
/// 空列意为**声明此处没有值**——十字段拼法用它没有的来源写成 `|||`——因此算作未携带。会让某一列消失的
/// 记录被拒绝而不是接受为替代：悄悄丢掉锁曾钉住的签名，正是这条规则要防的降级；而重新签名（同样的列、
/// 不同的值）是维护者对锁的编辑，不是一次追加。
fn supersedes(old: &PluginRecord, new: &PluginRecord) -> bool {
    let carried = |value: Option<&str>| value.is_some_and(|text| !text.is_empty());
    let mut gained = false;
    for (old, new) in [
        (old.signature.as_deref(), new.signature.as_deref()),
        (
            old.public_key_fingerprint.as_deref(),
            new.public_key_fingerprint.as_deref(),
        ),
        (
            old.revocation_list.as_deref(),
            new.revocation_list.as_deref(),
        ),
    ] {
        match (carried(old), carried(new)) {
            (true, false) => return false,
            (false, true) => gained = true,
            _ => {}
        }
    }
    gained
}

fn accounts_for(record: &PluginRecord, candidate: &PluginRecord) -> bool {
    record.source == candidate.source
        && record.framework == candidate.framework
        && record.package == candidate.package
        && record.version == candidate.version
        && record.crate_name == candidate.crate_name
        && same_digest(&record.checksum, &candidate.checksum)
        && record.mode == candidate.mode
        && accounts_for_provenance(record.signature.as_deref(), candidate.signature.as_deref())
        && accounts_for_provenance(
            record.public_key_fingerprint.as_deref(),
            candidate.public_key_fingerprint.as_deref(),
        )
        && accounts_for_provenance(
            record.revocation_list.as_deref(),
            candidate.revocation_list.as_deref(),
        )
}

/// Whether two checksum spellings name the same digest.
/// 两种校验和拼法是否指同一个摘要。
///
/// One rule, read the same way the manifest reader reads it: drop an optional `sha256:`
/// prefix, then compare case-insensitively.
/// 一条规则，与清单读取器同解：去掉可选的 `sha256:` 前缀，再按大小写不敏感比较。
fn same_digest(left: &str, right: &str) -> bool {
    checksum_body(left).eq_ignore_ascii_case(checksum_body(right))
}

/// A checksum without its optional algorithm prefix.
/// 去掉可选算法前缀的校验和。
fn checksum_body(value: &str) -> &str {
    value.strip_prefix("sha256:").unwrap_or(value)
}

/// Whether one provenance field of a lock record covers the candidate's, by the
/// value's meaning rather than by its spelling.
/// 锁记录的某个来源字段是否覆盖候选记录的同名字段——按值的含义判定，而不是按拼法。
///
/// `None` is the seven-field form: the record never mentioned the field, so it pins
/// nothing and leaves it to the signature check. `Some("")` is the ten-field form
/// with that column empty: the record **declares** there is no value, so a candidate
/// that names one is refused and a candidate that names none is still accounted for.
/// A recorded value pins exactly that value. Reading the empty column as `None` is
/// what let a spelled-out "no signature" pass as "not mentioned" — audit `X-1` /
/// `LGC-LG-40`.
/// `None` 是七字段形式：记录从未提到该字段，因此不钉任何东西，交给签名校验。`Some("")` 是带有
/// 空列的十字段形式：记录**声明**此处没有值，因此点了值的候选被拒绝，而同样没点值的候选仍被覆盖。
/// 记录下的值则精确钉住那个值。把空列读成 `None`，正是让一个写明了的"没有签名"混作"没提到"的原因
/// ——审计 `X-1` / `LGC-LG-40`。
fn accounts_for_provenance(recorded: Option<&str>, candidate: Option<&str>) -> bool {
    match recorded {
        None => true,
        Some("") => candidate.is_none_or(str::is_empty),
        Some(recorded) => candidate == Some(recorded),
    }
}

#[cfg(test)]
#[path = "catalog_tests.rs"]
mod catalog_tests;
