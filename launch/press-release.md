# Press release

FOR IMMEDIATE RELEASE

## LambdaQ Labs releases Claw, an open-source programming language that ships with its own AI model — and makes hallucinated code structurally impossible

**The one-command install includes a fine-tuned local model, and every line
it generates is verified by the real compiler before the developer sees it.
Benchmark harness and results are public and reproducible.**

Claw, released today by LambdaQ Labs, takes a different position on AI
coding errors: instead of better prompts, a language designed so the
failure can't be expressed.

Three mechanisms work together. Claw code lives in a queryable,
content-addressed database, so an AI assistant asks "what exists with this
type?" rather than guessing an API. Generation is constrained by a grammar
projected from the project's actual scope — out-of-scope names are
unrepresentable at the token level, not merely discouraged. And everything
a model produces passes through the real compiler and executable contracts
before it is shown: Claw's `claw ai gen` command prints either "verified —
real compiler: OK" or a rejection with the compiler's error.

The measured effect, from the project's execution-graded public benchmark:
hallucinated API calls per run fell from 38 to 0, and a 0.5-billion-
parameter model fine-tuned for about three cents of GPU time writes 94%
functionally-correct Claw — where the same model writes 56% correct Python
and 7% correct Go. The result held when the experiment was repeated at 7B
scale against far stronger baselines (Rust 87%, Go 85%). The full harness
ships in the repository so any lab can run its own models against it.

Claw installs with one command and no dependencies. The bundle includes the
compiler, editor tooling, an MCP server that connects Claude, Cursor,
Windsurf, Zed, Gemini CLI and other AI assistants to the code database, and
the fine-tuned model with its inference engine — running locally, CPU-only,
with no API key. The package registry at registry.clawlang.dev enforces
that every published package carries machine-readable definitions, so
installing a dependency makes it immediately visible to any connected AI.

The language is a fork of the Roc compiler, released under the UPL-1.0
open-source license. Anonymous usage metrics are collected by default and
disabled with one command; source code is never collected without explicit
opt-in.

**Try it:** `curl -fsSL https://clawlang.dev/install.sh | sh` — or in the
browser at clawlang.dev/playground.html (the actual engine, compiled to
WebAssembly).

**Benchmark method and caveats:** github.com/LambdaQ-Labs/claw
(docs/parity-2026-07-05.md)

**Contact:** LambdaQ Labs · dev@clawlang.dev · clawlang.dev

---

### Email pitch template (per-outlet, keep under 150 words)

Subject: A language that ships its own LLM — hallucinated code made unrepresentable (benchmarks inside)

Hi {name},

One-line pitch: we released an open-source programming language whose
installer includes a fine-tuned local model, and whose toolchain makes
AI-hallucinated code structurally impossible — grammar-constrained
generation, compiler verification of every output, receipts included.

The number that matters: the same 0.5B model writes 94% functionally-
correct Claw vs 56% Python and 7% Go (execution-graded, harness public,
result reproduced at 7B).

Try in 60 seconds: curl -fsSL https://clawlang.dev/install.sh | sh
Method + caveats: https://github.com/LambdaQ-Labs/claw

Happy to walk you through the constrained-decoding design or run the
benchmark live. {outlet-specific line — see media-list.md}.

— Ninad, LambdaQ Labs
