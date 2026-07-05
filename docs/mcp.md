# Achuk MCP — wire the code-as-database into any AI coding tool

`achuk-mcp` is a Model Context Protocol server (stdio transport) over a Achuk
CDB. Any MCP client gets five tools:

| tool | what it answers |
|---|---|
| `achuk_symbols` | every definition that actually exists (`name : type`) |
| `achuk_candidates` | type-directed search: "what in scope has this type?" |
| `achuk_mask` | the legal-symbol set + GBNF grammar for constrained decoding |
| `achuk_render` | a definition rendered as `.achuk` source |
| `achuk_check` | typecheck Def-JSON with the REAL compiler (needs `achukc`) |

This is the anti-hallucination loop: an agent asks `achuk_candidates` before
writing a call, and verifies with `achuk_check` after — instead of inventing
an API and finding out at review time.

It's the same loop the bundled model uses: `achuk ai gen` prompts from the
same CDB, constrains decoding with the same grammar `achuk_mask` serves, and
verifies with the same real compiler as `achuk_check`. MCP hands that loop
to *your* agent. And because `achuk add` ingests a package's published defs
into the project CDB, these tools answer over installed packages too.

## Build / locate the binary

```sh
cargo build --release --bin achuk-mcp        # → target/release/achuk-mcp
```

Point it at your project's CDB (created by `achuk index`):

```sh
achuk-mcp --db /path/to/project/achuk.cdb
```

`achuk_check` runs the vendored compiler: put `achukc` on PATH or set
`ACHUK_CLAWC=/path/to/achukc` in the server's env.

Below, replace `/abs/path/to/` with your actual paths. Every client speaks
the same stdio protocol — only the config file differs.

## Claude Code

Inside a Achuk project, one command does everything (writes `.mcp.json`,
locates `achuk-mcp`, indexes the project):

```sh
achuk mcp install
```

Or by hand:

```sh
claude mcp add achuk -- /abs/path/to/achuk-mcp --db /abs/path/to/achuk.cdb
```

Or per-project `.mcp.json`:

```json
{
  "mcpServers": {
    "achuk": {
      "command": "/abs/path/to/achuk-mcp",
      "args": ["--db", "achuk.cdb"],
      "env": { "ACHUK_CLAWC": "/abs/path/to/achukc" }
    }
  }
}
```

## Claude Desktop

`~/Library/Application Support/Claude/claude_desktop_config.json` (macOS) or
`%APPDATA%\Claude\claude_desktop_config.json` (Windows):

```json
{
  "mcpServers": {
    "achuk": {
      "command": "/abs/path/to/achuk-mcp",
      "args": ["--db", "/abs/path/to/achuk.cdb"],
      "env": { "ACHUK_CLAWC": "/abs/path/to/achukc" }
    }
  }
}
```

## Cursor

`.cursor/mcp.json` in the project (or `~/.cursor/mcp.json` globally):

```json
{
  "mcpServers": {
    "achuk": {
      "command": "/abs/path/to/achuk-mcp",
      "args": ["--db", "achuk.cdb"],
      "env": { "ACHUK_CLAWC": "/abs/path/to/achukc" }
    }
  }
}
```

## Windsurf

`~/.codeium/windsurf/mcp_config.json`:

```json
{
  "mcpServers": {
    "achuk": {
      "command": "/abs/path/to/achuk-mcp",
      "args": ["--db", "/abs/path/to/achuk.cdb"]
    }
  }
}
```

## VS Code (GitHub Copilot agent mode)

`.vscode/mcp.json`:

```json
{
  "servers": {
    "achuk": {
      "type": "stdio",
      "command": "/abs/path/to/achuk-mcp",
      "args": ["--db", "achuk.cdb"]
    }
  }
}
```

## Zed

`settings.json` → `context_servers`:

```json
{
  "context_servers": {
    "achuk": {
      "source": "custom",
      "command": "/abs/path/to/achuk-mcp",
      "args": ["--db", "/abs/path/to/achuk.cdb"]
    }
  }
}
```

## Gemini CLI

`~/.gemini/settings.json`:

```json
{
  "mcpServers": {
    "achuk": {
      "command": "/abs/path/to/achuk-mcp",
      "args": ["--db", "/abs/path/to/achuk.cdb"]
    }
  }
}
```

## Codex CLI

`~/.codex/config.toml`:

```toml
[mcp_servers.achuk]
command = "/abs/path/to/achuk-mcp"
args = ["--db", "/abs/path/to/achuk.cdb"]

[mcp_servers.achuk.env]
ACHUK_CLAWC = "/abs/path/to/achukc"
```

## Cline / Continue / anything else

Any MCP client that can spawn a stdio server works with the same three
fields: command `achuk-mcp`, args `["--db", "<path>"]`, optional env
`ACHUK_CLAWC`. There is no HTTP transport yet — file an issue if you need
one.

## Smoke test

```sh
printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | achuk-mcp --db achuk.cdb
```

should list the five tools. In your client, ask the agent: *"use
achuk_symbols to list what exists, then achuk_check this definition"* — if
both round-trip, the loop is closed.
