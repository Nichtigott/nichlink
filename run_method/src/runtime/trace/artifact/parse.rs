//! Parsing half of the trace artifact document.
//! trace artifact 文档的解析一半。
//!
//! Parsing is separate from the document type only to keep each file under the
//! repository's size ratchet. It interns every distinct function and file, so a
//! repeated name is leaked once rather than once per record.
//! 解析与文档类型分开，只是为了把每个文件保持在仓库尺寸棘轮之下。它对每个不同的函数名与文件名
//! 各驻留一次，因此重复的名字只泄漏一次，而不是每条记录一次。

use std::collections::BTreeMap;

use crate::registry_core::declaration::SourceLocation;
use crate::registry_core::identity::NodeId;
use crate::runtime::trace::{DataEdge, LocalKind, LocalValue, Observation, TraceMode};

use super::{TRACE_ARTIFACT_VERSION, TraceArtifact, TraceArtifactError, TraceFrame};

impl TraceArtifact {
    /// Parse a document, interning every distinct function and file once.
    /// 解析文档，对每个不同的函数名与文件名各驻留一次。
    ///
    /// Records may appear in any order, but a local resolves its source function
    /// from a frame already read and an edge resolves it from a local already
    /// read; the canonical render writes frames, then locals, then edges.
    /// 记录可以任意顺序出现，但局部值从已读入的帧解析其来源函数，边从已读入的局部值解析；
    /// 规范渲染按帧、局部值、边的顺序写出。
    pub fn parse(source: &str) -> Result<Self, TraceArtifactError> {
        let mut version = None;
        let mut namespace = None;
        let mut root = None;
        let mut mode = None;
        let mut frames: Vec<TraceFrame> = Vec::new();
        let mut locals: Vec<LocalValue> = Vec::new();
        let mut edges: Vec<DataEdge> = Vec::new();
        let mut frame_functions: BTreeMap<u64, &'static str> = BTreeMap::new();
        let mut local_functions: BTreeMap<u64, &'static str> = BTreeMap::new();
        for (index, raw) in source.lines().enumerate() {
            let line = raw.strip_suffix('\r').unwrap_or(raw);
            let at = index + 1;
            if line.trim().is_empty() || line.trim_start().starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                return Err(malformed(at, "expected `key=value`"));
            };
            match key.trim() {
                "version" => {
                    if version.is_some() {
                        return Err(malformed(at, "duplicate key `version`"));
                    }
                    let parsed = value.trim().parse::<u32>().map_err(|_| {
                        malformed(at, format!("version `{}` is not a number", value.trim()))
                    })?;
                    if parsed != TRACE_ARTIFACT_VERSION {
                        return Err(TraceArtifactError::UnsupportedVersion(parsed));
                    }
                    version = Some(parsed);
                }
                "namespace" => {
                    if namespace.is_some() {
                        return Err(malformed(at, "duplicate key `namespace`"));
                    }
                    namespace = Some(unescape(value.trim(), at)?);
                }
                "root" => {
                    if root.is_some() {
                        return Err(malformed(at, "duplicate key `root`"));
                    }
                    root = Some(value.trim().parse::<NodeId>().map_err(|_| {
                        malformed(
                            at,
                            format!("root `{}` is not a node identity", value.trim()),
                        )
                    })?);
                }
                "mode" => {
                    if mode.is_some() {
                        return Err(malformed(at, "duplicate key `mode`"));
                    }
                    mode = Some(TraceMode::parse(value.trim()).ok_or_else(|| {
                        malformed(at, format!("mode `{}` is not a trace mode", value.trim()))
                    })?);
                }
                "frame" => {
                    let fields = split_fields(value, 7, at, "frame")?;
                    let frame_id = parse_u64(fields[0], at, "frame id")?;
                    let parent = parse_optional_id(fields[1], at, "frame parent")?;
                    let node = fields[2].parse::<NodeId>().map_err(|_| {
                        malformed(
                            at,
                            format!("frame node `{}` is not a node identity", fields[2]),
                        )
                    })?;
                    let function = intern(&unescape(fields[3], at)?);
                    let source =
                        parse_optional_source(fields[4], fields[5], fields[6], function, at)?;
                    frame_functions.insert(frame_id, function);
                    frames.push(TraceFrame {
                        frame_id,
                        parent,
                        node,
                        function,
                        source,
                    });
                }
                "local" => {
                    let fields = split_fields(value, 10, at, "local")?;
                    let id = parse_u64(fields[0], at, "local id")?;
                    let frame_id = parse_optional_id(fields[1], at, "local frame")?;
                    let kind = parse_local_kind(fields[2]).ok_or_else(|| {
                        malformed(
                            at,
                            format!("local kind `{}` is not a local kind", fields[2]),
                        )
                    })?;
                    let observation = parse_observation(fields[3]).ok_or_else(|| {
                        malformed(
                            at,
                            format!("observation `{}` is not an observation", fields[3]),
                        )
                    })?;
                    let function = frame_id
                        .and_then(|frame_id| frame_functions.get(&frame_id).copied())
                        .unwrap_or("<local>");
                    let source = SourceLocation {
                        file: intern(&unescape(fields[7], at)?),
                        line: parse_u32(fields[8], at, "local source line")?,
                        column: parse_u32(fields[9], at, "local source column")?,
                        function,
                    };
                    local_functions.insert(id, function);
                    locals.push(LocalValue {
                        id,
                        name: unescape(fields[4], at)?,
                        type_name: unescape(fields[5], at)?,
                        value: unescape(fields[6], at)?,
                        kind,
                        source,
                        frame_id,
                        observation,
                    });
                }
                "edge" => {
                    let fields = split_fields(value, 6, at, "edge")?;
                    let from = parse_u64(fields[0], at, "edge source")?;
                    let to = parse_u64(fields[1], at, "edge destination")?;
                    let function = local_functions.get(&to).copied().unwrap_or("<runtime>");
                    let source =
                        parse_optional_source(fields[3], fields[4], fields[5], function, at)?;
                    edges.push(DataEdge {
                        from,
                        to,
                        label: unescape(fields[2], at)?,
                        source,
                    });
                }
                other => return Err(TraceArtifactError::UnknownKey(other.to_owned())),
            }
        }
        Ok(Self {
            version: version.ok_or(TraceArtifactError::MissingKey("version"))?,
            namespace: namespace.ok_or(TraceArtifactError::MissingKey("namespace"))?,
            root: root.ok_or(TraceArtifactError::MissingKey("root"))?,
            mode: mode.ok_or(TraceArtifactError::MissingKey("mode"))?,
            frames,
            locals,
            edges,
        })
    }
}

/// Split one record value into exactly `count` tab-separated fields.
/// 把一个记录取值切成恰好 `count` 个制表符分隔的字段。
fn split_fields<'a>(
    value: &'a str,
    count: usize,
    line: usize,
    record: &str,
) -> Result<Vec<&'a str>, TraceArtifactError> {
    let fields: Vec<&str> = value.split('\t').collect();
    if fields.len() != count {
        return Err(malformed(
            line,
            format!(
                "{record} expects {count} tab-separated fields, found {}",
                fields.len()
            ),
        ));
    }
    Ok(fields)
}

fn parse_u64(value: &str, line: usize, field: &str) -> Result<u64, TraceArtifactError> {
    value
        .parse()
        .map_err(|_| malformed(line, format!("{field} `{value}` is not a number")))
}

fn parse_u32(value: &str, line: usize, field: &str) -> Result<u32, TraceArtifactError> {
    value
        .parse()
        .map_err(|_| malformed(line, format!("{field} `{value}` is not a number")))
}

fn parse_optional_id(
    value: &str,
    line: usize,
    field: &str,
) -> Result<Option<u64>, TraceArtifactError> {
    if value == "-" {
        return Ok(None);
    }
    parse_u64(value, line, field).map(Some)
}

fn parse_optional_source(
    file: &str,
    line: &str,
    column: &str,
    function: &'static str,
    at: usize,
) -> Result<Option<SourceLocation>, TraceArtifactError> {
    if file == "-" {
        if line != "-" || column != "-" {
            return Err(malformed(
                at,
                "an absent source spells file, line, and column as `-`",
            ));
        }
        return Ok(None);
    }
    Ok(Some(SourceLocation {
        file: intern(&unescape(file, at)?),
        line: parse_u32(line, at, "source line")?,
        column: parse_u32(column, at, "source column")?,
        function,
    }))
}

fn parse_local_kind(token: &str) -> Option<LocalKind> {
    [
        LocalKind::Input,
        LocalKind::Binding,
        LocalKind::Output,
        LocalKind::Consumer,
    ]
    .into_iter()
    .find(|kind| kind.label() == token)
}

fn parse_observation(token: &str) -> Option<Observation> {
    [Observation::Observed, Observation::Unobserved]
        .into_iter()
        .find(|observation| observation.label() == token)
}

fn malformed(line: usize, message: impl Into<String>) -> TraceArtifactError {
    TraceArtifactError::Malformed {
        line,
        message: message.into(),
    }
}

/// Reverse the document's escaping, refusing an escape this format does not define.
/// 逆向执行文档的转义，拒绝本格式未定义的转义。
fn unescape(value: &str, line: usize) -> Result<String, TraceArtifactError> {
    let mut output = String::with_capacity(value.len());
    let mut characters = value.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            output.push(character);
            continue;
        }
        match characters.next() {
            Some('\\') => output.push('\\'),
            Some('t') => output.push('\t'),
            Some('n') => output.push('\n'),
            Some('r') => output.push('\r'),
            Some(other) => return Err(malformed(line, format!("unknown escape `\\{other}`"))),
            None => return Err(malformed(line, "trailing backslash in an escaped field")),
        }
    }
    Ok(output)
}

/// Intern one distinct string, reusing the same `&'static str` for equal text.
/// 驻留一个不同的字符串，相同文本复用同一个 `&'static str`。
///
/// `function` and `file` are `&'static str`, so a decoder that allocated per
/// record would leak once per record; this leaks once per distinct string
/// instead, bounded by the artifact's vocabulary.
/// `function` 与 `file` 是 `&'static str`，因此按记录分配的解码器会按记录泄漏；这里改为
/// 每个不同字符串泄漏一次，规模由 artifact 的词表决定。
fn intern(value: &str) -> &'static str {
    static INTERNER: std::sync::Mutex<std::collections::BTreeSet<&'static str>> =
        std::sync::Mutex::new(std::collections::BTreeSet::new());
    let mut interner = INTERNER.lock().unwrap_or_else(|poison| poison.into_inner());
    if let Some(existing) = interner.get(value).copied() {
        return existing;
    }
    let leaked: &'static str = Box::leak(value.to_owned().into_boxed_str());
    interner.insert(leaked);
    leaked
}
