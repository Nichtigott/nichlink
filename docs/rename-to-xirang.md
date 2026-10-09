# Renaming to XiRang / 更名为 XiRang

English: from 2026-10-09 the project is named **XiRang**（息壤）; the published crates are
`xirang-kernel`, `xirang-macro` and `xirang-toolchain`, and the binaries are `xirang`,
`cargo-xirang`, `xirang-mcp`, `xirang-studio` and `xirang-dev`. Everything published under the
old name (`nichlink-*`) keeps its history on the index; the new line starts at `0.1.0`.

**Migration for an existing tree**: rename the state directory (`mv .nichlink .xirang`), then
re-run `check` and re-record. Identities change, because `NodeId = hash(namespace, relative
source path, name)` and the namespace is the package name, so graft plans, the adoption ledger
and external plans written under the old name no longer resolve. Documents written before the
rename have been updated to the new name; where they name a **published artifact of version
`0.2.2` or earlier**, its real name is `nichlink-*`.

中文：自 2026-10-09 起项目名为 **XiRang**（息壤）；发布的 crate 是 `xirang-kernel`、
`xirang-macro`、`xirang-toolchain`，二进制是 `xirang`、`cargo-xirang`、`xirang-mcp`、
`xirang-studio`、`xirang-dev`。旧名字（`nichlink-*`）已发布的内容在 index 上保留其历史，
新线从 `0.1.0` 开始。**已有项目的迁移**：把状态目录改名（`mv .nichlink .xirang`），再重跑
`check` 并重新登记。**身份会变** —— `NodeId = hash(命名空间, 包内相对路径, 名字)` 而命名空间就是
包名，因此旧名字下写的 graft 计划、采信台账与外部计划不再解析。更名前的文档已统一改成新名字；
凡其中点名 **`0.2.2` 及更早的已发布产物**处，其真实名字是 `nichlink-*`。
