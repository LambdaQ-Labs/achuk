# Changelog

## Unreleased (v0.1.1) — 2026-07-05

### Research
- **Reference gate at 100%:** the tuned model is **121/121 hallucination-free
  + effect-sound** on the 121-task gate, at both 0.5B and 7B. See
  `docs/p4-v3-gate-2026-07-05.md`.
- **P4 parity gate passed — at both scales** (functional Pass@1, 116 tasks,
  execution-graded, same model per row; `docs/parity-2026-07-05.md`):
  - 0.5B: Achuk-tuned **94%** vs JS 89%, Python 56%, Rust 35%, Go 7%.
  - 7B: Achuk-tuned **94%** (110/116) vs Rust 87%, Go 85%, Python 71%, JS 68%.
    Train loss 0.039, ~54 min, ~$0.25.
- **Corpus v4** (`train/corpus-v4.jsonl`, 1661 examples) covering all
  expression shapes.
- **A2 grammar upgrades:** sibling calls + `If`/`Let`/`Match`/`Tag`
  expression forms in the decode grammar.
- **Held-out task set** (`bench/tasks-holdout/`, 25 tasks) and
  **real-compile grading** (every graded solution goes through `achukc`).

### Added
- **`achuk ai` (gen/serve/status/stop):** the bundled model wired to the
  guardrails. `achuk ai gen "<task>"` prompts with the project CDB's real
  symbols, constrains decoding with the scope's GBNF grammar, and
  verifies the result with the real compiler (prints `verified` /
  `REJECTED`). The server (bundled llama.cpp, `bin/achuk-infer`)
  auto-starts on port 8873; `ACHUK_MODEL_PATH`/`ACHUK_INFER_PATH` override
  discovery in dev checkouts.
- **One-bundle artifacts:** release tarballs now ship the fine-tuned
  model (`model/achuk-0.5b-q8.gguf`, ~506 MB q8_0 of Qwen2.5-Coder-0.5B)
  and `bin/achuk-infer` alongside the toolchain — no separate model
  download. `NOTICE` credits llama.cpp (MIT) and Qwen (Apache-2.0).
- **Registry live** at https://registry.achuk.dev (the CLI default),
  with the MCP-compatibility gate: `achuk publish` exports every
  definition's name/type/effects/doc (`defs.json`) and the registry
  rejects packages without parseable defs; `achuk add` ingests a
  package's defs into the project `achuk.cdb`, so MCP,
  `achuk db candidates`, and `achuk ai` know installed packages.
- **Live domains:** https://achuk.dev (site, wasm playground,
  `install.sh` at the web root) and https://telemetry.achuk.dev
  (now the default ingest endpoint).
- CLI: `achuk defs-check`, `achuk defs-grade`, `achuk task-grammar`,
  `achuk telemetry (status|share|clear)`, `achuk upgrade`,
  `achuk publish` / `achuk add`.
- MCP: two new tools — `achuk_render` (Def-JSON → `.achuk` source) and
  `achuk_check` (real-compile with structured errors) — five total.
- **VS Code extension** (`editors/vscode`): tmLanguage grammar + snippets,
  packaged vsix.
- **Website** (`site/`, achuk.dev).
- **Telemetry**: anonymous metrics by default (never code; `achuk telemetry off` to disable, loud first-run notice); `full` code-sharing level stays opt-in. Ingest worker deployed (Cloudflare + R2).
- **Registry + playground plan**: `docs/registry-playground-plan.md`.

## v0.1.0 — first downloadable release (2026-07-04)

The release where Achuk becomes something you can **download and build with**,
not just a research toolchain.

### Added
- **Install in one line:** `curl -fsSL https://achuk.dev/install.sh | sh`
  installs a self-contained toolchain into `~/.achuk` (bundled compiler,
  platform, and linker — no system toolchain required).
- **Project model:** `achuk new <name>` scaffolds a runnable project;
  `achuk run [file]` compiles and runs it.
- **AI guardrail on your real code:** `achuk index` ingests a project's real
  functions + inferred types into the code-as-database; `achuk mcp install`
  registers an MCP server so Claude Code (and any MCP client) can call
  `achuk_symbols` / `achuk_candidates` / `achuk_mask` over *your* symbols and
  cannot reference APIs that don't exist.
- **Distribution:** `scripts/package.sh` builds per-platform tarballs; a
  GitHub Actions release workflow builds + smoke-tests + publishes for
  macOS (arm64) and Linux (x64).
- **Docs & examples:** getting-started, a 10-minute language tour, and
  runnable examples (hello, fizzbuzz, pattern matching, args).
- `achuk --version`.

### Fixed
- 20 findings from a multi-agent code review across the toolchain
  (interpreter stack-overflow guard, checked arithmetic, type-variable
  capture in `candidates()`, GBNF canonical integers, emitter keyword
  escaping, distillation gate, and more).

### Known limits (roadmap)
- I/O is print + compute + args only; file/stdin/network is v0.1.1.
- The AI guardrail is symbol-level; lowering real bodies + call-graph into
  the database (so the AI understands whole programs) is v0.2.
- Contracts / effects / `emit-rust` operate on the synthetic AST, not yet
  on real `.achuk` bodies.
- The bundled fine-tuned model ships as a separate research download.
- Windows is not yet a release target.

### Research
- First base-vs-tuned P4 gate: a fine-tuned 0.5B went **0 → 98%
  hallucination-free** on the target distribution for ~$0.30 of GPU, while
  its own base stays at 0%. See `docs/p4-gate-2026-07-04.md`.
