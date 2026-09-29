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

use serde_json::{Value, json};
use std::path::Path;

use crate::mcp::adopted::adopted;
use crate::mcp::affected::affected;
use crate::mcp::apply::apply;
use crate::mcp::build_evidence::explain;
use crate::mcp::callgraph::callgraph;
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
use crate::mcp::registry::registry;
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
            json!({"type":"object","properties":{"query":{"type":"string","description":"a name: face, file or function"},"literal":{"type":"string","description":"text to find verbatim anywhere in a source file (raw bytes, case-sensitive, comments and string literals included)"},"converge":{"type":"boolean","description":"answer in layers: the tree's verdicts, then the call chain this name leads into, then the next step and the bounds of what was looked at"},"root":{"type":"string"},"limit":{"type":"integer","minimum":1,"maximum":200}},"anyOf":[{"required":["query"]},{"required":["literal"]}]}),
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
             were withheld, and `path` selects one definition when several share the name. A \
             **virtual workspace root** is answered as the workspace: a named `path` is answered \
             by the member that owns it, and without one every member is asked and its answer \
             grouped under it, with `tree unavailable (reason)` where a member could not answer.",
            json!({"type":"object","properties":{"function":{"type":"string"},"path":{"type":"string"},"limit":{"type":"integer","minimum":1,"maximum":50},"source":{"type":"boolean","description":"also print each definition's own lines (capped, declared through the truncation outlet)"},"root":{"type":"string"}},"required":["function"]}),
        ),
        tool(
            "nichlink.read",
            "Read a bounded source window around a line, an explicit `lines` range, or the \
             `whole` file. **Every header states the file's total**, as `path:START-END (N \
             lines)`: `N` is the file's own line count, not the printed range, so a windowed \
             read still says how much of the file is left — which is the fact the old \
             81-line window withheld, forcing six calls for a 473-line file. Default: a \
             window of `context` lines either side of `line` (context 40, at most 120, \
             240 lines). `whole: true` prints the file; `lines: \"120-260\"` prints that \
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
                "context":{"type":"integer","minimum":0,"maximum":120,"description":"window: lines either side of `line` (default 40)"},
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
             wrote. Every reply is the tree that results, so the next call can be aimed with it. A \
             **virtual workspace root** names no package and no unique owner, so a write there is \
             refused with the candidate member directories and nothing is copied or written — a \
             write runs against a member it is the unique owner of, or it does not run. **`fields.module` is a bare snake_case module name** (`widget`), not the logical path the tree reports: the executor names the module's directory and file, so `control::object::widget` is refused by name. Every other `fields` value is a **string**: `exports` is one export per call rather than an array, `requires` entries are spelled `capability=>provider`, and `handle_traits` entries are the **labels** the registration rule checks (a Rust path such as `crate::control::ControlHandle` is accepted here and only the parent rule refuses it later). The one non-string key is `needs_registry`, a boolean. **A request the kernel refuses comes back as an error** (`isError` true), because a request this tool cannot carry out is a tool failure rather than a fact about the tree; the read tools are the other way round and print their verdict in the body.",
            json!({"type":"object","properties":{
                "action":{"type":"string","enum":["add","edit","rename","delete"]},
                "node":{"type":"string","description":"edit/rename/delete: the face, by logical path or identity"},
                "parent":{"type":"string","description":"add: the parent's logical path or identity; defaults to the registry root"},
                "fields":{"type":"object","description":"the face's fields; edit and rename change only the keys given, add takes the rest as defaults. Values are strings unless noted: `module` is a bare snake_case name, `exports` one export per call, `requires` entries `capability=>provider`, `handle_traits` entries rule labels","properties":{"needs_registry":{"type":"boolean","description":"the one non-string field"}},"additionalProperties":{"type":"string"}},
                "apply":{"type":"boolean","description":"false (the default) previews on a copy; true writes to the project"},
                "confirm":{"type":"boolean","description":"delete: must be true. A delete is the one operation whose preview a caller can step past by accident, so the request says it rather than the bridge adding it"},
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
             `package` is the crate name and `kind` is `binary` or `library`. **A request is \
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
                "apply":{"type":"boolean","description":"false (the default) previews in a throwaway directory; true writes the project"},
                "confirm":{"type":"boolean","description":"must be true when the destination directory already exists: the write lands in a directory the caller already has, so the request says it rather than the bridge assuming it"},
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
             names no package, so it is answered as the workspace: one section per member under \
             that member's own package name, with a census naming every member's status above them \
             — `published`, `not built` (no records, so its tree was derived now), `no faces` (a \
             framework crate declares none), or `unresolvable` with the reason. Point `root` at \
             one member for that package's own answer.",
            json!({"type":"object","properties":{"root":{"type":"string"}}}),
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
            json!({"type":"object","properties":{"path":{"type":"string"},"against":{"type":"string"},"jsonl":{"type":"boolean"},"limit":{"type":"integer","minimum":1,"maximum":200},"root":{"type":"string"}},"required":["path"]}),
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
             and `confirm: true` — so a confirmation is one more line rather than a rewrite.",
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
    ]
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({ "name": name, "description": description, "inputSchema": input_schema })
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
    ("nichlink.verify", verify),
];

/// `nichlink.status` reads no arguments, so it joins the table through a shim.
/// `nichlink.status` 不读参数，因此经一个转接函数进入分派表。
fn status_tool(root: &Path, _arguments: &Value) -> Result<String, String> {
    status(root)
}

/// `nichlink.registry` reads no arguments either (its `root` is resolved above).
/// `nichlink.registry` 同样不读参数（它的 `root` 已在上面解析）。
fn registry_tool(root: &Path, _arguments: &Value) -> Result<String, String> {
    registry(root)
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
    let result = match DISPATCH.iter().find(|(listed, _)| *listed == name) {
        Some((_, handler)) => crate::mcp::ownership::dispatch(&root, name, arguments, *handler),
        None => Err(format!("unknown tool `{name}`")),
    };
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
    Ok(format!(
        "root {}\nrust_files={} functions={} tool=nichlink-toolchain",
        root.display(),
        files.len(),
        functions
    ))
}

#[cfg(test)]
#[path = "tools_tests.rs"]
mod tools_tests;
