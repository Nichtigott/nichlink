# nichlink-cli

`nichlink-cli` is the single entry point for NichLink tooling.

```sh
cargo install --git https://github.com/Nichtigott/nichlink nichlink-cli
nichlink new my-app
cd my-app && nichlink studio
```

## Commands

| Command | Action |
| --- | --- |
| `nichlink new <name> [--lib] [--path <workspace> \| --git <url>]` | Scaffold a NichLink host project in `./<name>` |
| `nichlink check [path]` | Run the registration discovery and validation pass without compiling |
| `nichlink build [path] [cargo options]` | Validate the registration tree, then run `cargo build` |
| `nichlink studio` | Launch the Studio TUI for the current project |
| `nichlink mcp` | Run the read-only MCP stdio bridge |

Dependency source is detected automatically: a CLI running from a NichLink
checkout writes path dependencies; an installed CLI writes Git dependencies
(with a version floor, so Cargo resolves crates.io once published). Override
with `--path` or `--git`.

The `cargo-nichlink` binary in the same package registers the plugin form:
`cargo nichlink studio` is equivalent to `nichlink studio`.

The library target (`nichlink_cli`) holds the command dispatch so other
binaries can reuse it.

简体中文见 [README.zh-CN.md](README.zh-CN.md)。
