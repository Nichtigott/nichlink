//! The persisted `graft.plan` document.
//! 持久化的 `graft.plan` 文档。
//!
//! A plan file records one external overlay declaration: which logical slot a
//! host hands over, which external implementation is meant to take it, and
//! whether the whole subtree or only the node is replaced. It is a declaration,
//! not an application: the runtime record resolver
//! ([`Registry::overlay_recorded`](crate::Registry::overlay_recorded)) is what
//! turns it into an overlay cut, and this document never touches the base tree
//! or the external tree.
//! 计划文件记录一条外部覆盖声明：宿主交出哪个逻辑槽位、打算由哪个外部实现接管、
//! 是整棵子树替换还是只替换节点。它是声明而不是应用：把它变成覆盖切口的是运行期记录
//! 解析器（[`Registry::overlay_recorded`](crate::Registry::overlay_recorded)），本文档
//! 永不改动原树或外部树。
//!
//! The format is versioned so a reader can refuse a document it does not
//! understand instead of guessing at it.
//! 格式带版本，读取方因此可以拒绝读不懂的文档，而不是猜。

use std::fmt;

use crate::registry_core::identity::NodeId;

/// The only `graft.plan` layout this build understands.
/// 本版本唯一能读懂的 `graft.plan` 版式。
pub const GRAFT_PLAN_VERSION: u32 = 1;

/// One external overlay declaration as written to `graft.plan`.
/// 写进 `graft.plan` 的一条外部覆盖声明。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraftPlanDocument {
    /// Layout version; must equal `GRAFT_PLAN_VERSION`.
    /// 版式版本；必须等于 `GRAFT_PLAN_VERSION`。
    pub version: u32,
    /// Compile-time identity of the target face in the base tree.
    /// 原树中目标注册面的编译期身份。
    pub target: NodeId,
    /// The target's logical path, for humans and for string-selector overlays.
    /// 目标的逻辑路径，供人阅读，也供字符串选择器的 overlay 使用。
    pub target_path: String,
    /// Name, path, or identity of the external implementation's face.
    /// 外部实现注册面的名称、路径或身份。
    pub graft: String,
    /// Whether the whole subtree rooted at the target is replaced.
    /// 是否替换目标根节点下的整棵子树。
    pub full: bool,
}

impl GraftPlanDocument {
    /// The declaration a freshly created plan carries.
    /// 新建计划携带的声明。
    pub fn new(
        target: NodeId,
        target_path: impl Into<String>,
        graft: impl Into<String>,
        full: bool,
    ) -> Self {
        Self {
            version: GRAFT_PLAN_VERSION,
            target,
            target_path: target_path.into(),
            graft: graft.into(),
            full,
        }
    }

    /// Parse a plan document, refusing anything this version cannot read.
    /// 解析计划文档；读不懂的内容一律拒绝，而不是猜。
    pub fn parse(source: &str) -> Result<Self, GraftPlanDocumentError> {
        let mut version = None;
        let mut target = None;
        let mut target_path = None;
        let mut graft = None;
        let mut full = None;
        for (index, raw) in source.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, value) =
                line.split_once('=')
                    .ok_or_else(|| GraftPlanDocumentError::Malformed {
                        line: index + 1,
                        message: "expected `key=value`".to_owned(),
                    })?;
            let key = key.trim();
            let value = value.trim();
            match key {
                "version" => {
                    let parsed =
                        value
                            .parse::<u32>()
                            .map_err(|_| GraftPlanDocumentError::Malformed {
                                line: index + 1,
                                message: format!("version `{value}` is not a number"),
                            })?;
                    if parsed != GRAFT_PLAN_VERSION {
                        return Err(GraftPlanDocumentError::UnsupportedVersion(parsed));
                    }
                    version = Some(parsed);
                }
                "target" => {
                    let parsed =
                        value
                            .parse::<NodeId>()
                            .map_err(|_| GraftPlanDocumentError::Malformed {
                                line: index + 1,
                                message: format!("target `{value}` is not a node identity"),
                            })?;
                    target = Some(parsed);
                }
                "target_path" => target_path = Some(validate_path(value, index + 1)?),
                "graft" => graft = Some(validate_selector(value, index + 1)?),
                "full" => {
                    let parsed = match value {
                        "true" => true,
                        "false" => false,
                        _ => {
                            return Err(GraftPlanDocumentError::Malformed {
                                line: index + 1,
                                message: format!("full `{value}` must be true or false"),
                            });
                        }
                    };
                    full = Some(parsed);
                }
                other => return Err(GraftPlanDocumentError::UnknownKey(other.to_owned())),
            }
        }
        Ok(Self {
            version: version.ok_or(GraftPlanDocumentError::MissingKey("version"))?,
            target: target.ok_or(GraftPlanDocumentError::MissingKey("target"))?,
            target_path: target_path.ok_or(GraftPlanDocumentError::MissingKey("target_path"))?,
            graft: graft.ok_or(GraftPlanDocumentError::MissingKey("graft"))?,
            full: full.ok_or(GraftPlanDocumentError::MissingKey("full"))?,
        })
    }

    /// Render the canonical document, one `key=value` per line.
    /// 渲染规范文档，每行一个 `key=value`。
    pub fn render(&self) -> String {
        format!(
            "version={}\ntarget={}\ntarget_path={}\ngraft={}\nfull={}\n",
            self.version,
            self.target,
            self.target_path,
            self.graft,
            if self.full { "true" } else { "false" }
        )
    }

    /// The `cut … graft …` clause a host entry writes inside
    /// `static_graft_plan!`.
    /// 宿主入口写在 `static_graft_plan!` 里的 `cut … graft …` 子句。
    ///
    /// The string form needs no linked external implementation, which is why it
    /// is the form tooling can generate: the implementation may arrive later as
    /// a plugin, and the compiler is not asked to resolve a crate the author has
    /// not written yet.
    /// 字符串写法不需要链接外部实现，这正是工具能生成它的原因：实现可以稍后作为插件
    /// 到位，也不必要求编译器解析作者还没写的 crate。
    pub fn declaration(&self) -> String {
        format!(
            "cut \"{}\"{} graft \"{}\",",
            self.target_path,
            if self.full { " full" } else { "" },
            self.graft
        )
    }
}

fn validate_path(value: &str, line: usize) -> Result<String, GraftPlanDocumentError> {
    if value.is_empty() || value.split('/').any(str::is_empty) || value.contains(['\\', '"']) {
        return Err(GraftPlanDocumentError::Malformed {
            line,
            message: format!("target_path `{value}` is not a `/`-separated logical path"),
        });
    }
    Ok(value.to_owned())
}

/// Check one external implementation selector against the plan format.
/// 按计划格式校验一个外部实现选择器。
///
/// One rule, shared by the file writer and the parser, so a plan this crate
/// writes is always a plan this crate can read back.
/// 写入方与解析方共用同一条规则，因此本 crate 写出的计划永远读得回来。
pub fn validate_graft_selector(selector: &str) -> Result<(), GraftPlanDocumentError> {
    if selector.trim().is_empty() {
        return Err(GraftPlanDocumentError::InvalidSelector(
            "external graft name must be a non-empty selector".to_owned(),
        ));
    }
    if selector.contains(['/', '\\']) {
        return Err(GraftPlanDocumentError::InvalidSelector(
            "external graft name must not contain path separators".to_owned(),
        ));
    }
    if selector.chars().any(char::is_whitespace) || selector.contains('"') {
        return Err(GraftPlanDocumentError::InvalidSelector(
            "external graft name must be one word without quotes".to_owned(),
        ));
    }
    Ok(())
}

fn validate_selector(value: &str, line: usize) -> Result<String, GraftPlanDocumentError> {
    validate_graft_selector(value).map_err(|message| GraftPlanDocumentError::Malformed {
        line,
        message: format!("graft `{value}` is invalid: {message}"),
    })?;
    Ok(value.to_owned())
}

/// Why a `graft.plan` document was refused.
/// `graft.plan` 文档被拒绝的原因。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GraftPlanDocumentError {
    /// Document declares a layout this build cannot read.
    /// 文档声明的版式是本构建读不懂的。
    UnsupportedVersion(u32),
    /// The selector names something the plan format cannot store.
    /// 选择器命名了计划格式存不下的东西。
    InvalidSelector(String),
    /// Document omitted a required key.
    /// 文档缺少一个必需的键。
    MissingKey(&'static str),
    /// Document carried a key this format does not define.
    /// 文档带有本格式未定义的键。
    UnknownKey(String),
    /// Document had a line the format cannot read.
    /// 文档中存在本格式读不懂的行。
    Malformed {
        /// 1-based line the failure was found on.
        /// 发现失败的行号，从 1 起。
        line: usize,
        /// What was wrong with that line.
        /// 该行错在哪里。
        message: String,
    },
}

impl fmt::Display for GraftPlanDocumentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedVersion(version) => write!(
                formatter,
                "graft plan version `{version}` is not supported (expected {GRAFT_PLAN_VERSION})"
            ),
            Self::InvalidSelector(message) => write!(formatter, "{message}"),
            Self::MissingKey(key) => write!(formatter, "graft plan is missing `{key}`"),
            Self::UnknownKey(key) => write!(formatter, "graft plan has an unknown key `{key}`"),
            Self::Malformed { line, message } => {
                write!(formatter, "graft plan line {line}: {message}")
            }
        }
    }
}

impl std::error::Error for GraftPlanDocumentError {}

impl From<GraftPlanDocumentError> for String {
    fn from(error: GraftPlanDocumentError) -> Self {
        error.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document() -> GraftPlanDocument {
        GraftPlanDocument::new(
            NodeId::from_path("control/object/button/button.rs", "Button"),
            "root/control/button",
            "button_fast",
            false,
        )
    }

    #[test]
    fn a_document_round_trips_through_its_text_form() {
        let document = document();
        let text = document.render();
        assert_eq!(
            text,
            format!(
                "version=1\ntarget={}\ntarget_path=root/control/button\ngraft=button_fast\nfull=false\n",
                document.target
            )
        );
        assert_eq!(GraftPlanDocument::parse(&text).expect("parses"), document);
    }

    #[test]
    fn a_subtree_document_round_trips_and_renders_the_full_clause() {
        let mut document = document();
        document.full = true;
        assert_eq!(
            GraftPlanDocument::parse(&document.render()).expect("parses"),
            document
        );
        assert_eq!(
            document.declaration(),
            "cut \"root/control/button\" full graft \"button_fast\","
        );
        assert!(document.full);
    }

    /// The partial form renders without the `full` clause; the granularity now
    /// lives only in `full`, because the cut builder moved to the record
    /// resolver.
    /// 普通形式渲染时不带 `full` 子句；粒度现在只存在于 `full` 字段，因为切口构造器
    /// 已移入记录解析器。
    #[test]
    fn a_partial_document_omits_the_full_clause() {
        let document = document();
        assert_eq!(
            document.declaration(),
            "cut \"root/control/button\" graft \"button_fast\","
        );
        assert_eq!(document.target_path, "root/control/button");
        assert_eq!(document.graft, "button_fast");
        assert!(!document.full);
    }

    #[test]
    fn key_order_and_comments_do_not_matter() {
        let document = document();
        let reordered = format!(
            "# written by hand\ngraft=button_fast\nfull=false\n\ntarget_path=root/control/button\nversion=1\ntarget={}\n",
            document.target
        );
        assert_eq!(
            GraftPlanDocument::parse(&reordered).expect("parses"),
            document
        );
    }

    #[test]
    fn an_unknown_version_is_refused_instead_of_guessed() {
        let text = document().render().replace("version=1", "version=2");
        assert_eq!(
            GraftPlanDocument::parse(&text),
            Err(GraftPlanDocumentError::UnsupportedVersion(2))
        );
    }

    #[test]
    fn missing_unknown_and_malformed_keys_are_reported() {
        let missing = document().render().replace("full=false\n", "");
        assert_eq!(
            GraftPlanDocument::parse(&missing),
            Err(GraftPlanDocumentError::MissingKey("full"))
        );
        let unknown = format!("{}editor=vscode\n", document().render());
        assert_eq!(
            GraftPlanDocument::parse(&unknown),
            Err(GraftPlanDocumentError::UnknownKey("editor".to_owned()))
        );
        let malformed = document().render().replace("full=false", "full=yes");
        assert!(matches!(
            GraftPlanDocument::parse(&malformed),
            Err(GraftPlanDocumentError::Malformed { line: 5, .. })
        ));
        let selector = document()
            .render()
            .replace("graft=button_fast", "graft=a/b");
        assert!(matches!(
            GraftPlanDocument::parse(&selector),
            Err(GraftPlanDocumentError::Malformed { line: 4, .. })
        ));
        let path = document().render().replace(
            "target_path=root/control/button",
            "target_path=control//button",
        );
        assert!(matches!(
            GraftPlanDocument::parse(&path),
            Err(GraftPlanDocumentError::Malformed { line: 3, .. })
        ));
    }
}
