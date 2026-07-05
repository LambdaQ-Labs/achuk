# Getting started with Achuk

Achuk is an AI-agent-first programming language. This page gets you from zero
to a running program in about a minute.

## Install

```sh
curl -fsSL https://achuk.dev/install.sh | sh
```

This downloads a self-contained toolchain into `~/.achuk` and puts `achuk` on
your PATH. No system compiler, linker, or network platform is required — the
bundle ships everything: the compiler (with its own linker), the tooling,
and the fine-tuned Achuk model with its inference server (`achuk ai`).

Check it:

```sh
achuk --version
```

## Your first program

```sh
achuk new hello
cd hello
achuk run
```

`achuk new` scaffolds a project:

```
hello/
  main.achuk     # your program
  achuk.toml     # name, version, entry point
  achuk.cdb      # the code-as-database (indexed automatically)
  README.md
```

`achuk run` compiles and runs `main.achuk`. The starter prints `Hello, world!`.

## The program

```achuk
greet = |who| "Hello, ${who}!"

main! = |_args| {
    echo!(greet("world"))
    Ok({})
}
```

- `greet` is a function. `|who| ...` is a lambda; the body is an expression.
- `"${who}"` is string interpolation.
- `main!` is the entry point. The `!` marks it effectful (it can print).
  It receives the command-line arguments as a `List Str` and returns
  `Ok({})` on success.
- `echo!` prints a line.

## Letting the bundled model write Achuk for you

Every install includes a fine-tuned model that already speaks Achuk. One
command generates a definition — prompted with your project's *real*
symbols, grammar-constrained at decode time, and typechecked by the real
compiler before it's shown:

```sh
achuk ai gen "define double : Nat -> Nat"
```

The output prints as `.achuk` source followed by a `verified` (real compiler:
OK) or `REJECTED` verdict. Related commands:

```sh
achuk ai status   # where the model and server are, and whether it's running
achuk ai serve    # start the model server (gen does this automatically)
achuk ai stop     # stop it
```

The model (`model/achuk-0.5b-q8.gguf`) and inference server (`bin/achuk-infer`)
are found automatically inside the install; in a dev checkout, point
`ACHUK_MODEL_PATH` and `ACHUK_INFER_PATH` at them.

## Letting an AI agent write Achuk for you

Achuk's headline feature: an agent can't invent APIs that don't exist. Wire
it into Claude Code with one command:

```sh
achuk mcp install
```

This registers a local MCP server (`.mcp.json`) that exposes five tools
over *your real code*:

- `achuk_symbols` — every function that actually exists, with its type.
- `achuk_candidates` — given a target type, which real functions fit.
- `achuk_mask` — a decode grammar so out-of-scope calls are ungeneratable.
- `achuk_render` — render a Def-JSON definition to `.achuk` source.
- `achuk_check` — real-compile a definition and get structured errors back.

Re-index after adding files:

```sh
achuk index
```

## Packages

The package registry is live at
[registry.achuk.dev](https://registry.achuk.dev) (override with
`ACHUK_REGISTRY`):

```sh
achuk add mylib          # fetch a package, record it in achuk.toml
achuk publish            # bundle this package and upload it
```

Every published package carries its definitions (names, types, effects,
docs) — the registry rejects a publish without them — and `achuk add`
ingests them into your project's `achuk.cdb`, so the MCP tools and
`achuk ai` know an installed package's API immediately.

## Next

- [The Achuk language in 10 minutes](tour.md)
- Runnable examples: [`examples/`](../examples)

## What works today (v0.1)

- **Compile & run** real programs, self-contained.
- **Print + compute + args** with `achuk run`.
- **Networking** (a real HTTP server) — `achuk new myapi --platform http`
  scaffolds one; see [networking.md](networking.md).
- **AI guardrail** over your real symbol table (via `achuk index` + MCP).
- **Bundled model** — `achuk ai gen`, grammar-constrained + compiler-verified.
- **Packages** — `achuk publish` / `achuk add` against the live registry.

See the README's feature matrix for what's experimental vs planned.
