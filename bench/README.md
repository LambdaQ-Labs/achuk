# bench/ — WS-J (Benchmark Harness)

**Build this FIRST.** Both kill-gates (P2, P4) are defined against it. Full spec: [`../docs/benchmark-harness.md`](../docs/benchmark-harness.md).

- `tasks/` — 31 tasks (JSON schema + reference solutions + test oracles).
- `tasks-holdout/` — 25 held-out tasks (never in the corpus; anti-memorization).
- `tasks-large/` — 121 tasks, the reference-gate set.
- `grammars/` — 146 per-task decode grammars (GBNF).
- `grader/` — deterministic, model-free grader: compile ∧ tests ∧ contracts ∧ no-forbidden ∧ no-hallucinated-symbols.
- Runner arms: A0 baseline → A1 +context → A2 +mask (P2 gate) → A3 +bundled model / Ref-Python (P4 gate).
- Parity harness (5 languages: Achuk / JS / Python / Rust / Go, execution-graded): `../train/parity_gen.py` + `../train/parity_grade.py`.

Gates:
- **P2:** A2 compile-error rate >30% below A0; hallucinated-symbols → ~0.
- **P4:** A3 pass-rate on Achuk ≥ Ref pass-rate on Python (held-out split). The Matthew-Effect reversal. **Passed 2026-07-05** at 0.5B and 7B — see `../docs/p4-v3-gate-2026-07-05.md` and `../docs/parity-2026-07-05.md`.
