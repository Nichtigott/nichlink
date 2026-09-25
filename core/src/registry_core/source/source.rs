//! Pure Rust-source lexer shared by search, callgraph, and indexing tools.
//! 供搜索、调用图与索引工具共用的纯 Rust 源码词法器。
//!
//! Every function here is a text transformation only: callers own file I/O.
//! 这里的所有函数只做文本变换，文件 I/O 由调用方负责。
//!
//! Function discovery lives here; `calls` scans call sites inside an extracted
//! body and `walk` owns the recursive source traversal.
//! 函数发现位于本页；`calls` 扫描已提取函数体内的调用点，`walk` 拥有递归源码遍历。
#[path = "lex.rs"]
pub(crate) mod lex;

#[path = "calls.rs"]
mod calls;
pub use calls::*;
#[path = "walk.rs"]
mod walk;
pub use walk::*;

/// One Rust function discovered in a source file.
/// 在源码文件中发现的一个 Rust 函数。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceFunction {
    /// Function name as written after the `fn` token.
    /// `fn` token 之后书写的函数名。
    pub name: String,
    /// Source text from the start of the line through the opening brace, trimmed.
    /// 从行首到左花括号的源码文本，已去除首尾空白。
    pub signature: String,
    /// Source text between the opening and closing braces.
    /// 左、右花括号之间的源码文本。
    pub body: String,
    /// 1-based line of the opening `fn` token.
    /// 以 1 起始的 `fn` 起始行行号。
    pub line: u32,
    /// 1-based line of the closing brace.
    /// 以 1 起始的右花括号所在行行号。
    pub end_line: u32,
}

/// Find the 0-based inclusive line range of a function by name.
/// 按名称查找函数的 0 起始闭区间行范围。
pub fn function_source_range(lines: &[&str], name: &str) -> Option<(usize, usize)> {
    // The start test runs on masked text and requires the name to be a whole
    // identifier. On the raw line, `// fn ghost() {` started a range for `ghost` that
    // closed on the next real function, and `fn renew(` matched the name `new`.
    // 起点判断跑在屏蔽后的文本上，并要求名字是完整标识符。按原始行时，`// fn ghost() {`
    // 会为 `ghost` 开出一个到下一个真实函数才闭合的范围，而 `fn renew(` 会匹配名字 `new`。
    let start = lines.iter().position(|line| {
        let masked = mask_non_code(line);
        let mut from = 0usize;
        while let Some(offset) = masked[from..].find("fn ") {
            let at = from + offset;
            let after = masked[at + "fn ".len()..].trim_start();
            if let Some(rest) = after.strip_prefix(name)
                && rest.trim_start().starts_with('(')
            {
                return true;
            }
            from = at + "fn ".len();
        }
        false
    })?;
    let mut depth = 0usize;
    let mut opened = false;
    for (index, line) in lines.iter().enumerate().skip(start) {
        // Braces inside a string or a comment are not code. Counting them on the
        // raw line let `let s = "{";` open a range that closed on a later `}` —
        // and with no closing brace at all the old fallback claimed a one-line
        // function. The masking below is the same rule the function index uses.
        // 字符串或注释里的花括号不是代码。按原始行计数会让 `let s = "{";` 打开一个在更后面的
        // `}` 处闭合的范围——而完全没有闭合花括号时，旧的兜底会声称这是个单行函数。下面的
        // 屏蔽与函数索引用的是同一条规则。
        for character in mask_non_code(line).chars() {
            match character {
                '{' => {
                    depth += 1;
                    opened = true;
                }
                '}' if opened => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
        if opened && depth == 0 {
            return Some((start, index));
        }
    }
    // Unbalanced at the end of `lines`: the function does not close inside what
    // the caller handed over. `Some((start, start))` reported that as a one-line
    // function, which is the opposite of what the caller needs to know.
    // 在 `lines` 末尾仍未配平：该函数没有在调用方给出的范围内闭合。过去用
    // `Some((start, start))` 把它报成单行函数，而这与调用方需要知道的事实相反。
    None
}

/// Advance `cursor` past one UTF-8 character, when there is one.
/// 若存在，把 `cursor` 推进一个 UTF-8 字符。
fn skip_one_character(bytes: &[u8], cursor: &mut usize) {
    if *cursor < bytes.len() {
        *cursor += 1;
        while bytes.get(*cursor).is_some_and(|byte| byte & 0xC0 == 0x80) {
            *cursor += 1;
        }
    }
}

/// Index function bodies without treating comments, strings, or macro text as Rust.
/// 扫描函数体时屏蔽注释、字符串和宏文本，避免把它们误认成 Rust 函数。
pub fn function_symbols(source: &str) -> Vec<SourceFunction> {
    let masked = mask_non_code(source);
    let bytes = masked.as_bytes();
    let mut result = Vec::new();
    // Line numbers come from one forward scan. Both offsets this reports are
    // non-decreasing — the loop resumes at the end of the function it just took —
    // so counting from the last offset instead of from zero makes the whole pass
    // linear; counting from zero per function made it quadratic in the number of
    // functions, which is the shape MCP's index walks.
    // 行号来自一次前向扫描。它报告的两个偏移都是非递减的——循环从刚取下的函数末尾继续——
    // 因此从上次的偏移继续数、而不是每次从 0 数，使整趟是线性的；过去每个函数都从 0 数，
    // 于是复杂度与函数个数相乘，而 MCP 的索引正是按那种形状遍历的。
    //
    // The `target < counted_to` arm is a correctness fallback, not the path any
    // caller takes today: a future caller that asks about an earlier offset gets
    // the right answer at the old cost instead of a wrong one.
    // `target < counted_to` 那一支是正确性兜底，不是今天任何调用方会走的路径：将来若有
    // 调用方问一个更早的偏移，它会以旧代价拿到正确答案，而不是拿到一个错答案。
    let mut counted_to = 0usize;
    let mut counted_lines = 1u32;
    let line_at = |target: usize, counted_to: &mut usize, counted_lines: &mut u32| -> u32 {
        if target < *counted_to {
            return source[..target]
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count() as u32
                + 1;
        }
        *counted_lines += source[*counted_to..target]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count() as u32;
        *counted_to = target;
        *counted_lines
    };
    let mut index = 0usize;
    while index < bytes.len() {
        if !is_ident_start(bytes[index]) {
            index += 1;
            continue;
        }
        let token_start = index;
        index += 1;
        while index < bytes.len() && is_ident_continue(bytes[index]) {
            index += 1;
        }
        if &masked[token_start..index] != "fn" {
            continue;
        }
        let mut name_start = index;
        while name_start < bytes.len() && bytes[name_start].is_ascii_whitespace() {
            name_start += 1;
        }
        if name_start >= bytes.len() || !is_ident_start(bytes[name_start]) {
            continue;
        }
        let mut name_end = name_start + 1;
        while name_end < bytes.len() && is_ident_continue(bytes[name_end]) {
            name_end += 1;
        }
        let name = masked[name_start..name_end].to_owned();
        let mut open = name_end;
        let mut angle_depth = 0usize;
        while open < bytes.len() {
            match bytes[open] {
                b'<' => angle_depth += 1,
                b'>' if angle_depth > 0 => angle_depth -= 1,
                b'{' if angle_depth == 0 => break,
                b';' if angle_depth == 0 => break,
                _ => {}
            }
            open += 1;
        }
        if open >= bytes.len() || bytes[open] != b'{' {
            continue;
        }
        let mut depth = 1usize;
        let mut close = open + 1;
        while close < bytes.len() && depth > 0 {
            match bytes[close] {
                b'{' => depth += 1,
                b'}' => depth = depth.saturating_sub(1),
                _ => {}
            }
            close += 1;
        }
        if depth != 0 {
            continue;
        }
        let line_start = source[..token_start].rfind('\n').map_or(0, |line| line + 1);
        let line = line_at(token_start, &mut counted_to, &mut counted_lines);
        let end_line = line_at(close, &mut counted_to, &mut counted_lines);
        let signature = source[line_start..open].trim().to_owned();
        result.push(SourceFunction {
            name,
            signature,
            body: source[open + 1..close - 1].to_owned(),
            line,
            end_line,
        });
        index = close;
    }
    result
}

fn is_ident_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_ident_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Replace comments and quoted literals with spaces while preserving offsets.
/// 用空格替换注释和引号字面量，同时保留原始偏移量。
/// Blank the contents of comments, string literals and character literals.
/// 把注释、字符串字面量与字符字面量的内容抹成空白。
///
/// The workspace's one text rule for "what is code": the source scanners use it so
/// a `fn` inside a comment or a brace inside a string is not read as Rust, and the
/// `conventions` gates use it so a workspace test's *fixture string* mentioning
/// `include!`/`std::fs` is not read as a violation. Macro bodies are deliberately
/// left alone — the macro name and its delimiter are code, and a gate that looks
/// for a macro invocation has to see them.
/// 本工作区关于"什么算代码"的唯一文本规则：源码扫描器用它，使注释里的 `fn` 或字符串里的花括号不被
/// 读成 Rust；`conventions` 的门禁也用它，使工作区测试里**夹具字符串**中提到的
/// `include!`/`std::fs` 不被读成违规。宏内容有意保留——宏名与它的定界符是代码，而查找宏调用的
/// 门禁必须看见它们。
pub fn mask_non_code(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut masked = bytes.to_vec();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'/') {
            index += 2;
            while index < bytes.len() && bytes[index] != b'\n' {
                masked[index] = b' ';
                index += 1;
            }
            continue;
        }
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
            masked[index] = b' ';
            if index + 1 < bytes.len() {
                masked[index + 1] = b' ';
            }
            index += 2;
            let mut depth = 1usize;
            while index < bytes.len() && depth > 0 {
                if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
                    depth += 1;
                    masked[index] = b' ';
                    masked[index + 1] = b' ';
                    index += 2;
                } else if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') {
                    depth = depth.saturating_sub(1);
                    masked[index] = b' ';
                    masked[index + 1] = b' ';
                    index += 2;
                } else {
                    if bytes[index] != b'\n' {
                        masked[index] = b' ';
                    }
                    index += 1;
                }
            }
            continue;
        }
        // A raw string is not closed by the quote that follows its opening `#`s, so the
        // ordinary branch below stopped at the first interior `"` and the rest of the
        // literal — braces, `fn`, call sites — was read as code. `r#"…"#` is ordinary
        // Rust, and JSON payloads inside it are full of interior quotes.
        // 原始字符串不由其开头 `#` 之后的那个引号闭合，因此下面的普通分支会在第一个内部 `"`
        // 处停下，而字面量剩下的部分——花括号、`fn`、调用点——会被读成代码。`r#"…"#` 是普通
        // Rust，而它里面的 JSON 载荷满是内部引号。
        if let Some(end) = lex::raw_string_end(bytes, index) {
            for byte in &mut masked[index..end] {
                if *byte != b'\n' {
                    *byte = b' ';
                }
            }
            index = end;
            continue;
        }
        if bytes[index] == b'"' {
            let quote = bytes[index];
            masked[index] = b' ';
            index += 1;
            while index < bytes.len() {
                let escaped = bytes[index] == b'\\';
                if bytes[index] != b'\n' {
                    masked[index] = b' ';
                }
                index += 1;
                if escaped && index < bytes.len() {
                    if bytes[index] != b'\n' {
                        masked[index] = b' ';
                    }
                    index += 1;
                } else if bytes[index - 1] == quote {
                    break;
                }
            }
            continue;
        }
        // A `'` opens a character literal only when that literal closes. A
        // lifetime (`'a`), a label (`'outer`) or the apostrophe of `&'static` has
        // no closing quote, and treating it as one masked everything up to the
        // next apostrophe — a function's `{` included — so every function with a
        // lifetime parameter vanished from the index, and every call after a
        // `&'static` was missed. The rule here is the lexer's: `'\…'`, or one
        // character followed by `'`.
        // `'` 只有在字符字面量闭合时才是它的起始。生命周期（`'a`）、标签（`'outer`）或
        // `&'static` 的撇号没有闭合引号，把它当成引号会一路遮到下一个撇号——包括函数的 `{`
        // ——于是每个带生命周期参数的函数都从索引里消失，`&'static` 之后的调用也全部漏掉。
        // 这里的规则与词法器相同：`'\…'`，或一个字符后紧跟 `'`。
        if bytes[index] == b'\'' {
            let mut cursor = index + 1;
            if bytes.get(cursor) == Some(&b'\\') {
                cursor += 1;
                match bytes.get(cursor) {
                    // `\u{…}`: the escape is delimited by braces.
                    Some(&b'u') if bytes.get(cursor + 1) == Some(&b'{') => {
                        cursor += 2;
                        while cursor < bytes.len() && bytes[cursor] != b'}' {
                            cursor += 1;
                        }
                        cursor = (cursor + 1).min(bytes.len());
                    }
                    // `\xNN`: exactly two hex digits.
                    Some(&b'x') => cursor = (cursor + 3).min(bytes.len()),
                    // `\n`, `\'`, `\\`, an escaped multi-byte character: one.
                    _ => skip_one_character(bytes, &mut cursor),
                }
            } else {
                skip_one_character(bytes, &mut cursor);
            }
            if bytes.get(cursor) == Some(&b'\'') {
                for byte in &mut masked[index..=cursor] {
                    if *byte != b'\n' {
                        *byte = b' ';
                    }
                }
                index = cursor + 1;
            } else {
                // A lifetime or a label: an apostrophe is not a token either
                // scanner looks for, so masking it alone is enough.
                // 生命周期或标签：撇号不是两个扫描器要找的 token，只遮掉它本身即可。
                masked[index] = b' ';
                index += 1;
            }
            continue;
        }
        index += 1;
    }
    String::from_utf8(masked).unwrap_or_else(|_| source.to_owned())
}

/// Collect the deduplicated `kind:` values used in registration declarations.
/// 收集注册声明中出现过的 `kind:` 取值，去重并排序。
pub fn registration_kinds(source: &str) -> Vec<String> {
    let mut kinds = Vec::new();
    for line in source.lines() {
        // A `kind:` in a comment or a string is prose, not a declaration: the raw line
        // scan reported a kind named `Ghost` for `// kind: Ghost`.
        // 注释或字符串里的 `kind:` 是散文而不是声明：按原始行扫描会为 `// kind: Ghost`
        // 报出一个名叫 `Ghost` 的 kind。
        let masked = mask_non_code(line);
        let trimmed = masked.trim();
        if let Some(kind) = trimmed.split_once("kind:").map(|(_, remainder)| remainder) {
            let kind = kind
                .trim()
                .split(|character: char| {
                    character == ',' || character == '}' || character.is_whitespace()
                })
                .next()
                .unwrap_or_default();
            if !kind.is_empty()
                && kind
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || character == '_')
            {
                kinds.push(kind.to_owned());
            }
        }
    }
    kinds.sort();
    kinds.dedup();
    kinds
}
#[cfg(test)]
#[path = "source_tests.rs"]
mod tests;
