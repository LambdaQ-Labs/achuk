# Show HN post

**Title** (no adjectives, no exclamation, per HN rules):

> Show HN: Claw – a programming language that ships its own LLM, verified by the compiler

**URL:** https://clawlang.dev

**First comment (post immediately after submitting):**

Hi HN — I built Claw because AI coding assistants keep inventing APIs, and
no amount of prompting fixes that reliably. Claw's answer is structural:

- Code lives in a queryable database. An agent asks "what exists with this
  type?" (`claw db candidates "Str -> Str"`) instead of guessing.
- Generation is constrained by a grammar projected from your project's
  actual scope — out-of-scope names are unrepresentable at the token level.
- Everything the model produces goes through the real compiler and
  executable contracts before you ever see it. `claw ai gen` prints either
  "verified — real compiler: OK" or "REJECTED" with the compiler error.

The install is one command and includes a fine-tuned 0.5B model plus its
inference engine (llama.cpp) — no API key, runs CPU-only, offline:

    curl -fsSL https://clawlang.dev/install.sh | sh
    claw ai gen "define atleast10 : Nat -> Nat that computes Nat.max of x and 10"

Receipts (execution-graded, harness in the repo): the bundled fine-tune
writes 94% functionally-correct Claw where the same model writes 56%
Python and 7% Go. Hallucinated calls per benchmark run went 38 → 0. The
result held when we reran everything at 7B.

Honest caveats: the bundled model is small and fails on unusual
compositions (the guardrails reject those instead of shipping them); the
benchmark tasks are micro-functions and share generator DNA with the
training corpus (a held-out human-written set is included; third-party
tasks welcome — it's the repo's good-first-issue); Windows binaries are
cross-compiled and untested on real Windows.

The compiler is a fork of Roc (credit where due — its effect-platform
design is what makes the effect checking possible). Everything is UPL-1.0:
https://github.com/LambdaQ-Labs/claw

Happy to answer anything about the constrained decoding, the benchmark
method, or why a language would ship its own model.

**Rules reminders:** don't ask anyone to upvote (rings get detected);
stay in the thread for the first 3-4 hours answering everything;
tryable-without-signup requirement is satisfied by the playground.
