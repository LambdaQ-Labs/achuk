# Achuk for VS Code

Syntax highlighting, snippets, and editor smarts for
[Achuk](https://achuk.dev) — the AI-first programming language.

Works in **VS Code, Cursor, Windsurf, and VSCodium** (anything that reads
VS Code extensions).

## Features

- Full TextMate grammar for `.achuk`: signatures, effectful `!` calls,
  `Module.function` references, tags, string interpolation `${…}`,
  lambdas, match/if, comments
- Snippets: `fn`, `main`, `match`, `if`, `lam`, `module`, `fold`
- Brackets, auto-closing, indentation rules

## The rest of the toolchain

- **Completions & hover** come from `achuk-lsp` (ships with the language):
  point your LSP client at `achuk-lsp --db path/to/achuk.cdb`.
- **AI integration** comes from `achuk-mcp` — the MCP server that lets
  Claude/Cursor/Windsurf agents query what actually exists in your
  project and typecheck generated code. See `docs/mcp.md` in the repo.

## Install (until it's on the marketplace)

```sh
cd editors/vscode
npx @vscode/vsce package        # produces achuk-lang-0.1.0.vsix
code --install-extension achuk-lang-0.1.0.vsix
```

Cursor/Windsurf: same `.vsix`, installed from their extension panes.
