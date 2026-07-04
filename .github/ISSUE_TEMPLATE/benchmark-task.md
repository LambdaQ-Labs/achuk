---
name: "🟢 New benchmark task"
about: Add a coding task to the benchmark suite (great first contribution)
title: "bench: <short description>"
labels: ["good first issue", "benchmark"]
---

**What should the task ask the model to do?**
<!-- e.g. "Implement `dedupe : List a -> List a` preserving first occurrence." -->

**In-scope symbols** (the CDB context — names + type signatures)
<!--
  - List.contains : List a, a -> Bool
  - List.cons     : a, List a -> List a
  - List.empty    : List a
-->

**Category:** from-scratch | translate | repo-feature | contract | effect

**Grading:** what must hold? (compiles / tests / contracts / no deprecated symbol / no hallucination)

<!-- Drop the finished JSON in bench/tasks/ and run `cargo test -p claw-bench-runner` to validate. -->
