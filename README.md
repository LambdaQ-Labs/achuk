<div align="center">

# 🐾 Claw

### The programming language where **AI can't hallucinate APIs.**

*Not "hallucinates less." Can't. It's ungeneratable.*

[![status](https://img.shields.io/badge/status-experimental-orange)](#status-honest)
[![built on](https://img.shields.io/badge/forked%20from-Roc-a020f0)](https://www.roc-lang.org)
[![license](https://img.shields.io/badge/license-UPL--1.0-blue)](#license)
[![PRs welcome](https://img.shields.io/badge/PRs-welcome-brightgreen)](#contributing)

**[Why](#the-idea) · [The Data](#the-data-real-not-vibes) · [Quickstart](#quickstart) · [How](#how-it-works) · [Status](#status-honest)**

</div>

---

Every LLM code assistant shares one dominant failure: it calls functions that **don't exist**. `generate_nonce()`, `list.sortBy()`, that method you *swear* the library has. In one study, **hallucinated APIs caused 41% of all compilation failures** in LLM-generated code.

Everyone else is fixing this with *bigger models* and *more retries*.

**Claw fixes it at the language level.** The compiler exposes a live database of every real, in-scope symbol, and the model is **constrained at decode time** to only emit those. A function that doesn't exist isn't "discouraged" — it is literally not in the grammar. The model *cannot type it.*

## The data (real, not vibes)

Same 15 tasks. Same models. The only change: give the model Claw's code-as-database symbol table.

| Model | Compiled ✗→✓ | Hallucinated symbols ✗→✓ |
|---|---|---|
| **DeepSeek-chat** | 0/15 → **13/15** | 38 → **0** |
| **Codestral** | 0/15 → **10/15** | 28 → **1** |

> API hallucination: **−96% to −100%**, from the language alone. No fine-tuning. No bigger model. [Full methodology →](docs/baseline-2026-07-03.md)

And with decode-time grammar constraints (arm A2), out-of-scope symbols hit a hard **structural zero** — proven by construction, not measured by luck.

## The idea

> A programming language designed to be **written by machines and verified by machines** — not typed by humans.

The research is blunt: [every prior "AI-first" language died](docs/master-plan.md) on training-data cold-start and ecosystem, not on ideas. So Claw is engineered around the failure modes LLMs *actually* have, measured on real benchmarks:

- **🚫 No hallucinated APIs** — code-as-database + decode-time grammar constraints
- **🧬 Code is a database, not text files** — content-addressed definitions; rename is O(1) and never breaks a caller
- **🔁 Structured errors, not prose** — every diagnostic is JSON with ranked patches, built for an agent's retry loop
- **🛡️ Memory-safe with no borrow-checker tax** — forked from [Roc](https://www.roc-lang.org): the strictness that helps LLMs, without the 92% compile-fail wall that Rust hits
- **📜 Contracts & effects** *(in progress)* — catch "compiles but does the wrong thing"

## Quickstart

```bash
git clone https://github.com/lambdaq-labs/claw && cd claw
cargo test --workspace            # the toolchain — ~60 tests, all green
cd compiler && zig build roc      # the compiler → clawc

# type-check Claw code
cargo run -p claw-cli -- check examples/hello.claw

# the magic: ask the code-as-database what really exists
cargo run -p claw-cli -- db candidates "Nat, Nat -> a"
cargo run -p claw-cli -- db mask "Nat, Nat -> a"   # → the grammar that makes hallucination impossible
```

Point any model at the benchmark and watch the hallucinations vanish:

```bash
export CLAW_MODEL_URL=… CLAW_MODEL_NAME=… CLAW_MODEL_KEY=…
cargo run -p claw-bench-runner -- run --arm A0 --tasks bench/tasks  # blind
cargo run -p claw-bench-runner -- run --arm A1 --tasks bench/tasks  # + Claw's symbol table
```

## How it works

```
 .claw source ─► clawc (typecheck) ─► code-as-database ─► candidates(type) ─► grammar mask ─► model
                                          │                                         │
                                    real symbols only              out-of-scope calls ungeneratable
```

The load-bearing trick: the model never references a symbol by guessing its name. It picks from a **typed menu of things that provably exist** — and the decoder's grammar won't let it write anything else.

## Status (honest)

**Experimental. Pre-alpha. Built in the open.** What works today: the compiler type-checks `.claw`, the code-as-database + constraint server run, the benchmark harness produces the numbers above. What's next: contracts, an effect system, a bundled model to beat the cold-start problem, and `--emit=rust` for ecosystem interop. See the [master plan](docs/master-plan.md).

This is a research bet with real early evidence, not a finished product. If that's your kind of thing — **★ star it and watch where it goes.**

## Contributing

Issues, ideas, and PRs welcome — especially benchmark tasks, grammar edge cases, and compiler work. Good first issues are tagged. Come argue with us about whether languages should be designed for humans or machines.

## License

UPL-1.0 (matching upstream Roc). Built by [LambdaQ Labs](https://clawlang.dev).

<div align="center">

*If a language where the AI **cannot** invent a fake API sounds interesting — the ★ button is right up there.*

</div>
