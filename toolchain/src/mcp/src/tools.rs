//! Tool catalog, the query implementations, and the write path's dispatch.
//! 工具目录、查询实现与写入路径的分派。
//!
//! The catalog below is the authority on how many tools there are and what each one reads. This
//! paragraph describes the *families* instead of counting them, because a count here goes stale the
//! moment a tool is added — it said "five + eight, plus `apply`" while the catalog held seventeen
//! (audit `L4`).
//! 下面的目录才是"有几个工具、每个读什么"的权威。这一段描述的是**几类**而不是数个数，因为这里
//! 的数字会在新加一个工具的那一刻过期——它曾说"五个 + 八个，外加 `apply`"，而目录里其实有十七个
//! （审计 `L4`）。
//!
//! One family reads Rust source text — `nichlink.search` also matches registry faces
//! (logical path, kind, module, registry_name) and annotates each with the build's verdict.
//! Another answers from evidence that is not
//! source text: `nichlink.registry` derives the tree the build derives
//! (`crate::build_time::face_views`), `nichlink.explain` reads the files the
//! build *published* (`target/nichlink/out`) for scope and release pruning,
//! `nichlink.diff` states the face-level delta between those two sides, and
//! `nichlink.trace` reads a recorded trace artifact, refusing one that describes
//! another tree; `nichlink.mir` reads a `-Zunpretty=mir` dump or a JSONL artifact
//! and can emit that JSONL, which nothing in the workspace ever wrote; and
//! `nichlink.unified` merges the two, where a live call confirms a compiler
//! candidate. Three tools write, and all three preview before they do:
//! `nichlink.apply` edits faces through the authoring executor, `nichlink.new_project`
//! scaffolds a host with `build_time::scaffold::create_project`, and `nichlink.plugin`
//! stores one plugin lock record through the kernel's `PluginCatalog` gate
//! (`apply.rs`, `new_project.rs` and `plugin.rs` explain each contract). What none of
//! them reports is contract,
//! admission, or registration-rule data: those live in the built
//! `RegistrationSnapshot`s, which need the compiled registrations rather than a
//! scan or a manifest.
//! 一类工具读取 Rust 源码文本——`nichlink.search` 还会匹配注册面（逻辑路径、kind、module、registry_name），
//! 并把构建的判断标在每一条上。另一类用非源码文本的证据作答：`nichlink.registry` 推导出构建所推导
//! 的那棵树（`crate::build_time::face_views`）；`nichlink.explain` 读构建**发布**的文件
//! （`target/nichlink/out`），回答作用域与发布剪枝；`nichlink.diff` 说出两侧的面级差异；
//! `nichlink.trace` 读取已记录的 trace artifact，并拒绝描述另一棵树的那份；`nichlink.mir` 读
//! `-Zunpretty=mir` 转储或 JSONL artifact，并能输出那份无人写过的 JSONL；`nichlink.unified` 把两者
//! 合并，真实调用在其中确认编译器候选。三个工具会写入，而三者都先预览再写：`nichlink.apply`
//! 经 authoring 执行器编辑注册面，`nichlink.new_project` 用
//! `build_time::scaffold::create_project` 脚手架出一个宿主，`nichlink.plugin` 经内核的
//! `PluginCatalog` 闸门存下一条插件锁记录（三份契约分别见 `apply.rs`、`new_project.rs` 与
//! `plugin.rs`）。它们都没有报告的是 contract、admission 与 registration rule 数据：那些住在已构建的
//! `RegistrationSnapshot` 里，需要已编译的注册，而不是扫描或清单。
//!
//! online: the dispatch layer answers for trees that were never built as well as for built ones, and a face added since the build is only in the sources.

use serde_json::{Value, json};
use std::path::Path;

use crate::mcp::adopted::adopted;
use crate::mcp::affected::affected;
use crate::mcp::apply::apply;
use crate::mcp::build_evidence::explain;
use crate::mcp::callgraph::callgraph;
use crate::mcp::check::check;
use crate::mcp::converge::converge;
use crate::mcp::diff::diff;
use crate::mcp::grafts::grafts;
use crate::mcp::impact::impact;
use crate::mcp::mir::{mir, unified};
use crate::mcp::new_project::new_project;
use crate::mcp::overlay::overlay;
use crate::mcp::plugin::plugin;
use crate::mcp::protocol::{error_response, success};
use crate::mcp::read::read_source;
use crate::mcp::registry::registry_brief;
use crate::mcp::search::search;
use crate::mcp::source_index::{load_one, load_sources, required_path, resolve_root};
use crate::mcp::trace::trace;
use crate::mcp::usages::usages;
use crate::mcp::verify::verify;

pub(crate) fn tools() -> Vec<Value> {
    vec![
        tool(
            "nichlink.search",
            "Find registration faces, source files, and Rust function declarations by name. Faces \
             come first and match on logical path, kind, module, or registry_name — the spellings the \
             other tools use — and each carries the build's verdict: `ok`, `added since build`, \
             `re-identified` (a `kind` change under an unmoved file, with both identities), or \
             `build unknown` when nothing has been published. A manifest that no longer describes \
             these sources is announced above the verdicts as `stale (run nichlink check)`, because \
             they then describe the tree that build saw rather than this one. A registration file \
             the derivation could not parse is counted as `unparsable faces N` above the hits, and \
             that line is **not** subject to `limit`: it reports the tree that was read rather \
             than one more result row, so it keeps appearing when the limit has already cut the \
             face list short. Below them the file \
             and function hits are unchanged. The tree half needs the identity namespace; a root \
             Cargo cannot name still answers the source half and says the tree half is unavailable. \
             A virtual workspace root is a root too, and it is answered as the workspace it is: \
             every member appears with its status (`published`, `not built`, `no faces`, or \
             `unresolvable` with the reason), each member's matching faces are grouped under it \
             with a line naming which tree those rows came from — a face is matched on facts the \
             published record does not carry, so a member's own records are read first and the \
             sources are derived when they cannot answer — and a member whose tree cannot be \
             derived is named rather than hidden behind the file hits. With `converge: true` the answer is layered — the tree's verdicts, then the call chain the name leads into, then the next step and the bounds of what was **not** looked at — so one call localizes instead of three. `query` matches **names** — faces, files, functions — while `literal` matches **text** anywhere in a source file: raw bytes, case-sensitive, comments and string literals included. The literal mode is what a failing assertion's message usually needs (find where that message is produced), and it replaces leaving the tool to grep for it. Pass one of the two, not both.",
            json!({"type":"object","properties":{"names":{"type":"array","items":{"type":"string"},"description":"several bare names in one call: each gets its own group, answered by the same path a single name takes; `--names a,b` on the command line, or a JSON array in `--json`"},"query":{"type":"string","description":"a name: face, file or function"},"literal":{"type":"string","description":"text to find verbatim anywhere in a source file (raw bytes, case-sensitive, comments and string literals included)"},"context":{"type":"integer","minimum":0,"maximum":10,"description":"literal mode: also print this many lines around each hit (default 0; the reply points at it)"},"converge":{"type":"boolean","description":"answer in layers: the tree's verdicts, then the call chain this name leads into, then the next step and the bounds of what was looked at"},"root":{"type":"string"},"limit":{"type":"integer","minimum":1,"maximum":200}},"anyOf":[{"required":["query"]},{"required":["names"]},{"required":["literal"]}]}),
        ),
        tool(
            "nichlink.digest",
            "One bounded summary of one file, for the case the maintainer named: a file holding \
             several algorithms where one branch of one of them is wrong. Each row gives a \
             function's line range, how many names it calls, how many callers it has, whether a test \
             names it, and the first line of the contract above it. It does **not** read the file \
             out in full, and it says so: which branches are dead or covered and whether the static \
             walk reaches a function are the census's columns (`check {face}`), and a body is \
             `read {path, line}`'s.",
            json!({"type":"object","properties":{"file":{"type":"string","description":"a path as this root sees it (e.g. `src/model/entry.rs`)"},"root":{"type":"string"}},"required":["file"]}),
        ),
        tool(
            "nichlink.conformance",
            "Say what an anchor's claim is right now: which ledger revision is in force (an \
             adoption is a lease, so the newest line wins), whether it still holds or lapsed at a \
             named file, and which files it covers. It answers the half of \"was this design carried \
             through?\" that the ledger itself can prove, and it names the half it does not answer — \
             whether the siblings follow the same shape (that is `consistency --specimen <anchor>`) \
             and the declared fields a specimen carries (those live in its own source, and \
             `consistency --specimen` reads them from there).",
            // No `required`: without an anchor the tool lists **every** anchor, one line each. The
            // schema used to demand it, and the client's `keys:` line derives its `*` marks from
            // `required` — so every reader was told `anchor*` about a call that no longer needs it,
            // which is a self-description that disagrees with the behaviour (found in the round-8
            // review, which checked the acceptance shape against the schema).
            // 不再 `required`：不带 anchor 时该工具**逐行列出每个锚点**。schema 以前要求它，而客户端的 `keys:`
            // 行正是从 `required` 推出那些 `*` 的——于是每个读者都被告诉 `anchor*`，而那次调用早已不需要它；
            // 这是**自述与行为不一致**（第八轮复核对着 schema 核接受形状时发现）。
            json!({"type":"object","properties":{"anchor":{"type":"string","description":"the route a ledger entry names; omit it to list every anchor, one line each"},"root":{"type":"string"}}}),
        ),
        tool(
            "nichlink.consistency",
            "Compare siblings directly under one logical path and name the ones that differ — \
             the shape the maintainer calls a mixed supply chain: one object following a different \
             convention from its siblings while every file is locally plausible. The sibling set is \
             read from the registration tree (a fact only a tree has); `by` picks the signal: `api` \
             (default) compares the names each sibling's own file calls, `kind` and `source` compare \
             the declared value. It prints what the majority shares that an outlier lacks, and what \
             the outlier calls that nobody else does. `specimen` is the other baseline: given a \
             ledger anchor it compares every sibling's own file against the declared shape the \
             ledger's entry **in force** certifies (parts, exports, handle_traits, part_traits, read \
             with the kernel's face parser) and names who lacks or re-states which declaration — the \
             design was written down and the question is whether it was carried through, so a \
             sibling that declares more is not reported. **It reads text**: a convention living in a \
             shared helper or generated code is invisible, units and arithmetic are not compared, \
             and an outlier is a place to look rather than a defect. A tree that comes from \
             published records is answered as such instead of being filled with defaults.",
            json!({"type":"object","properties":{"parent":{"type":"string","description":"the logical path whose children to compare (one level under it)"},"specimen":{"type":"string","description":"a ledger anchor: compare its siblings against the shape the ledger's entry in force certifies (its parent is derived from the anchor)"},"by":{"type":"string","enum":["api","kind","source","shape"],"description":"which signal to compare; omit it and both `api` (the names each sibling's own file calls) and `shape` (the fields each sibling declares) are compared — a sibling that declares an extra or a missing field is named in one call"},"full":{"type":"boolean","description":"print every member's row; the default prints the family line, the outliers and their source excerpts, and withholds the rest (the reply says how many were withheld)"},"root":{"type":"string"}},"anyOf":[{"required":["parent"]},{"required":["specimen"]}]}),
        ),
        tool(
            "nichlink.why",
            "Gather the upstream facts one `path:line` depends on, in one call: the contract lines \
             above its definition, who calls it (with the same test-file/outside-directory notes \
             `callgraph` prints), which adoption-ledger entries name that file, and the **plan half** \
             — whether this file is one of the sources the build selected or one the entry's scope \
             leaves out, whether a `#[cfg]` gates the definition, and which declared graft cut names \
             the face that owns the file (with the entry and line). Those three read the same \
             implementations `explain` and `grafts` read, so the answers cannot disagree. It also \
             names what it does **not** answer — how a grafted subtree looks at runtime — and which \
             call answers that.",
            json!({"type":"object","properties":{"at":{"type":"string","description":"a `path:line` inside this tree; a registration node is `explain`'s question"},"root":{"type":"string"}},"required":["at"]}),
        ),
        tool(
            "nichlink.locate",
            "Rank the places a symptom's own words point at, so a reader holding an assertion \
             message or an error does not have to guess a name first. The reply lists up to five \
             candidates as `file:line \u{60}name\u{60}` with the reason each ranked — word overlap \
             with the function's name and with the doc comments above it, a whole-phrase bonus when \
             the symptom's exact words appear in the file, and whether the file looks like a test \
             file. **It reads text only**: no call graph, no build records, no runtime evidence, so \
             a paraphrase that shares no word finds nothing, dynamic dispatch and macro expansion \
             are invisible, and a hit is a place to look rather than the defect. An empty answer \
             carries the root and the route back (`search {literal}` for an exact phrase).",
            json!({"type":"object","properties":{"symptom":{"type":"string","description":"the words of the symptom: a failing assertion's message, an error, or a description of what goes wrong"},"root":{"type":"string"}},"required":["symptom"]}),
        ),
        tool(
            "nichlink.inspect",
            "Summarize functions and registration declarations in one Rust file. A **virtual \
             workspace root** is answered as the workspace: the member whose root contains the \
             path answers it, from that member's own root, or the reply says no member owns the \
             path.",
            json!({"type":"object","properties":{"path":{"type":"string"},"root":{"type":"string"}},"required":["path"]}),
        ),
        tool(
            "nichlink.callgraph",
            "Show direct static callers and callees for one function. `limit` bounds how many \
             definitions are listed (default 5, at most 50); a truncation line names how many \
             were withheld, and Each definition's own lines are included unless `source: false` says otherwise — the short
             answer is available, it is just no longer the default, because every evaluation round
             spent one extra call to see the body. `path` selects one definition when several share
             the name. A \
             **virtual workspace root** is answered as the workspace: a named `path` is answered \
             by the member that owns it, and without one every member is asked and its answer \
             grouped under it, with `tree unavailable (reason)` where a member could not answer. \
             `orphans: true` asks the other question — which functions this tree defines and no \
             function here calls — and counts the test-file half separately, because the harness is \
             what calls those.",
            json!({"type":"object","properties":{"function":{"oneOf":[{"type":"string"},{"type":"array","items":{"type":"string"},"maxItems":8}],"description":"one symbol name, or up to 8 in one call (each answer is byte-identical to asking alone)"},"orphans":{"type":"boolean","description":"list the functions this tree defines that no function here calls; functions in test files are counted separately because the test harness calls them"},"path":{"type":"string"},"limit":{"type":"integer","minimum":1,"maximum":50},"source":{"type":"boolean","description":"also print each definition's own lines (capped, declared through the truncation outlet)"},"root":{"type":"string"}},"anyOf":[{"required":["function"]},{"required":["orphans"]}]}),
        ),
        tool(
            "nichlink.read",
            "Read a bounded window around a line, an explicit `lines` range, or the \
             `whole` file — of **any UTF-8 text file this tree carries** (the adoption ledger \
             and a build log included; the `.rs` ones additionally carry a symbol index). \
             **Every header states the file's total**, as `path:START-END (N \
             lines)`: `N` is the file's own line count, not the printed range, so a windowed \
             read still says how much of the file is left — which is the fact the old \
             81-line window withheld, forcing six calls for a 473-line file. Default: a \
             window of `context` lines either side of `line` (context 8, at most 120, \
             240 lines) — a peek, because a caller that named only a line asked for one; \
             pass `context` for a wider window and `whole` for the file. \
             `whole: true` prints the file; `lines: \"120-260\"` prints that \
             forward, 1-based range. The three shapes are exclusive and mixing them is \
             refused by name. An explicit whole/range read is bounded at 1200 lines and a \
             reply past that bound is cut by the shared truncation notice, which names the \
             withheld count, the cap, and the narrower request that reaches the rest — \
             never a silently short answer. The header keeps the file's total either way. A \
             **virtual workspace root** is answered as the workspace: the member whose root \
             contains the path answers it, from that member's own root, or the reply says no \
             member owns the path.",
            json!({"type":"object","properties":{
                "path":{"type":"string"},
                "line":{"type":"integer","minimum":1,"description":"window: the line to centre on (default 1)"},
                "context":{"type":"integer","minimum":0,"maximum":120,"description":"window: lines either side of `line` (default 8)"},
                "whole":{"type":"boolean","description":"print the whole file (bounded at 1200 lines)"},
                "lines":{"type":"string","description":"print one forward, 1-based range, e.g. `120-260` (bounded at 1200 lines)"},
                "root":{"type":"string"}
            },"required":["path"]}),
        ),
        tool(
            "nichlink.status",
            "Report the source root and indexed Rust file/function counts.",
            json!({"type":"object","properties":{"root":{"type":"string"}}}),
        ),
        tool(
            "nichlink.apply",
            "Edit this package's registration faces through the same authoring executor Studio \
             uses, so the kernel's admission, parent-rule, and topology checks run on the change. \
             **The seven actions fall into two classes, and the class decides whether a \
             hand-written face is a legal subject**: `add`, `deepen`, `cut` and `promote` are \
             **additive** — they write declarations (a new face, a layer inside one, the entry a \
             graft plan reads, a landed record) and reach an existing hand-written face fine; \
             `edit`, `rename` and `delete` are **rewrites** — they rewrite the face's own file, and \
             the executor only rewrites what it generated, so a hand-written face is refused by \
             name (taking one over is a separate, explicit adoption, not a spelling these actions \
             accept). Read this before choosing an action, because the refusal arrives after the \
             request. \
             `action` is `add` (create `fields.module` under `parent`), `edit` (change the \
             `fields` the request names and keep the rest), `rename` (change `fields.module`), or \
             `delete` (move the face's module into NichLink's recoverable trash). `node` names \
             the face and `parent` the parent, by logical path — the one nichlink.registry \
             reports — or by identity. `handle_contracts` and `part_contracts` are set by `add` \
             only: the executor's edit field order does not carry them, so `edit`/`rename` refuse \
             those two keys by name rather than returning success while the value is dropped. \
             **A request is previewed unless `apply` is true**: the \
             preview runs the real operation on a throwaway copy and returns the file diff plus \
             the registration tree it produces; `apply: true` writes it and names the files it \
             wrote. Every reply is the tree that results, so the next call can be aimed with it. \
             Every reply also states the **consequences** of the change, as static facts with their \
             boundary: the in-tree test lines that spell this face's name, and whether the entry \
             plan names it at all — so what this change will break, and whether the application \
             ships it, are answered before the write instead of after a red run. A refusal that has \
             a way forward says so: a parent that owns no registry is told `needs_registry`, and a \
             slot whose face now owns children is told `full graft`, while the rule itself still \
             refuses. `deepen` is the opposite direction: `inside.parts` adds a layer **inside** one \
             face — a parts struct and an accessor in that face's own file — and leaves its \
             declaration, its public path, its tree row and every factory-shape pin alone; the reply \
             also prices the other reading (another face under it), so the decision is made before \
             the write rather than by a red run. `promote` lands a confirmed external graft record into the base tree: the target face's declaration is rewritten to carry the external implementation's fields, the `cut(…)` entry that handed the slot over is retired, and the record directory moves to `.nichlink/trash/`. It needs `selector`, `apply: true` and `confirm: true`, and it refuses by name a declaration that names its implementation by string — a string says nothing about where that implementation lives, so redeclare the slot as `graft(<crate>::<module>::NODE_ID)` and run it again — an external face it cannot find in the tree that declaration names, and a target file that is hand-written rather than generated, because the executor only rewrites what it wrote; `kind` moves with the rest and is an identity input, so the reply warns when it changed. A \
             **virtual workspace root** names no package and no unique owner, so a write there is \
             refused with the candidate member directories and nothing is copied or written — a \
             write runs against a member it is the unique owner of, or it does not run. **`fields.module` is a bare snake_case module name** (`widget`), not the logical path the tree reports: the executor names the module's directory and file, so `control::object::widget` is refused by name. Every other `fields` value is a **string**: `exports` is one export per call rather than an array, `requires` entries are spelled `capability=>provider`, and `handle_traits` entries are the **labels** the registration rule checks (a Rust path such as `crate::control::ControlHandle` is accepted here and only the parent rule refuses it later). The one non-string key is `needs_registry`, a boolean. **A request the kernel refuses comes back as an error** (`isError` true), because a request this tool cannot carry out is a tool failure rather than a fact about the tree; the read tools are the other way round and print their verdict in the body.",
            json!({"type":"object","properties":{
                "action":{"type":"string","enum":["add","edit","rename","delete","deepen","cut","promote"]},
                "selector":{"type":"string","description":"promote: the .nichlink/external-grafts/<selector>/ directory to land"},
                "implementation":{"type":"string","description":"promote: where the external implementation crate lives, when the declaration names it as a Rust path but that crate is not a path dependency of this package"},
                "node":{"type":"string","description":"edit/rename/delete/deepen: the face, by logical path or identity"},
                 "inside":{"type":"object","description":"deepen: the layer to add inside the face. `parts` is an object of `field: Type` pairs (simple types only); the action writes a parts struct plus an accessor into the face's own file and touches nothing else — no declaration, no tree row, no public path, no factory-shape pin","properties":{"parts":{"type":"object","additionalProperties":{"type":"string"}}},"required":["parts"]},
                "parent":{"type":"string","description":"add: the parent's logical path or identity; defaults to the registry root"},
                "fields":{"type":"object","description":"the face's fields; edit and rename change only the keys given, add takes the rest as defaults. Values are strings unless noted: `module` is a bare snake_case name, `exports` one export per call, `requires` entries `capability=>provider`, `handle_traits` entries rule labels","properties":{"needs_registry":{"type":"boolean","description":"the one non-string field"}},"additionalProperties":{"type":"string"}},
                "apply":{"type":"boolean","description":"false (the default) previews on a copy; true writes to the project"},
                "full":{"type":"boolean","description":"print every face of the resulting tree instead of the census row and the face that changed (default false)"},
                "confirm":{"type":"boolean","description":"delete and promote: must be true. A delete is the one operation whose preview a caller can step past by accident, so the request says it rather than the bridge adding it"},
                "root":{"type":"string"}
            },"required":["action"]}),
        ),
        tool(
            "nichlink.new_project",
            "Create a host project with the same scaffold `nichlink new` (the CLI) and Studio's \
             new-project wizard run — `crate::build_time::scaffold::create_project` — so the \
             manifest, build script, source entry, editor snippets, and detected dependency \
             source are the ones this workspace ships rather than a second renderer written here. \
             `directory` is where the project goes: a relative one is taken against `root` (the \
             package or workspace root this call resolved), and a destination outside that root, \
             or one that steps out with `..`, is refused by name before anything is created. A \
             **virtual workspace root** is answered against that root's own directory — the \
             destination is explicit, so containment in the root is the whole ownership question \
             and no member has to be picked. \
             `package` is the crate name and `kind` is `binary` or `library`. `faces` creates \
             registration faces **in the same call**, so starting a project that already has two \
             objects is one request rather than a scaffold plus one `apply add` per object: each \
             entry is `{\"fields\": {\"module\": \"button\", \"kind\": \"Button\"}, \"parent\": \"<node>\"}` \
             — the shape `apply add` takes, without `action` — and `parent` defaults to the project \
             root. The entries run through that same executor, in order, and the reply reports the \
             faces the project now derives, so a second `registry` call is not needed; an entry the \
             kernel refuses leaves the destination untouched, because the whole project is built \
             beside it and moved in with one rename. **A request is \
             previewed unless `apply` is true**: the preview runs that executor in a throwaway \
             directory and prints every path it would write, with the bytes, under the real \
             destination and the root it is inside; `apply: true` writes them. A destination that \
             already exists must be empty, and then the request itself has to say \
             `confirm: true`, because the write lands in a directory the caller already has. The \
             scaffolded manifest declares its own `[workspace]`, so the project is not a member of \
             the root this call ran in.",
            json!({"type":"object","properties":{
                "directory":{"type":"string","description":"where the project is written; relative paths are taken against `root` and a destination outside it is refused"},
                "package":{"type":"string","description":"the new crate's name (letters, digits, `_` or `-`)"},
                "kind":{"type":"string","enum":["binary","library"],"description":"the entry the scaffold writes: `src/main.rs` or `src/lib.rs`"},
                "faces":{"type":"array","description":"registration faces to create in the same call, in order; each entry is `{\"fields\": {\"module\": \"button\", \"kind\": \"Button\"}, \"parent\": \"<node>\"}` (the shape `apply add` takes, without `action`); `parent` defaults to the project root","items":{"type":"object","properties":{"fields":{"type":"object","description":"the new face's fields, the same ones `apply add` takes (`module` is required in practice: a module name is what the executor names the file and directory after)"},"parent":{"type":"string","description":"the logical path or identity of the parent face; defaults to the project root"}},"required":["fields"]}},
                "apply":{"type":"boolean","description":"false (the default) previews in a throwaway directory; true writes the project"},
                "confirm":{"type":"boolean","description":"must be true when the destination directory already exists: the write lands in a directory the caller already has, so the request says it rather than the bridge assuming it"},
                "dependency":{"type":"string","enum":["path","registry","git"],"description":"where the manifest's `nichlink-toolchain` lines point: `path` at a checkout (this tool's own, or the one `path` names — the only spelling that resolves with no network), `registry` at the published release, `git` at a repository. Never guessed into `git`"},
                "path":{"type":"string","description":"a NichLink checkout for the manifest to point at; the way to get offline-resolving dependencies from a tool outside one"},
                "git":{"type":"string","description":"the repository the generated manifest pulls from; it is the URL for `dependency: \"git\"` and defaults to this project's own. Passing it without saying `dependency: \"git\"` is refused rather than obeyed"},
                "root":{"type":"string"}
            },"required":["directory","package","kind"]}),
        ),
        tool(
            "nichlink.plugin",
            "Write one plugin record into this package's lock — the same path Studio's plugin \
             form takes (`submit_plugin`). `source` is `official` or `user` and selects \
             `official.lock` or `user.lock` under `.nichlink/plugins/`; `framework`, `package`, \
             `version`, `crate`, and `checksum` are the record's identity fields and `mode` is \
             `extension` or `replacement`. An `official` record is admitted only when the kernel's \
             `PluginCatalog::contains_record` says the lock already accounts for that package \
             identity — the trust rule the runtime reads — and the lock's own parser decides \
             whether the append is legal, both before anything is written. The entry file that \
             imports the crate (`official.rs` or `user.rs`) carries the other half of the same \
             decision, so a failed lock write restores it. A record the lock already carries is \
             reported as already selected and nothing is written. **A request is previewed unless \
             `apply` is true**: the preview computes the exact bytes with the same kernel calls \
             and prints what each file would gain, so it cannot drift from the write; `apply: \
             true` stores them. A plugin write requires the request to say `confirm: true` itself, \
             because the lock is the artifact the host admits plugins from. The three provenance \
             columns of the ten-field spelling — `signature`, `fingerprint`, `revocations` — are \
             optional and are what an `official` write is *for*: the trusted lock already accounts \
             for that package identity, and a record carrying provenance the bare one does not is \
             the promotion the kernel accepts (it replaces that record rather than colliding with \
             it), while a request that names none of them writes the seven-field line. A \
             **virtual workspace root** names no package, so a plugin write there is refused with \
             the candidate member directories.",
            json!({"type":"object","properties":{
                "source":{"type":"string","enum":["official","user"],"description":"which lock the record goes into: `official.lock` or `user.lock`"},
                "framework":{"type":"string","description":"the target framework the record was written for"},
                "package":{"type":"string","description":"the plugin's package name (its identity)"},
                "version":{"type":"string","description":"the plugin version, exactly as the lock should spell it"},
                "crate":{"type":"string","description":"the Rust crate that carries the plugin implementation; must be an identifier"},
                "checksum":{"type":"string","description":"the digest the plugin bytes must match, with or without a `sha256:` prefix"},
                "mode":{"type":"string","enum":["extension","replacement"],"description":"whether the plugin extends a slot or replaces the face in it"},
                "signature":{"type":"string","description":"optional provenance (ten-field spelling): the signature over the plugin bytes; naming any provenance column writes the ten-field form"},
                "fingerprint":{"type":"string","description":"optional provenance: the signing key's fingerprint"},
                "revocations":{"type":"string","description":"optional provenance: the revocation list the record was checked against"},
                "apply":{"type":"boolean","description":"false (the default) previews the exact bytes; true stores them"},
                "confirm":{"type":"boolean","description":"must be true to write: the lock is the artifact the host admits plugins from, so the request says it rather than the bridge assuming it"},
                "root":{"type":"string"}
            },"required":["source","framework","package","version","crate","checksum","mode"]}),
        ),
        tool(
            "nichlink.registry",
            "Report the registration faces this package declares, as the build sees them. A \
             **member that published records** under `target/nichlink/out` is answered from them — \
             the face rows `pruning_manifest.tsv` carries (`node`, `source`, the tracked symbol) \
             and the scope verdict `source_scope.tsv` carries — without re-deriving its tree, and \
             the reply opens with `tree published from <dir>` naming the `discovery.fingerprint` \
             and whether that output still describes these sources (`build current`, or `build \
             stale (run nichlink check)`). Those records carry no logical path and no `kind`, so a \
             member answered that way has no `root/...` column and the reply's last line says so: \
             `nichlink.explain` reports the derived per-face projection that has them. A member \
             that published nothing is derived now and says `tree derived now (no published records \
             at ...)` above the derived rows. Contract, admission, and registration-rule data need \
             the built snapshots and are not included. `root` is a package root; omitting it uses \
             NICH_LINK_PACKAGE_ROOT. A **virtual workspace root** (a manifest with no `[package]`) \
             names no package, so it is answered as the workspace: a census naming every member's \
             status — `published`, `not built` (no records, so its tree was derived now), `no faces` \
             (a framework crate declares none), or `unresolvable` with the reason — and, in the \
             **default** answer, one row per member with the faces it declares. The per-member \
             trees are what that default leaves out: pass `full: true` for one section per member \
             under that member's own package name, as the build sees it. Point `root` at one member \
             for that package's own answer, which is always the whole tree.",
            json!({"type":"object","properties":{
                "root":{"type":"string"},
                "full":{"type":"boolean","description":"print the face rows (default: the census — the count and how many sit at each level), one page at a time"},
                "offset":{"type":"integer","description":"with `full`: the first row of this page (default 0)"},
                "limit":{"type":"integer","minimum":1,"maximum":200,"description":"with `full`: rows per page (default 200, capped at 200)"}
            }}),
        ),
        tool(
            "nichlink.explain",
            "Report the build's own evidence for one face — identity, logical path, kind, source, \
             module, parent, registry_name — or the tree projection the build scoped. \
             `nichlink.registry` derives from source text and is therefore never `stale`; this reads \
             the files the build *published* under `target/nichlink/out`, so it answers what ships: \
             whether the scope selected the face and whether release pruning strips its symbols. \
             The `build` line says `current`, or `stale (run nichlink check)` when the published \
             output no longer describes these sources; an unbuilt project is told which command \
             publishes the evidence. `node` names one face by logical path or identity; omit it for \
             the projection, bounded by `limit`. With `overlay: true` it renders the *overlay* \
             projection instead — which slot each declared cut replaces, and which faces the scope \
             prunes — the published state after replacement, from the same traversal the CLI's \
             `explain --overlay` uses. That is a static projection and not `Registry::dump`: an \
             overlay needs two live registries, and the reply carries the note saying where the \
             live tree comes from. `overlay` and `node` are mutually exclusive. Declared graft \
             state stays in `nichlink grafts`; contract and admission fields need a loaded registry \
             and are not reported here. A **virtual workspace root** is answered as the workspace: \
             a census of every member with its status, then each member's own report under it, with \
             `tree unavailable (reason)` where a member could not answer the named face.",
            json!({"type":"object","properties":{"node":{"type":"string"},"overlay":{"type":"boolean"},"limit":{"type":"integer","minimum":1,"maximum":200},"root":{"type":"string"}}}),
        ),
        tool(
            "nichlink.diff",
            "Report the face-level delta between two sides of this package. By default the sources \
             now against the build's own manifest: which faces were added, which are gone, and which \
             changed identity under a file that did not move (a `kind` change is an identity change, \
             so only this comparison sees it). The unit is the face rather than the line, because the \
             face is what the registry ships. Needs a prior `nichlink check` or `build`; a project \
             with no build evidence is told so instead of being handed an empty diff. With \
             `records: true` the other pair: every external graft record against the sources, where a \
             record stores the identity it was written for, so a face that changed identity under an \
             unmoved slot breaks it silently. A record comes back `ok`; `undeclared` (the identity \
             is in the tree, but no `static_graft_plan!` cut names its slot — the release prunes \
             that slot, which is the same verdict `nichlink.grafts` gives and the same one the \
             build refuses); `stale` (nothing in the tree has that identity or that path); \
             `re-identified` (the path is there, the identity moved); or `unreadable` (a record \
             file that could not be read), which is counted rather than dropped. `undeclared` wins \
             over `stale`/`re-identified`, because whether a record can ever take effect is the \
             question that changes what a caller does. There is no separate bucket for a typed \
             cut: the plan's path is always a logical path (every writer uses `registry.path_for`), \
             and with the identity absent nothing resolves a typed declaration's module, so such a \
             record is judged by the declaration rule like any other — and the reply prints the \
             path it looked for rather than guessing. A **virtual workspace root** is answered as \
             the workspace: a census of every member with its status, then each member's own \
             comparison, so a member with no build evidence and a member that cannot be resolved \
             are told apart rather than merged into one empty diff. With `against`, the other side is a **second directory of published records** rather than the sources: the reply compares the two sets as data (scope identities, pruning rows, the discovery token) and names the sections it cannot compare because this tree has no reader for them.",
            json!({"type":"object","properties":{"records":{"type":"boolean"},"against":{"type":"string","description":"a second directory of published records: the reply compares the two sets of records as data instead of comparing this one against the sources"},"limit":{"type":"integer","minimum":1,"maximum":200},"root":{"type":"string"}}}),
        ),
        tool(
            "nichlink.trace",
            "Read this package's recorded trace artifact (`NICH_LINK_TRACE_FILE`, else \
             `<root>/.nichlink/traces/nichlink.trace`) and answer with the headless call report it \
             implies — what actually ran, which no static read can tell you. With `values: true` the \
             same read answers what that run *saw* instead: the recorded locals grouped by the frame \
             they belong to (name, type, rendered value, role, observed-or-inferred, callsite), the \
             locals recorded outside every traced call, and the observed data edges between them. A \
             local or edge naming something the artifact does not contain is counted rather than \
             dropped, so an incomplete artifact cannot read as a complete one. The artifact's identity \
             is checked first (namespace, registry root, every frame's node), and an artifact that \
             describes a different tree is refused by name rather than rendered, because frames are \
             node identities and foreign ones would draw a plausible, wrong call tree. Absence is \
             reported together with the way to produce one; `query` filters either report and a long \
             one is truncated with its total named. A **virtual workspace root** is answered as the \
             workspace: a census of every member with its status, then each member's own read of \
             its own trace artifact, with `tree unavailable (reason)` where a member could not \
             answer.",
            json!({"type":"object","properties":{"query":{"type":"string"},"values":{"type":"boolean"},"root":{"type":"string"}}}),
        ),
        tool(
            "nichlink.mir",
            "Read a MIR artifact and report the compiler's call candidates, emit it as canonical \
             JSONL, or report the call-graph delta against another artifact. A `rustc \
             -Zunpretty=mir` text dump and the compact JSONL artifact are both accepted, chosen by \
             extension: JSONL parses strictly, so a malformed line fails the whole read, while a \
             text dump never fails because a line that is not a call is simply not a call. The JSONL \
             form is the portable channel Studio could already render and parse and nothing in this \
             workspace ever wrote — `jsonl: true` makes this tool that writer, and what it prints \
             reads back here. It prints only a snapshot it can print **whole**: a graph whose JSONL \
             would run past this bridge's reply cap is refused by name — the line count, the cap, and \
             the narrower dump that would fit are all named — instead of being printed short, because \
             a payload cut at the cap would read back here as a complete snapshot of a smaller graph. \
             What it writes is a *snapshot*: a header naming the identity namespace \
             and registry root of the tree the artifact came from, which is what makes two artifacts \
             comparable — a snapshot of another tree is refused by name, and an unidentified one (a \
             text dump can't name its tree) makes the delta say what it cannot rule out. With \
             `against`, that other artifact is the baseline and the reply is the call-graph delta \
             from it forward: added and gone relations, plus function symbols. The text producer \
             stays `cargo rustc -Zunpretty=mir` on a nightly toolchain; a missing artifact says \
             exactly that instead of reporting an empty graph. A **virtual workspace root** is \
             answered as the workspace: the member whose root contains the artifact path answers \
             it, from that member's own root, or the reply says no member owns the path.",
            json!({"type":"object","properties":{"path":{"type":"string"},"against":{"type":"string"},"against_trace":{"type":"string","description":"like `against`, but compares call chains read from a trace file instead of from the sources"},"jsonl":{"type":"boolean"},"limit":{"type":"integer","minimum":1,"maximum":200},"root":{"type":"string"}},"required":["path"]}),
        ),
        tool(
            "nichlink.unified",
            "Merge a MIR artifact with this package's recorded trace: the compiler's call candidates \
             together with what actually ran, where a live call confirms its candidate and carries \
             `evidence=Live` while the rest stay `evidence=Mir`. This is `debug_method`'s \
             `UnifiedCallGraph`, the one place the two evidence sources are joined, so the merge \
             cannot drift from the library's own. The trace is read from this package's artifact \
             path; when none has been recorded the merge still answers and labels every relation a \
             compiler candidate rather than failing, because an absent trace is a weaker answer and \
             not a broken one. A **virtual workspace root** is answered as the workspace: the \
             member whose root contains the artifact path answers it, from that member's own root, \
             or the reply says no member owns the path. With `against`, the two artifacts are compared rather than merged: the reply lists the relations only one side has and the ones **both** carry under **different** evidence (`Live` against a compiler candidate), which is the difference an edge-level delta cannot express. `against_trace` gives the baseline side its own trace artifact; without it both sides share this package's.",
            json!({"type":"object","properties":{"path":{"type":"string"},"against":{"type":"string","description":"a second MIR artifact: the reply compares the two chains at the evidence level instead of merging one"},"against_trace":{"type":"string","description":"a trace artifact for the baseline side; omit it to compare both chains under this package's own trace"},"limit":{"type":"integer","minimum":1,"maximum":200},"root":{"type":"string"}},"required":["path"]}),
        ),
        tool(
            "nichlink.grafts",
            "Every external graft plan under `.nichlink/external-grafts/<selector>/graft.plan` \
             joined with the host entry's `static_graft_plan!` declarations: the selector, the \
             logical path and replacement each plan targets, whether it covers the whole subtree, \
             and whether the host entry names that slot. A plan the entry does not declare is \
             reported as `NOT declared by the host entry` and counted under `unkept plans`: the \
             release prunes that slot, so the record can never take effect — the build warns about \
             it and a cargo log is where that warning goes to die. An unreadable plan carries its \
             reason rather than being skipped, and with no readable declaration the state is \
             `declaration unknown` rather than a false `not declared`. A declaration is about \
             the *slot* a plan targets, not the implementation it selects, so each row shows the \
             plan's own target and graft next to the declaration's cut and graft. Read-only. A \
             **virtual workspace root** is answered as the workspace: a census of every member \
             with its status (`published`, `not built`, `no faces`, or `unresolvable` with the \
             reason), then each member's own plans, since a plan is a record under one package's \
             `.nichlink/` and a declaration is judged against a logical path, which is a fact the \
             published record does not carry.",
            json!({"type":"object","properties":{"limit":{"type":"integer","minimum":1,"maximum":400},"root":{"type":"string"}}}),
        ),
        tool(
            "nichlink.impact",
            "The transitive blast radius of a change to one face, over the three dependency kinds \
             this tree actually declares: a face's descendants (its registry owns them), the faces \
             whose `requires` names a capability it provides, and the declared graft cuts that hand \
             it over. Each reached node carries its shortest hop distance and the chain of reasons \
             that got there, so a capability cycle is a shorter path rather than a hang. A face the \
             traversal did not reach is reported as unreached within `depth` — which is not proof of \
             independence — and graft records or recorded traces naming the same identity are not \
             traversed, which the reply states. A **virtual workspace root** is answered as the \
             workspace: a census of every member with its status, then each member's own radius \
             under it, with `tree unavailable (reason)` where a member could not answer the named \
             face — and a face no member declares is reported as such rather than as another \
             member's answer.",
            json!({"type":"object","properties":{"node":{"type":"string"},"depth":{"type":"integer","minimum":1,"maximum":16},"limit":{"type":"integer","minimum":1,"maximum":200},"root":{"type":"string"}},"required":["node"]}),
        ),
        tool(
            "nichlink.usages",
            "Report the neighbourhood of one face: its parent and children as the tree has them, every \
             field the write path accepts read back from the generated module (`module`, `preset`, \
             `parts`, `name_zh`, `name_en`, `summary_zh`, `summary_en`, `stable_name`, `exports`, \
             `requires`, `provides`, `handle_traits`, `handle_contracts`, `part_traits`, \
             `part_contracts`, `registration_rule`, `admission`, `flow`, `flow_provider`, \
             `runtime_checks`, `getting_from_other_registry`, `needs_registry`) — so every field \
             `nichlink.apply` can set becomes reportable — and which other faces mention the same \
             capability tokens. Capability matches are on declared tokens rather than a resolved graph, \
             and the reply says so. A hand-written module has no generated field list and the executor \
             refuses to invent one, so those faces are counted as unreadable rather than shown empty. \
             A package whose own faces the authoring connector refuses still gets its tree half — the \
             parent, the children and this face's identity come from the source derivation — with the \
             verdict printed and the field and capability halves named unavailable rather than shown \
             empty; that is the same verdict `nichlink.verify` prints on its connector line, and it is \
             an answer about the tree rather than a tool failure, so `isError` stays false. \
             Declared graft cuts are not reported here; the CLI's `explain --json` carries them. A \
             **virtual workspace root** is answered as the workspace: a census of every member \
             with its status, then each member's own neighbourhood under it, with `tree \
             unavailable (reason)` where a member could not answer the named face.",
            json!({"type":"object","properties":{"node":{"type":"string"},"limit":{"type":"integer","minimum":1,"maximum":200},"root":{"type":"string"}},"required":["node"]}),
        ),
        tool(
            "nichlink.converge",
            "The converged starting point for one face in a single answer, or for a recorded run. With \
             `node`, it returns the build's scope and pruning verdicts, the tree's edges, the declared \
             fields, and — the verdict no single tool can give — whether each \
             `capability=>ProviderKind` requirement is actually answered by something in the package, \
             named when it is and UNANSWERED when it is not; it ends with the files to read (this \
             face, its parent, its children) and which tool has the detail. With `trace: true` it \
             starts from the recorded run instead and collapses the whole tree to the files that both \
             declare a face and actually ran, with the frames that landed in each; frames are matched \
             to faces by source file, which the reply states, because a face is a declaration and a \
             frame is a function. Everything is composed from what the other tools report. A \
             **virtual workspace root** is answered as the workspace: a census of every member \
             with its status, then each member's own converged point under it, with `tree \
             unavailable (reason)` where a member could not answer the named face.",
            json!({"type":"object","properties":{"node":{"type":"string"},"trace":{"type":"boolean"},"limit":{"type":"integer","minimum":1,"maximum":200},"root":{"type":"string"}}}),
        ),
        tool(
            "nichlink.adopted",
            "Report this package's adoption ledger at `.nichlink/adopted/entries`: which routes \
             were adopted **provisionally**, and whether the bytes each decision was taken on are \
             still here. An entry reads `adopted since <at> at <fingerprint> (provisional)` while \
             its files are unchanged, and `adoption lapsed at <file>; needs confirmation` once one \
             of them moved or went missing — the file is named because that is where to look. \
             Reading never renews anything, and the word `verified` is deliberately absent: an \
             adoption is a lease whose honest level is `provisional`, and re-earning it takes a \
             person. With `anchor`, `certifies`, `evidence`, `verifier`, `reason` and `files` the \
             same tool **appends** one line — a preview unless the request also says `apply: true` \
             and `confirm: true` — so a confirmation is one more line rather than a rewrite. The \
             read also names the action for a route this ledger does not carry: a route the ledger \
             has never named is a **new anchor** and a first confirmation, not a renewal.",
            json!({"type":"object","properties":{"anchor":{"type":"string"},"certifies":{"type":"string"},"evidence":{"type":"string"},"verifier":{"type":"string"},"reason":{"type":"string"},"files":{"type":"array","items":{"type":"string"}},"apply":{"type":"boolean"},"confirm":{"type":"boolean"},"root":{"type":"string"}}}),
        ),
        tool(
            "nichlink.affected",
            "Which tests a set of changed files reaches: for each path, the definitions it holds and \
             the test files that call any of them, by the same source index every other tool reads. \
             This is the question a reader asks before running anything, and it is static — it names \
             what to run, never that a test fails. `files` is an array of workspace paths. A file no \
             test mentions is reported as such rather than as an empty list, because the smallest \
             honest answer there is the owning package's suite. The test-file rule is this \
             bridge's own convenience (a `tests/` directory, a `_tests.rs` sibling, or a file that \
             declares `#[test]` itself), not the conventions gate's placement rule, which the \
             bridge cannot use. A **virtual workspace root** answers from the member each path \
             lives in and prefixes the listed tests with that member's directory.",
            json!({"type":"object","properties":{"files":{"type":"array","items":{"type":"string"}},"root":{"type":"string"}},"required":["files"]}),
        ),
        tool(
            "nichlink.check",
            "Test this tree by **running** a face: `face` is `default`, `all`, or one feature name, \
             and it **may be omitted** — then it runs `default`. Naming a face is still what this \
             tool is *for*: the round measured 12 calls arriving without `face`, 6 of which left the \
             tool for a shell `cargo test` instead of naming one, while the two faces a question \
             usually needs often share one red — so the default is a convenience, not the point. \
             It is the one tool that runs a process (`cargo test` in the root), and it \
             exists because the alternative was measured: a defect compiled only under a \
             non-default feature cannot fail on the default face, so a green default run is not \
             evidence about that feature. `nichlink.status` says which faces exist; this says what \
             one of them did. **The reply's first line is the run's verdict** — `verdict  passed \
             (cargo exit 0)`, `verdict  failed (cargo exit 101)`, or `verdict  unknown (…)` for a run \
             that timed out, was signalled, or left no `test result:` line — so a reader that stops at \
             the top of the reply cannot read a failing face as green. That verdict is the run's, \
             **not the one-shot client's exit code**: `--call` exits `0` when the tool answered (this \
             tool answers even about a failing run), `1` when it refused, `2` on a usage error. The \
             full output goes to `target/nichlink/out/check-<face>.log`, and the rest of the reply is \
             the exact command, cargo's own exit code, the elapsed time, every `test result:` \
             line (capped, and the cap says so), the failing test names, and that path. A run that \
             reaches `timeout_ms` (default 900000, clamped to 1000–3600000) is reported as \
             **unknown**, never as a pass, and the reply says the direct child was killed and its \
             own children may survive. A log with no `test result:` line reports that nothing ran: \
             `0 passed` is not a pass. The reply then carries a **sampled whole-tree census** of \
             static facts for the open question — what else is wrong here — each column saying what \
             it does not cover: numeric constants that are re-spelled elsewhere or read nowhere \
             outside tests, production `pub fn` names no test writes down, the entry plan's own site \
             counts, and the production functions **no test can reach**, by a static walk from this \
             tree's test files along the same name-in-a-call-list rule the orphan view uses (five \
             rows and the withheld count, or `skipped (N functions over the limit)` when the tree is \
             larger than that walk was sized for). The sample keeps the first five rows and always \
             the lines that say what the census is not, and those lines are one-sentence **indexes** \
             there — each keeps what it does not cover and the phrase that stops an inventory \
             reading as a measurement. Pass `census: true` for the whole table: the full boundary \
             prose, every test-unreachable function instead of five, and that column broken down \
             **per directory** (`src/mcp 3 of 20 · …`) so \"which parts of this tree are the \
             unreachable ones in\" is one call rather than one per directory. That last column says \
             what it cannot see instead of calling \
             itself coverage: calls made through dynamic dispatch, function pointers, FFI or macro \
             expansion are invisible to it, a function reached only through a trait method or a \
             closure does not count, and matching is by name, so an unrelated same-named call \
             counts as reaching it. The sixth column is **branch-level**: the arms no execution can \
             enter **by construction** — an `if`/`else if` whose condition is the literal `false`, \
             and a `match` arm on a variant of an enum declared in this tree without `pub` and \
             without an attribute that could build a value, where no construction of that variant \
             is spelled anywhere in this tree. It is a static read of the source text and it says \
             so: a condition whose value depends on data — a field, a parameter, a comparison, a \
             `match` over a value — is **not judged at all**, so an arm no run has taken yet stays \
             invisible here; `false` is the only guard literal decided; macro expansion, dynamic \
             dispatch, function pointers and FFI are invisible, while a `macro_rules!` body this tree \
             writes **is** text — an `if false` inside one is listed, and when that body sits outside \
             any function its row names no function (there is none to name) — and an arm that only \
             exists after expansion is invisible; a construction this tree does not spell (a derive \
             that builds a value, `unsafe`, a consumer outside this scanned root) would falsify a row \
             rather than merely be missed by one; a `pub` enum is never judged, an arm reached \
             through a wildcard or a binding is not read, and an enum name a file imports from \
             another crate is **conservatively skipped**, so a same-named foreign enum's arms are a \
             miss here rather than a false row. Each row names the line it decided — the guard's own \
             line, or the arm's pattern line.",
            json!({"type":"object","properties":{
                "face":{"type":"string","description":"`default`, `all`, or one feature name; omit it to run `default` (naming a face is still the point, but the round measured 6 of 12 face-less calls leaving the tool for a shell)"},
                "timeout_ms":{"type":"integer","minimum":1000,"maximum":3600000,"description":"how long the run may take before it is reported as unknown (default 900000)"},
                "target_dir":{"type":"string","description":"`CARGO_TARGET_DIR` for this run — pass a directory OUTSIDE the tree; without it cargo writes `target/` inside the tree, and on a tree that was restored (sources older than the leftover artifacts) the next run reuses the old binary and reports a green that belongs to the previous code"},
                "census":{"type":"boolean","description":"print the whole census table instead of the sampled rows and the one-sentence boundary indexes; the whole table also names every test-unreachable function and splits that column per directory (default false)"},
                "verbose":{"type":"boolean","description":"print every `test result:` line, including the groups that passed (default: the failing ones line by line, the passing ones as a count and the first)"},
                "root":{"type":"string"}
            }}),
        ),
        tool(
            "nichlink.verify",
            "Run the kernel's registration validation over this package and report the tree delta the \
             run just published. It drives the same entry the CLI's `check` drives, so a verdict here \
             cannot drift from `nichlink check`, and it refreshes the build evidence as a side effect — \
             which is why the delta below it describes the tree that was just verified rather than the \
             last build. **Two verdicts are printed, because this package is judged on two surfaces.** \
             The first is the static one: it judges the faces the build ships, and a failure there is \
             the answer and not a tool failure — the reply says `verdict failed` with the diagnostics \
             (each naming its phase, node, source and line) while `isError` stays false, because the \
             verification itself succeeded. The second is a line of its own, `connector verdict: ok` or \
             `connector verdict: rejected`, from the authoring connector every face tool validates \
             through (`apply`, `usages`, `converge`): it registers the faces on disk rather than the \
             ones the build scoped, so a tree can pass the static line and be refused here. A rejection \
             is rendered the same way — the diagnostic names the field and the line — and `isError` \
             stays false for it too, because `rejected` is what this surface says about the tree rather \
             than a failure to ask. A **virtual workspace root** is answered as the workspace: a \
             census of every member with its status, then each member's own verdict and delta \
             under it, with `tree unavailable (reason)` where a member has no tree to verify.",
            json!({"type":"object","properties":{"limit":{"type":"integer","minimum":1,"maximum":200},"root":{"type":"string"}}}),
        ),
        tool(
            "nichlink.graph",
            crate::mcp::graph::DESCRIPTION,
            crate::mcp::graph::schema(),
        ),
        catalogue_entry(),
    ]
}

/// What `tools/list` advertises: the two entry points, plus the catalogue tool.
/// `tools/list` 广告的东西：两个入口，加上目录工具。
///
/// Audit `W1-1`: the client used to be handed **all twenty-seven** tools in one frame — around
/// 36,000 characters of prose — so a session's first decision was made against a wall of text, and
/// the round measured what that costs (25 steps against the compared tool's 14). Nothing is removed:
/// the catalogue still holds every tool, `--list <tool>` still prints a whole description, and this
/// meta-tool lists them all. What changed is **what arrives unasked** — the two calls the workflows
/// actually enter through, and one tool whose whole job is to show the rest on request.
/// 审计 `W1-1`：客户端过去在**一帧里**拿到**全部二十七个**工具——约 36,000 字符的散文——于是一个会话的
/// 第一个决定是在一堵文字墙前做出的，而那一轮量到了代价（25 步 vs 对照工具的 14 步）。什么都没有删：
/// 目录里仍然是每个工具，`--list <tool>` 仍然印完整描述，而这个元工具会把它们全列出来。变的是
/// **不请自来的东西**——工作流真正进入的那两个调用，以及一个"按需展示其余"就是它全部职责的工具。
pub(crate) fn advertised() -> Vec<Value> {
    vec![
        tool(
            "nichlink.check",
            ADVERTISED_CHECK,
            advertised_schema("nichlink.check"),
        ),
        tool(
            "nichlink.apply",
            ADVERTISED_APPLY,
            advertised_schema("nichlink.apply"),
        ),
        catalogue_entry(),
    ]
}

/// The catalogue tool's entry, shared by the catalogue and the advertisement.
/// 目录工具的条目，目录与广告共用。
///
/// One builder for both lists, because two copies of a description drift exactly the way the round
/// measured them drifting: what `tools/list` says and what `--list` says are the same sentence about
/// the same tool, or one of them is wrong.
/// 两份清单共用一个构造器，因为描述的两份拷贝会按那一轮量到的方式漂移：`tools/list` 说的与 `--list` 说的
/// 是关于同一个工具的同一句话，否则其中一句是错的。
fn catalogue_entry() -> Value {
    tool(
        "nichlink_tools",
        "Every tool here, one line each. No argument: name, keys, `*` = \
         required — the shape a session needs to pick one. `tool: \
         \"nichlink.consistency\"`: that tool's **whole** description and schema, which is what \
         `--list <tool>` prints. The advertised tools are where the workflows enter; everything else \
         is reached through this list, because the round measured a 36,000-character catalogue as \
         the first thing a session read and the last thing it used. \
         列出这个桥的整个能力面：不给参数 ⇒ 每个工具一行（名字、它接受的键、`*` = 必填）；\
         `tool: \"nichlink.consistency\"` ⇒ 那个工具的**完整**描述与 schema（即 `--list <tool>`）。",
        json!({"type":"object","properties":{
            "tool":{"type":"string","description":"a tool name from the no-argument listing; omit it to list them all"},
            "full":{"type":"boolean","description":"the whole description, not the bounded page"},
            "slim":{"type":"boolean","description":"with no `tool`: the thirteen entry tools"}
        }}),
    )
}

/// The advertised description of `nichlink.check` (audit `W1-4`: decision information only).
/// `nichlink.check` 的广告描述（审计 `W1-4`：只留决策信息）。
///
/// The full text — the census columns, the verdict states, the log path — is one `nichlink_tools`
/// call away, and it is still what `--list check` prints. What stays here is what decides **whether
/// to call it** and **what to pass**.
/// 全文——普查各栏、判定状态、日志路径——离一次 `nichlink_tools` 调用，而且仍是 `--list check` 印的东西。
/// 留在这里的是决定**要不要调它**与**传什么**的东西。
const ADVERTISED_CHECK: &str = "\
**Runs** `cargo test` on one face and puts the run's own verdict on the reply's first line \
(`verdict  passed (cargo exit 0)` / `failed (exit 101)`) — never this client's exit code. `face` is \
`default` (what omitting it runs), `all`, or a feature name: a defect compiled only under a \
non-default feature cannot fail on the default face. It also carries a sampled whole-tree census.";

const ADVERTISED_APPLY: &str = "\
**The only write path** for this tree's registration faces. Seven actions in two classes: \
`add`/`deepen`/`cut`/`promote` are **additive** (a hand-written face is fine), \
`edit`/`rename`/`delete` are **rewrites** (generated faces only). \
**Previewed unless `apply: true`**; `delete`/`promote` also need `confirm: true`.";

/// The slim schema of an advertised tool: the keys that decide the call, one line each.
/// 一个广告工具的瘦 schema：决定这次调用的那几个键，每个一行。
///
/// Audit `W1-4`: the apply parameter block alone was 2,140 characters of prose repeated in every
/// frame. What moved out is not deleted — `--list <tool>` and `nichlink_tools` still carry it — and
/// the refusals carry the *instantiated* form, which is the one a caller can actually run.
/// 审计 `W1-4`：仅 apply 的参数块就是 2,140 字符的散文，每一帧都重发一遍。搬走的不是删掉的——
/// `--list <tool>` 与 `nichlink_tools` 仍然带着它——而拒绝文案带的是**实例化**的那一份，也就是调用方
/// 真能跑的那一份。
fn advertised_schema(tool: &str) -> Value {
    match tool {
        "nichlink.check" => json!({"type":"object","properties":{
            "face":{"type":"string","description":"`default` (omitted), `all`, or a feature name"},
            "census":{"type":"boolean","description":"the whole table, not the sample"},
            "verbose":{"type":"boolean","description":"every `test result:` line, passing groups included"},
            "timeout_ms":{"type":"integer","description":"default 900000; a timeout is `unknown`"},
            "root":{"type":"string"}
        }}),
        "nichlink.apply" => json!({"type":"object","properties":{
            "action":{"type":"string","enum":["add","edit","rename","delete","deepen","cut","promote"],"description":"add/deepen/cut/promote: declarations; edit/rename/delete: generated files"},
            "node":{"type":"string","description":"edit/rename/delete/deepen: the face"},
            "parent":{"type":"string","description":"add: parent (default root)"},
            "fields":{"type":"object","description":"the face's fields; strings except `needs_registry`; `module` is bare snake_case","properties":{"needs_registry":{"type":"boolean"}},"additionalProperties":{"type":"string"}},
            "inside":{"type":"object","description":"deepen: the layer","properties":{"parts":{"type":"object","additionalProperties":{"type":"string"}}},"required":["parts"]},
            "cut":{"type":"string","description":"cut: the face handed over (`crate::…::NODE_ID`)"},
            "graft":{"type":"string","description":"cut: what replaces it"},
            "to":{"type":"string","description":"cut: range end"},
            "full":{"type":"boolean","description":"cut: covers the subtree"},
            "selector":{"type":"string","description":"promote: the record to land"},
            "implementation":{"type":"string","description":"promote: the crate a typed declaration names, when it is not a path dependency of this package"},
            "apply":{"type":"boolean","description":"false previews; true writes"},
            "full":{"type":"boolean","description":"print every face of the tree, not the census and the changed one"},
            "confirm":{"type":"boolean","description":"delete/promote: must be true"},
            "root":{"type":"string"}
        },"required":["action"]}),
        _ => json!({"type":"object"}),
    }
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    // `title` and `annotations` are built **here**, in the one constructor every entry goes
    // through, so an entry cannot be added without them — the MCP spec's `annotations` are how a
    // client learns which tools make destructive changes (`readOnlyHint`, `destructiveHint`,
    // `idempotentHint`, `openWorldHint`), and a hint that has to be remembered per entry is a
    // hint that goes missing. Verified: all four hints and a non-empty title on every entry.
    // `title` 与 `annotations` 就建在**这里**——每个条目都要经过的唯一构造器，因此新条目不可能漏掉它们：
    // MCP 规范的 `annotations` 是客户端据以知道"哪些工具会做破坏性改动"的东西（`readOnlyHint`、
    // `destructiveHint`、`idempotentHint`、`openWorldHint`），而一条要靠每条自己记得写的提示，就是一条
    // 会缺席的提示。已验：每个条目四个提示齐全、标题非空。
    json!({
        "name": name,
        "title": title(name),
        "description": description,
        "inputSchema": input_schema,
        "annotations": annotations(name),
    })
}

/// What a tool does to the tree, as `annotations` discloses it.
/// 一个工具对这棵树做了什么，按 `annotations` 披露的那样。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Effect {
    /// Reads only.
    /// 只读。
    Read,
    /// Adds something that was not there, and removes nothing.
    /// 增添原本不存在的东西，且不移除任何东西。
    Add,
    /// Rewrites or removes what is there.
    /// 改写或移除已有的东西。
    Rewrite,
}

/// The effect of one tool.
/// 一个工具的效果。
///
/// Three names write and the rest read, and this is the **one** place that says which: the
/// alternative is a hint per catalogue entry, which drifts from the dispatch the first time a
/// tool is added. `a_write_tool_is_the_one_the_write_path_knows` pins it against
/// `ownership`'s own classification, so the two cannot disagree silently.
/// 三个名字会写、其余只读，而**这里**是唯一说清楚是哪三个的地方：另一种做法是每条目录项各写一遍提示，
/// 而那样的东西会在第一次新增工具时与分派漂移。`a_write_tool_is_the_one_the_write_path_knows` 把它
/// 与 `ownership` 自己的分类钉在一起，因此两者不会无声地不一致。
pub(crate) fn effect(tool: &str) -> Effect {
    match tool {
        // `plugin` appends (and promotes) a catalog record: it adds, and removes nothing.
        // `plugin` 追加（并升级）一条目录记录：它只增添，不移除任何东西。
        "nichlink.new_project" | "nichlink.plugin" => Effect::Add,
        // `apply` can move a subtree into the trash (`delete`), which is the one action here
        // that takes something away from the caller.
        // `apply` 能把一棵子树移进回收目录（`delete`）——这是这里唯一一个把东西从调用方手里拿走的动作。
        "nichlink.apply" => Effect::Rewrite,
        _ => Effect::Read,
    }
}

/// The MCP annotations for one tool.
/// 一个工具的 MCP `annotations`。
///
/// `destructiveHint` is set **explicitly on every tool** rather than left to the protocol's
/// default (which is `true`): a read tool that omits it is telling a client the opposite of the
/// truth, and `apply` really does move subtrees into the trash, so it says `true` while
/// `new_project` says `false` — additive and destructive are not the same permission.
/// `destructiveHint` **每个工具都显式写出**，而不是留给协议默认值（默认是 `true`）：一个省略它的读工具
/// 等于对客户端说了反话；而 `apply` 确实会把子树移进回收目录，因此它写 `true`，`new_project` 写
/// `false`——"新增"与"破坏"不是同一种许可。
fn annotations(tool: &str) -> Value {
    let read_only = effect(tool) == Effect::Read;
    json!({
        "readOnlyHint": read_only,
        "destructiveHint": matches!(effect(tool), Effect::Rewrite),
        "idempotentHint": read_only,
        "openWorldHint": false,
    })
}

/// A human-readable title for one tool, derived from its name.
/// 一个工具的人类可读标题，由它的名字推导。
///
/// Derived rather than written per entry: a title is a display name, and a second list of
/// eighteen names is a second list that drifts.
/// 推导而不是逐条写：标题是显示名，而第二份十八个名字的清单就是一份会漂移的清单。
fn title(tool: &str) -> String {
    let bare = tool.strip_prefix("nichlink.").unwrap_or(tool);
    let mut spaced = bare.replace('_', " ");
    if let Some(first) = spaced.get(0..1) {
        spaced.replace_range(0..1, &first.to_uppercase());
    }
    spaced
}

/// One tool's implementation: the package root plus the `arguments` object.
/// 一个工具的实现：包根加上 `arguments` 对象。
type Handler = fn(&Path, &Value) -> Result<String, String>;

/// The dispatch table, in the catalog's order.
/// 分派表，顺序与目录一致。
///
/// This is one ordered table rather than a `match` because the two lists used to be
/// aligned only by hand: the catalog put `nichlink.apply` sixth while the dispatch put
/// it last, and adding a tool to one side alone was silent — the missing arm only
/// surfaced at call time as `unknown tool` (audit `BR-12`).
/// `tools_tests::the_dispatch_table_follows_the_catalog` pins this table's names and
/// their order against `tools()`, so a tool added to the catalog without a handler, or
/// in another position, fails the suite instead of shipping.
/// 这里用一张有序表而不是 `match`，因为两张名单过去只靠手工对齐：目录把 `nichlink.apply` 放在第 6，
/// 分派把它放在最后；而只往一侧新增工具是静默的——缺的那条臂只在调用时以 `unknown tool` 现身
/// （审计 `BR-12`）。`tools_tests::the_dispatch_table_follows_the_catalog` 用 `tools()` 钉住本表的
/// 名字与顺序：只往目录里加工具而漏掉处理函数、或把它放错位置，都会让测试失败而不是出厂。
const DISPATCH: &[(&str, Handler)] = &[
    ("nichlink.search", search),
    ("nichlink.digest", crate::mcp::digest::digest),
    ("nichlink.conformance", crate::mcp::adopted::conformance),
    ("nichlink.consistency", crate::mcp::consistency::consistency),
    ("nichlink.why", crate::mcp::why::why),
    ("nichlink.locate", crate::mcp::locate::locate),
    ("nichlink.inspect", inspect),
    ("nichlink.callgraph", callgraph),
    ("nichlink.read", read_source),
    ("nichlink.status", status_tool),
    ("nichlink.apply", apply),
    ("nichlink.new_project", new_project),
    ("nichlink.plugin", plugin),
    ("nichlink.registry", registry_tool),
    ("nichlink.explain", explain_tool),
    ("nichlink.diff", diff),
    ("nichlink.trace", trace),
    ("nichlink.mir", mir),
    ("nichlink.unified", unified),
    ("nichlink.grafts", grafts),
    ("nichlink.impact", impact),
    ("nichlink.usages", usages),
    ("nichlink.converge", converge),
    ("nichlink.adopted", adopted),
    ("nichlink.affected", affected),
    ("nichlink.check", check),
    ("nichlink.verify", verify),
    ("nichlink.graph", crate::mcp::graph::graph),
    ("nichlink_tools", catalogue),
];

/// `nichlink.status` reads no arguments, so it joins the table through a shim.
/// `nichlink.status` 不读参数，因此经一个转接函数进入分派表。
fn status_tool(root: &Path, _arguments: &Value) -> Result<String, String> {
    status(root)
}

/// `nichlink.registry` reads one argument: `full`, which buys the per-member trees a virtual root's
/// default answer leaves out.
/// `nichlink.registry` 只读一个参数：`full`，它买下虚拟根的默认答案省掉的那些逐成员树。
fn registry_tool(root: &Path, arguments: &Value) -> Result<String, String> {
    // The stage decides whether there is a tree to answer about at all, and it is checked here
    // because this is the entry both spellings share. Measured (T-12): on a directory with no
    // manifest this tool returned Cargo's own sentence about a missing file — naming the failure and
    // not the way to start — and this is one of the two calls an agent reaches for first.
    // 阶段决定这里到底有没有树可答，检查放在这里是因为它是两种拼法共用的入口。量到的（T-12）：在没有清单的
    // 目录上，这个工具回的是 Cargo 自己关于文件缺失的那句话——点名失败、不点名起步的方式——而它正是代理最先
    // 够到的两个调用之一。
    if let Some(entry) = crate::mcp::workspace::entry_point(crate::mcp::workspace::stage(root)) {
        // The line opens with `next` on purpose: `with_next_hint` adds the catalogue's own hint
        // only when the answer has none, and for a bare tree that hint ("explain one face's
        // contract") is about faces that do not exist yet.
        // 这一行有意以 `next` 开头：`with_next_hint` 只在答案里没有 hint 时才补目录里那条，而对一棵空树，
        // 那条（"解释某个面的契约"）说的是还不存在的面。
        return Ok(format!("no registration tree here yet\nnext   {entry}\n"));
    }
    if arguments.get("full").and_then(Value::as_bool) == Some(true) {
        // Audit `W2-1`: the rows are bought, and they arrive a page at a time so a 50,000-face tree
        // does not answer with a 5 MB list.
        // 审计 `W2-1`：行是买来的，而且一次一页，因此五万面的树不会回一份 5 MB 的清单。
        let offset = arguments.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize;
        let limit = arguments
            .get("limit")
            .and_then(Value::as_u64)
            .map_or(crate::mcp::registry::PAGE_ROWS, |value| {
                (value as usize).clamp(1, crate::mcp::registry::PAGE_ROWS)
            });
        crate::mcp::registry::registry_page(root, offset, limit)
    } else {
        registry_brief(root)
    }
}

/// `nichlink.explain` is the one name that routes to two implementations: `overlay:
/// true` renders the overlay projection, everything else the per-face report.
/// `nichlink.explain` 是唯一会分到两个实现的名字：`overlay: true` 渲染覆盖投影，其余走逐面报告。
fn explain_tool(root: &Path, arguments: &Value) -> Result<String, String> {
    if arguments.get("overlay").and_then(Value::as_bool) == Some(true) {
        overlay(root, arguments)
    } else {
        explain(root, arguments)
    }
}

/// Run one tool by name, the single place a name becomes a handler.
/// 按名跑一个工具——名字变成处理函数的唯一位置。
///
/// The stdio bridge and the one-shot client both come through here, so a tool cannot answer two
/// different ways depending on which client asked.
/// stdio 桥与一次性客户端都从这里进，因此同一个工具不会因为"谁问的"而答出两种样子。
pub(crate) fn run_tool(root: &Path, name: &str, arguments: &Value) -> Result<String, String> {
    crate::mcp::freshness::set_policy(crate::mcp::freshness::policy_from(arguments));
    let answer = match DISPATCH.iter().find(|(listed, _)| *listed == name) {
        Some((_, handler)) => crate::mcp::ownership::dispatch(root, name, arguments, *handler),
        None => Err(format!("unknown tool `{name}`")),
    };
    // W1.3: every read answer ends by naming the next call. The rounds measured that a separate
    // `--list` lookup is never made (0 calls in two rounds) while a next step written into the
    // answer is taken, so the hint rides here — the single place a name becomes a handler.
    // W1.3：每个读答案末尾点名"下一次调用"。两轮实测 `--list` 都是 0 次调用，而写进答案的下一步会被
    // 照做，因此这句话挂在这里 —— 名字变成处理函数的唯一位置。
    // T-23: the answer carries a line the caller can **quote** instead of re-running the call. The
    // measured reason is bookkeeping, not curiosity: the brief demands that every claim come with
    // the command, its raw output and its exit code, so the round's arm re-ran calls purely to have
    // something to cite — "I claimed a closing `status` call … I haven't run it. Let me run it", and
    // "I did NOT actually run explain … to make the claim true". Two calls spent on making a written
    // sentence true is a call the answer can save.
    // T-23：答案携带一行**可引用**的东西，让调用方不必为了引用而重跑。量出来的理由是记账而不是好奇：题面
    // 要求每条声称都给命令、原始输出与退出码，于是那一轮的臂**为了有东西可引**而补跑调用——原文「I claimed
    // a closing `status` call … I haven't run it. Let me run it」与「I did NOT actually run explain …
    // to make the claim true」。为让一句写下来的话成真而花掉两次调用，正是答案能替它省下的。
    //
    // The line is rendered **from the same `arguments` value the dispatch ran with**, so it cannot
    // describe a different call than the one that produced this answer; a refusal carries no line,
    // because the refusal text is its own evidence.
    // 这一行**由派发实际使用的那个 `arguments` 值**渲染，因此它不可能描述与产出本答案不同的另一次调用；
    // 被拒的调用不携带这一行——拒绝文案本身就是它的证据。
    answer.map(|text| with_evidence(name, arguments, with_next_hint(name, text)))
}

/// The quotable line: this call, and that it answered.
/// 可引用的那一行：这次调用，以及它作答了。
fn with_evidence(name: &str, arguments: &Value, text: String) -> String {
    const BUDGET: usize = 120;
    let mut request = arguments.to_string();
    if request.chars().count() > BUDGET {
        request = request.chars().take(BUDGET).collect::<String>() + "…";
    }
    let line = format!("evidence {name} {request} → exit 0");
    if text.ends_with('\n') {
        format!("{text}{line}\n")
    } else {
        format!("{text}\n{line}\n")
    }
}

/// Append the next-call hint, unless the answer already names one.
/// 追加"下一次调用"的提示，除非答案里已经点名了一次。
fn with_next_hint(name: &str, text: String) -> String {
    match next_hint(name) {
        Some(hint) if !text.lines().any(|line| line.starts_with("next")) => {
            if text.ends_with('\n') {
                format!("{text}{hint}")
            } else {
                format!("{text}\n{hint}")
            }
        }
        _ => text,
    }
}

/// The next call a read tool's answer should name, when it has one.
/// 一个读工具的答案应当点名的那次调用；没有就什么都不加。
fn next_hint(name: &str) -> Option<&'static str> {
    let hint = match name {
        "nichlink.status" => {
            "next   `registry` lists the tree, `check` runs one face (`face` defaults to `default`; pass \
             `all` or a feature name for another). On a tree with **no registered face**, \
             \"the object\" means a *symbol*: go `search {query}` / `locate {symptom}` / \
             `read {path, line}` first\n"
        }
        "nichlink.registry" => {
            // A tree with no registered face is the case the round measured: `registry` printed
            // `0 face(s) (no faces)` while this line still sent the reader to `explain {node}`, and
            // four steps and 37 KB went into working out what "the object" could mean. The line now
            // covers that branch itself.
            // 没有任何注册面的树就是那一轮量到的情形：`registry` 印 `0 face(s) (no faces)`，而这一行
            // 仍把读者送去 `explain {node}`，四步与 37 KB 花在猜"对象"指什么。这一行现在自己覆盖那一支。
            "next   `explain {node}` for one face's contract, `check {face}` for whether it builds; \
             on a tree with **no registered face** `explain` can only say `no such node` — there \
             \"the object\" means a *symbol*, so go `search {query}` / `locate {symptom}` / \
             `read {path, line}` first\n"
        }
        "nichlink.explain" => {
            "next   `callgraph {function}` for its callers and callees, `read {path, line}` for the body\n"
        }
        "nichlink.callgraph" => {
            "next   `read {path, line}` for a body, `affected {files}` for what depends on it\n"
        }
        "nichlink.inspect" => {
            "next   `callgraph {function}` for its callers, `read {path, line}` for the body\n"
        }
        "nichlink.affected" => {
            "next   `check {face}` to run the tests this change touches, `read {path, line}` for a line\n"
        }
        _ => return None,
    };
    Some(hint)
}

/// The closure line: whether this answer is everything, and the calls left when it is not.
/// 闭合行：这个答案是不是全部；不是时还剩下哪些调用。
///
/// Audit `W4-5` measured both failure directions on the debug battery: an answer that stopped at a
/// deviation and said nothing about the rest left the agent guessing whether to keep going, and an
/// answer that always appended a `next` line made every answer look unfinished. So this is stated as
/// one of exactly two things, and it is judged **conservatively** — `closed` is written only when
/// every question the answer's own shape raises has been answered; anything the tool could not read,
/// or a deviation it named, is `open` with the call that resolves it.
/// 审计 `W4-5` 在 debug 电池上量到了两个方向的失败：一个停在一处偏离、对其余只字不提的答案，让代理猜
/// 要不要继续；而一个总是追加 `next` 行的答案，让每个答案看起来都没做完。因此这里只说两种之一，而且判定
/// **从保守**——只有"这个答案自己的形状会提出的每个问题都已被回答"时才写 `closed`；任何工具读不了的
/// 东西、或它点名的一处偏离，都是 `open` 并附上解决它的那次调用。
pub(crate) fn closure(closed: bool, remaining: &[String]) -> String {
    if closed {
        return "closure    closed — nothing this answer leaves open\n".to_owned();
    }
    let mut text = String::from("closure    open\n");
    if remaining.is_empty() {
        // An `open` with nothing to do would be the worst of both: it says "unfinished" and gives
        // no call. The invariant is checked rather than assumed, because a caller that forgets to
        // collect a command is exactly how that shape would ship.
        // 一个没有下文的 `open` 是两者中最糟的：它说"没做完"，又不给调用。这条不变量是**检查**出来的而
        // 不是假设的，因为"调用方忘了收集一条命令"正是那种形状出厂的方式。
        text.push_str("remaining  (nothing to run — this answer's own gaps are named above)\n");
        return text;
    }
    for command in remaining {
        text.push_str(&format!("remaining  {command}\n"));
    }
    text
}

/// The catalogue tool: every tool on one line, or one tool's whole page.
/// 目录工具：所有工具一行式列出，或某一个工具的整页。
///
/// It reads the **same** catalogue and prints the **same** one-liner `--list` does
/// (`client::list_tool_lines`) and the **same** page `--list <tool>` does (`client::describe_tool`),
/// so the advertisement and the manual cannot drift: there is one implementation of each.
/// 它读的是**同一份**目录、印的是 `--list` 印的那张**同一份**一行式清单（`client::list_tool_lines`）与
/// `--list <tool>` 印的**同一页**（`client::describe_tool`），因此广告与手册不可能漂移：各自只有一份实现。
fn catalogue(_root: &Path, arguments: &Value) -> Result<String, String> {
    let flag = |key: &str| arguments.get(key).and_then(Value::as_bool) == Some(true);
    let Some(name) = arguments.get("tool").and_then(Value::as_str) else {
        return Ok(if flag("slim") {
            crate::mcp::client::slim_tools_page()
        } else {
            crate::mcp::client::list_tool_lines().join("\n") + "\n"
        });
    };
    // `full: true` prints the whole description (audit `W2-6`); the default page is bounded and says
    // so, so the manual is requested rather than paid for by every session.
    // `full: true` 印完整描述（审计 `W2-6`）；默认页有界并且自己说出来，因此手册是被**索取**的，而不是
    // 每个会话都为它付费。
    let page = if flag("full") {
        crate::mcp::client::describe_tool_fully(name)
    } else {
        crate::mcp::client::describe_tool(name)
    };
    page.ok_or_else(|| {
        format!(
            "`{name}` is not a tool this bridge has; call `nichlink_tools` with no `tool` to list \
             them, or `--list` for the same list (plus the workflow table)"
        )
    })
}

pub(crate) fn tool_call(root: &Path, id: Value, params: &Value) -> Value {
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return error_response(id, -32602, "tools/call requires name".to_owned());
    };
    let arguments = params.get("arguments").unwrap_or(&Value::Null);
    let requested_root = arguments.get("root").and_then(Value::as_str);
    let root = match resolve_root(root, requested_root) {
        Ok(root) => root,
        Err(error) => {
            return success(
                id,
                json!({ "content": [{"type":"text","text":error}], "isError": true }),
            );
        }
    };
    // The `freshness` argument is a request-level policy: it decides how much this one call is
    // willing to pay for a content verification. Setting it here — the single place every tool
    // enters through — is what keeps a body from having to remember it.
    // `freshness` 参数是**请求级**策略：它决定这一次调用愿意为内容核验付多少。在这里设置它——每个
    // 工具进入的唯一位置——就不必让每个主体自己记着这件事。
    crate::mcp::freshness::set_policy(crate::mcp::freshness::policy_from(arguments));
    let result = run_tool(&root, name, arguments);
    match result {
        Ok(value) => success(
            id,
            json!({ "content": [{"type":"text","text":value}], "isError": false }),
        ),
        Err(error) => success(
            id,
            json!({ "content": [{"type":"text","text":error}], "isError": true }),
        ),
    }
}

fn inspect(root: &Path, arguments: &Value) -> Result<String, String> {
    let relative = required_path(arguments)?;
    let file = load_one(root, &relative)?;
    let mut output = format!("file {}\n", file.relative);
    for function in &file.functions {
        output.push_str(&format!(
            "fn {} lines {}-{} calls=[{}]\n",
            function.name,
            function.line,
            function.end_line,
            function.calls.join(", ")
        ));
    }
    let registrations = nichlink_kernel::source::registration_kinds(&file.source);
    if !registrations.is_empty() {
        output.push_str("registrations: ");
        output.push_str(&registrations.join(", "));
        output.push('\n');
    }
    if file.functions.is_empty() && registrations.is_empty() {
        output.push_str("no function or registration declaration found\n");
    }
    Ok(output)
}

fn status(root: &Path) -> Result<String, String> {
    let files = load_sources(root)?;
    let functions = files.iter().map(|file| file.functions.len()).sum::<usize>();
    // The census says how much tree was read; the faces say which face a red run would appear on.
    // They belong together because the blindness they describe is a fact about *this* tree: a file
    // compiled only under a non-default feature cannot fail on the default face, and a caller who
    // has just counted the files is the one about to run the tests.
    // 普查说读到了多少树；特性面说红会出现在哪个面。它们该在一起，因为它们描述的那种失明是关于**这棵**
    // 树的事实：只在非默认特性下编译的文件在默认面上不可能失败，而刚数完文件的调用方正是要去跑测试的人。
    let mut lines = vec![format!(
        "root {}\nrust_files={} functions={} tool=nichlink-toolchain",
        root.display(),
        files.len(),
        functions
    )];
    lines.extend(crate::mcp::faces::faces_lines(root));
    // The entry point for the stage this tree is actually in. Measured (T-12): on an empty directory
    // the answers named the failure and never the way to start, so the flow table's first shape was
    // in the prose and not in the product.
    // 这棵树**实际所处阶段**的入口。量到的（T-12）：在空目录上答案点名了失败、从不点名起步的方式，于是流程表
    // 的第一个形状只在散文里、不在产物里。
    let stage = crate::mcp::workspace::stage(root);
    if let Some(entry) = crate::mcp::workspace::entry_point(stage) {
        lines.push(format!("next   {entry}"));
    }
    Ok(lines.join("\n"))
}

#[cfg(test)]
#[path = "tools_tests.rs"]
pub(crate) mod tools_tests;
