# Independent verification of f7ddbb0 — only explain/verify expand (auditor, t7)

**Method.** `before` = `git archive f7ddbb0^` tree, `after` = `git archive f7ddbb0` tree, each built with its own target dir (md5 `ee2d75d3…` / `3d5661a9…`); the f7ddbb0 tree is byte-identical to the working tree, and no repository source was touched (`git status` clean). Fixture `target/trim-fixture/fixture`, **build artifacts reset before every call** (`rm -rf crates/*/target target/xirang`) so both binaries answer over the same pristine tree. Raw stdout per call: `target/trim-fixture/t6-raw/{before,after}-<tool>.txt`; driver `target/trim-fixture/t6-measure.py`.

| tool | before | after | verdict |
| --- | --- | --- | --- |
| explain | 1222 ch (1226 B) | **1546 ch (1550 B)** | +324, expanded |
| verify | 1888 ch (1892 B) | **2212 ch (2216 B)** | +324, expanded |
| read | 1389 ch (1425 B) | 1389 ch (1425 B) | `cmp` identical |
| registry | 241 ch (243 B) | 241 ch (243 B) | `cmp` identical |
| check --face default | 2347 ch (2349 B) | 2347 ch (2349 B) | `cmp` identical |

`diff` of the explain outputs (and of the verify outputs) is exactly two lines: the member rows gain `; not built (cannot read …/crates/core/target/xirang/out/source_scope.tsv: No such file or directory (os error 2))` and the same for `crates/report` (2 × 162 = +324 ch); no body line moved. **`read` is not larger ⇒ `resolve_owner` stayed terse.**

**Source face.** `git show f7ddbb0 -- workspace.rs` with comment lines stripped is a one-line diff: `fn roster_expanded(…)` → `pub(crate) fn roster_expanded(…)` — visibility only, body untouched. `ownership.rs` call sites in the after tree: `workspace::roster_expanded` once, at `every_member` (line 470); `workspace::roster` still three times — `dispatch` 177, `resolve_owner` 253, `refused_write` 561 — each carrying the terse comment. Behaviourally: `inspect` byte-identical before/after, and the write refusal (`apply` on the virtual root) still prints terse rows (`not built`, zero rows with a reason) with its red `REFUSED:` shape line intact.

**Second row shape.** On `target/trim-fixture/t6-nosrc` (copy with `crates/report/src` removed) after prints `unresolvable  ledger-report  tree unavailable (no source tree at …/crates/report/src)`, where before printed the terse `tree unavailable` — both expanded shapes are live.

**Red lines, really called on the after binary.** `no matches in …/target/trim-fixture/fixture — names only; … pass \`literal\`` ✓ · `check`: 2× `not a coverage measurement` + `not covered by the test-reachability column:` + `not covered: this census reads exactly what the columns above name; …` ✓ · `search --literal e`: `… truncated: 130 of 170 lines withheld at the limit of 40; raise \`limit\`…` ✓ · member-root `read --whole true --lines 1-6`: `` `whole` reads the file and `lines` reads a range; ask for one of them `` exit 1 ✓ · ledger `target/trim-fixture/ledger`: `next   a route this ledger does not name is a **new anchor** — a first confirmation, not a renewal: …` (1×; `adopted` byte-identical before/after) ✓.

**Gates, all rc=0.** build `-p xirang-toolchain --features mcp --bin xirang-mcp`; `cargo test … --lib -- mcp::ownership mcp::workspace` = 20 passed / 0 failed / 401 filtered; `cargo fmt --all -- --check`; `cargo test --workspace --offline` = 36 result lines / 676 passed / 0 failed; `cargo clippy --workspace --all-targets --offline` and `… --all-features` both `-D warnings` clean; `tools/xirang-publish --check-table` → `dependency table matches the manifests (3 crates)`.

**Verdicts.** ① self-built before/after + reset-per-call + per-tool chars: **pass** · ② only explain/verify grow, read/registry/check byte-identical: **pass** · ③ workspace.rs only the visibility word, ownership.rs only `every_member` switched: **pass** · ④ both expanded row shapes and all six red lines still present: **pass** · ⑤ gates green: **pass** · ⑥ report here and no repo source changed: **pass**. Nothing failed or was undecidable. Note, not a finding: my absolute counts are ~23 chars above the commit message's (1203→1527) — same +324 delta, different fixture path length; only my own readings are reported.
