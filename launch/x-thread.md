# X / Twitter launch thread

**1/**
AI assistants invent APIs. Everyone's fix is better prompts.

Ours is a programming language where hallucinated code is unrepresentable.

Claw is out today — and it ships with its own LLM in the installer.
https://clawlang.dev

**2/**
The pipeline, not the model, is the product:

▸ code lives in a queryable database — the AI asks what exists
▸ generation is grammar-constrained to your project's real symbols
▸ the real compiler verifies every result before you see it

`claw ai gen` prints "verified: OK" or "REJECTED" — never silent garbage.

**3/**
One command. No API key. No cloud. The model runs on your laptop, offline:

curl -fsSL https://clawlang.dev/install.sh | sh

Inside: compiler, LSP, MCP server (Claude/Cursor/Zed/Gemini plug in
directly), a fine-tuned model, and its inference engine.

**4/**
The receipts — same model, five languages, every function executed and
checked:

Claw (tuned for $0.03): 94%
JavaScript: 89%
Python: 56%
Rust: 35%
Go: 7%

Reran at 7B: Claw still first at 94%. Harness is in the repo. Run it on
your own model.

**5/**
The part I'm proudest of: packages are AI-legible by law.

Publishing to registry.clawlang.dev requires your definitions — names,
types, effects. `claw add somelib` feeds them straight into your AI's
context. Install a package; your assistant already knows it.

**6/**
Honest limits: the bundled 0.5B fails on unusual compositions (guardrails
reject instead of shipping), benchmarks are micro-functions, Windows
builds are untested on real hardware.

It's day one. The thesis is measurable and the harness is public:
https://github.com/LambdaQ-Labs/claw

**7/**
Built on the shoulders of @rtfeldman's Roc compiler — the effect-platform
design is what makes checkable effects possible. Thank you.

Try it in the browser (the actual engine, compiled to wasm):
https://clawlang.dev/playground.html
