//! Plugin lock records and catalog matching.
//! 插件锁记录与清单匹配。

use crate::registry_core::declaration::{PluginManifest, PluginMode, PluginSource};
use std::collections::BTreeSet;
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
    /// Recorded signature, absent when the lock line carried none.
    /// 记录的签名；锁记录未携带时为 None。
    pub signature: Option<String>,
    /// Recorded signing-key fingerprint, when present.
    /// 记录的签名密钥指纹；存在时才有值。
    pub public_key_fingerprint: Option<String>,
    /// Recorded revocation-list snapshot, when present.
    /// 记录的撤销列表快照；存在时才有值。
    pub revocation_list: Option<String>,
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
}

/// Whether a lock's declared schema is the identity schema this build reads.
/// 锁声明的 schema 是否就是本次构建所读的身份 schema。
///
/// The canonical spelling is `v{IDENTITY_SCHEMA}` — the same one the scope
/// environment variable uses. A bare version is the layout that predates the
/// `v` prefix; it stays readable so locks written by earlier releases keep
/// working, and it is accepted for no other reason.
/// 规范写法是 `v{IDENTITY_SCHEMA}`，与范围环境变量一致。裸版本号是加 `v` 前缀之前的
/// 版式；保留它只为让更早版本写下的锁继续可用，没有别的理由。
///
/// TODO: drop the bare spelling once no supported lock predates the `v` prefix.
/// TODO: 等不再有早于 `v` 前缀的受支持锁文件时，删掉裸写法。
fn schema_matches(version: &str) -> bool {
    version.strip_prefix('v') == Some(IDENTITY_SCHEMA) || version == IDENTITY_SCHEMA
}

impl PluginCatalog {
    /// Parse a plugin lock, refusing unknown schemas and malformed or duplicate lines.
    /// 解析插件锁；未知 schema、格式错误或重复的记录一律拒绝。
    pub fn parse(lock: &str) -> Result<Self, PluginLockError> {
        let mut records = Vec::new();
        let mut identities = BTreeSet::new();
        let mut schema = None;
        for (line_number, line) in lock.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(value) = line.strip_prefix("# nichlink-schema=") {
                schema = Some(value.trim());
                continue;
            }
            if line.starts_with('#') {
                continue;
            }
            if let Some(version) = schema
                && !schema_matches(version)
            {
                return Err(PluginLockError::new(
                    line_number + 1,
                    format!("uses identity schema {version}, expected v{IDENTITY_SCHEMA}"),
                ));
            }
            let fields = line.split('|').collect::<Vec<_>>();
            if fields.len() != 7 && fields.len() != 10 {
                return Err(PluginLockError::new(
                    line_number + 1,
                    "must contain 7 or 10 fields",
                ));
            }
            let source = PluginSource::parse(fields[0]).ok_or_else(|| {
                PluginLockError::new(
                    line_number + 1,
                    format!("unknown plugin source `{}`", fields[0]),
                )
            })?;
            let mode = PluginMode::parse(fields[6]).ok_or_else(|| {
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
                signature: fields
                    .get(7)
                    .filter(|value| !value.is_empty())
                    .map(ToString::to_string),
                public_key_fingerprint: fields
                    .get(8)
                    .filter(|value| !value.is_empty())
                    .map(ToString::to_string),
                revocation_list: fields
                    .get(9)
                    .filter(|value| !value.is_empty())
                    .map(ToString::to_string),
            };
            let identity = (
                record.source,
                record.framework.clone(),
                record.package.clone(),
                record.version.clone(),
                record.crate_name.clone(),
            );
            if !identities.insert(identity) {
                return Err(PluginLockError::new(
                    line_number + 1,
                    format!(
                        "duplicates package identity `{}` `{}`",
                        record.package, record.version
                    ),
                ));
            }
            records.push(record);
        }
        Ok(Self { records })
    }

    /// The lock text that carries `line` as one more record, or the parser's
    /// refusal of the text that would result.
    /// 把 `line` 作为又一条记录承载的锁文本，或解析器对将产生文本的拒绝。
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
    pub fn with_appended_line(lock: &str, line: &str) -> Result<String, PluginLockError> {
        let mut candidate = String::with_capacity(lock.len() + line.len() + 2);
        candidate.push_str(lock);
        if !candidate.is_empty() && !candidate.ends_with('\n') {
            candidate.push('\n');
        }
        candidate.push_str(line);
        if !candidate.ends_with('\n') {
            candidate.push('\n');
        }
        Self::parse(&candidate)?;
        Ok(candidate)
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
fn accounts_for(record: &PluginRecord, candidate: &PluginRecord) -> bool {
    record.source == candidate.source
        && record.framework == candidate.framework
        && record.package == candidate.package
        && record.version == candidate.version
        && record.crate_name == candidate.crate_name
        && record.checksum == candidate.checksum
        && record.mode == candidate.mode
        && record
            .signature
            .as_deref()
            .is_none_or(|recorded| Some(recorded) == candidate.signature.as_deref())
        && record
            .public_key_fingerprint
            .as_deref()
            .is_none_or(|recorded| Some(recorded) == candidate.public_key_fingerprint.as_deref())
        && record
            .revocation_list
            .as_deref()
            .is_none_or(|recorded| Some(recorded) == candidate.revocation_list.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_lock_parser_keeps_official_and_user_records_typed() {
        let catalog = PluginCatalog::parse(
            "# source|framework|package|version|crate|checksum|mode|signature|key|revocations\n\
             official|com.nichui.editor|canvas|1.0.0|canvas|sha256:a|replacement|sig-v1|key-v1|official-2026\n\
             user|com.nichui.editor|local|0.1.0|local_canvas|sha256:b|extension\n",
        )
        .expect("lock should parse");
        assert_eq!(catalog.records().len(), 2);
        assert_eq!(catalog.records()[0].source, PluginSource::Official);
        assert_eq!(catalog.records()[0].signature.as_deref(), Some("sig-v1"));
        assert_eq!(
            catalog.records()[0].revocation_list.as_deref(),
            Some("official-2026")
        );
        assert_eq!(catalog.records()[1].mode, PluginMode::Extension);
    }

    #[test]
    fn plugin_lock_parser_rejects_ambiguous_duplicate_identity() {
        let lock = "official|com.nichui.editor|canvas|1.0.0|canvas|sha256:a|extension\n\
                    official|com.nichui.editor|canvas|1.0.0|canvas|sha256:b|extension\n";
        let error = PluginCatalog::parse(lock).unwrap_err().to_string();
        assert!(error.contains("line 2"));
        assert!(error.contains("duplicates package identity"));
    }

    /// A writer asks the parser before it writes: a second record reusing the
    /// identity five-tuple is refused with the parser's own reason, and a record
    /// the parser accepts comes back as the text to write.
    /// 写入方在写之前先问解析器：复用身份五元组的第二条记录被解析器自己的理由拒绝，而解析器
    /// 接受的记录作为待写文本交回。
    #[test]
    fn appending_a_duplicate_identity_is_refused_by_the_parser() {
        let seed = "user|com.nichui.editor|canvas|1.0.0|canvas|sha256:a|extension\n";
        let duplicate = "user|com.nichui.editor|canvas|1.0.0|canvas|sha256:b|extension";
        let error = PluginCatalog::with_appended_line(seed, duplicate)
            .expect_err("a duplicate identity must not be appended");
        assert!(
            error.to_string().contains("duplicates package identity"),
            "{error}"
        );

        let fresh = "user|com.nichui.editor|panel|1.0.0|panel|sha256:b|extension";
        let text = PluginCatalog::with_appended_line(seed, fresh).expect("a fresh record appends");
        assert_eq!(
            PluginCatalog::parse(&text)
                .expect("the text handed back parses")
                .records()
                .len(),
            2
        );
    }

    /// The separator belongs to the parser, not to the writer: a lock whose last
    /// line carries no terminator gains one instead of gluing two records into a
    /// line that is neither.
    /// 分隔符属于解析器而不属于写入方：末行没有终止符的锁会补上一个换行，而不是把两条记录粘成
    /// 一条两者都不是的行。
    #[test]
    fn appending_completes_a_missing_line_terminator() {
        let seed = "user|com.nichui.editor|canvas|1.0.0|canvas|sha256:a|extension";
        let line = "user|com.nichui.editor|panel|1.0.0|panel|sha256:b|extension";
        let text = PluginCatalog::with_appended_line(seed, line).expect("the text parses");
        assert!(text.ends_with('\n'), "{text:?}");
        assert_eq!(
            PluginCatalog::parse(&text)
                .expect("two records")
                .records()
                .len(),
            2
        );
    }

    /// The seven-field record leaves the three provenance fields to the
    /// signature check, and a record that carries one pins it.
    /// 七字段记录把三个来源字段交给签名校验；携带某个值的记录则把它钉住。
    fn manifest() -> PluginManifest {
        PluginManifest {
            name: "canvas",
            crate_name: "canvas",
            version: "1.0.0",
            framework: crate::FrameworkId::new("com.nichui.editor"),
            source: PluginSource::Official,
            mode: PluginMode::Extension,
            checksum: "sha256:a",
            signature: Some("sig-v1"),
            public_key_fingerprint: Some("key-v1"),
            revocation_list: Some("official-2026"),
        }
    }

    #[test]
    fn a_ten_field_record_accounts_for_a_seven_field_lock() {
        let bare = PluginCatalog::parse(
            "official|com.nichui.editor|canvas|1.0.0|canvas|sha256:a|extension\n",
        )
        .expect("a seven-field lock parses");
        let pinned = PluginCatalog::parse(
            "official|com.nichui.editor|canvas|1.0.0|canvas|sha256:a|extension|sig-v1|key-v1|official-2026\n",
        )
        .expect("a ten-field lock parses");
        let candidate = pinned.records()[0].clone();

        assert!(
            !bare.contains(&candidate),
            "an identical-record rule refuses the write — the old writer's answer"
        );
        assert!(
            bare.contains_record(&candidate),
            "the runtime's rule, asked about the same candidate record, accepts it"
        );
        assert!(
            bare.contains_manifest(manifest()),
            "and the manifest spelling of the same plugin agrees"
        );

        // The expectation still binds in the other direction: a lock that pins a signature does
        // not account for a record written without it.
        // 期望在另一个方向上仍然生效：钉住了签名的锁，不覆盖一条没写签名的记录。
        let other = PluginCatalog::parse(
            "official|com.nichui.editor|canvas|1.0.0|canvas|sha256:a|extension|sig-v2|key-v1|official-2026\n",
        )
        .expect("a ten-field lock parses");
        assert!(
            !other.contains_record(&bare.records()[0].clone()),
            "a record the lock pins differently must still be refused"
        );
    }

    #[test]
    fn a_record_without_provenance_fields_does_not_pin_them() {
        let bare = PluginCatalog::parse(
            "official|com.nichui.editor|canvas|1.0.0|canvas|sha256:a|extension\n",
        )
        .expect("a seven-field lock parses");
        assert!(
            bare.contains_manifest(manifest()),
            "a record the host's own UI wrote must match a signed official manifest"
        );

        let pinned = PluginCatalog::parse(
            "official|com.nichui.editor|canvas|1.0.0|canvas|sha256:a|extension|sig-v1|key-v1|official-2026\n",
        )
        .expect("a ten-field lock parses");
        assert!(pinned.contains_manifest(manifest()));

        let other = PluginCatalog::parse(
            "official|com.nichui.editor|canvas|1.0.0|canvas|sha256:a|extension|sig-v2|key-v1|official-2026\n",
        )
        .expect("a ten-field lock parses");
        assert!(
            !other.contains_manifest(manifest()),
            "a record that names a signature must pin it"
        );
    }

    #[test]
    fn plugin_lock_parser_rejects_unknown_identity_schema() {
        let error = PluginCatalog::parse(
            "# nichlink-schema=2\n\
             user|com.nichui.editor|local|0.1.0|local_canvas|sha256:b|extension\n",
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("identity schema 2"));
        assert!(error.contains("expected v3"));
    }

    /// The canonical spelling is `v3`; the bare `3` written before the `v`
    /// prefix existed stays readable, and nothing else does.
    /// 规范写法是 `v3`；加 `v` 前缀之前写下的裸 `3` 仍可读，别的写法都不行。
    #[test]
    fn plugin_lock_parser_accepts_canonical_and_legacy_schema_spellings() {
        for schema in ["v3", "3"] {
            let lock = format!(
                "# nichlink-schema={schema}\n\
                 user|com.nichui.editor|local|0.1.0|local_canvas|sha256:b|extension\n"
            );
            assert!(
                PluginCatalog::parse(&lock).is_ok(),
                "schema `{schema}` must stay readable"
            );
        }
        for schema in ["v4", "V3", "vv3", ""] {
            let lock = format!(
                "# nichlink-schema={schema}\n\
                 user|com.nichui.editor|local|0.1.0|local_canvas|sha256:b|extension\n"
            );
            assert!(
                PluginCatalog::parse(&lock).is_err(),
                "schema `{schema}` must be refused"
            );
        }
    }
}
