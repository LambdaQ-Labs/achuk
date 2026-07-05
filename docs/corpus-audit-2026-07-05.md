# Corpus sufficiency audit — 2026-07-05

**Question:** are 1,446 examples enough for "any kind of development
activity"? **Answer: no — and now measured.** The v3 corpus taught only 4
of the language's 11 expression forms:

| expression form | v3 (1446) | v4 (1661) | note |
|---|---|---|---|
| Lam / App / Var | 100% | 100% | call-a-function-with-args |
| Lit | 640 | 640+ | int/str constants |
| If (native, lazy) | **0** | 50 | was only the `Bool.if` CALL shape |
| Let | **0** | 125 | intermediate bindings (binder from the p-pool: p8) |
| Match | **0** | 30 | Maybe/Result unwrapping, binder p1 |
| Tag | **0** | 40 | Some/Ok construction |
| recursion (self-call) | **0** | 10 | guarded by a base case |
| Record / Field | **0** | **0** | blocked: core `Type` has no record type yet |
| Ref (by hash) | 0 | 0 | intentional — models reference by NAME |

v3 could teach a model to *wire existing functions together*. It could not
teach branching syntax, naming an intermediate value, unwrapping a
Maybe/Result, constructing a tagged value, or recursion — i.e. most of what
a real function body does. v4 adds all of those as generator classes
(deterministic, hallucination-free by construction, validated: 0 bad
references across 1,661 examples).

## Supporting fixes in the same pass

- **Eval scoring understands binders now.** The scripts' free-variable walk
  treated `let p8 = ...` and match binders as hallucinated names. Replaced
  with a proper scoped walk (Lam params, Let names, Match pattern vars,
  sibling/self def names — recursion is legal), mirroring the Rust grader.
- **A2 grammar covers the new shapes.** GBNF gains If/Let/Match/Tag rules
  (depth-bounded like App/Lam), a pattern grammar (Wild / p-pool binder /
  literal / one-level tag), a tag pool (Some/None/Ok/Err), and — from the
  earlier fix — a sibling-name pool (step/helper/aux/go/part) so multi-def
  outputs are structurally expressible under constrained decoding.

## What "enough" looks like from here

v4 is enough to *train the discipline* on every shape the Def-JSON protocol
can express today. It is NOT yet enough for open-ended development:

1. **Records** need `Type::Record` in claw-core first (surface + compiler
   already have them; the protocol doesn't).
2. **Scale**: shape coverage ≠ volume. For a bigger base model, multiply
   each class by more type combinations (Str/Bool variants of If/Let/Match)
   and richer stdlib — hundreds of symbols, not 34.
3. **Real usage data** beats synthetic pairs — which is what the telemetry
   channel (opt-in) exists to collect.

Retrain on v4 + re-gate happens with the parity-eval pod session.
