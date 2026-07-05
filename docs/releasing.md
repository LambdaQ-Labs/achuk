# Shipping Achuk: releases, updates, and the model channel

How a change in this repo reaches a user's machine — binaries, language,
and the bundled model.

## The pipeline

```
git tag vX.Y.Z && git push --tags
        │
        ├── Drone CI (.drone.yml, ci.hostingduty.com)
        │     linux-x64 + windows-x64: zig cross-compiles achukc,
        │     cargo builds achuk/achuk-mcp/achuk-lsp, tarball/zip +
        │     sha256 → uploaded to the GitHub Release
        │     (does not yet stage model/ + achuk-infer — the v0.1.0
        │      one-bundles were assembled from the mac cross-build)
        │
        └── macOS (manual until a mac runner exists):
              scripts/package.sh vX.Y.Z → dist/achuk-vX.Y.Z-macos-arm64.tar.gz
              gh release upload vX.Y.Z dist/*.tar.gz
```

One-time setup still pending (owner action): grant the Drone OAuth app
access to the LambdaQ-Labs org and add a `github_token` secret so the
release-upload step can publish.

## How users get it

- **First install:** `curl -fsSL https://achuk.dev/install.sh | sh` —
  resolves the latest tag via the GitHub API, downloads the platform
  tarball into `~/.achuk`, adds `achuk` to PATH.
- **Updates:** `achuk upgrade` — compares the running version against the
  latest release, downloads the tarball, verifies the `.sha256` sidecar
  when published, and swaps the binaries in place (`achuk upgrade --check`
  only reports). Dev checkouts are refused — use git + cargo there.

## Versioning

Workspace version (`Cargo.toml [workspace.package] version`) is the single
source; tag `vX.Y.Z` must match it. Compiler (`achukc`) ships inside the
same tarball, so language + tooling always move together — no version
skew between the CLI and the compiler a user has.

## The model channel

**The model ships IN the release bundle.** Every artifact packages
`model/achuk-0.5b-q8.gguf` (a ~506 MB q8_0 quantization of the
Qwen2.5-Coder-0.5B fine-tune) plus `bin/achuk-infer` (llama.cpp's
llama-server) next to the toolchain — `achuk ai` finds both by the install
layout, no separate download or configuration. `scripts/package.sh`
requires both files (override with `ACHUK_MODEL_FILE` / `ACHUK_INFER_BIN`);
attribution for llama.cpp (MIT) and Qwen (Apache-2.0) lives in `NOTICE`.
The model dominates artifact size: linux-x64 ~597 MB, windows-x64 ~578 MB.

A *separate* model channel remains future work — for shipping model
updates between toolchain releases:

- `achuk-model-<ver>.tar.gz` attached to a `model-<ver>` release (or
  hosted on R2 next to telemetry — no egress fees either way).
- Future `achuk model upgrade`: same flow as `achuk upgrade` — check,
  download, sha256, swap under `~/.achuk/model/`. The gate report
  (hallucination-free %, parity numbers) is published in the release
  notes so users see exactly what a model update buys.
- Cadence: retrain when telemetry + corpus growth move the gate, not on
  a clock. Every model release must re-pass the reference gate before
  tagging.

## Artifact test findings (v0.1.0 dry run, 2026-07-05)

All three artifacts built and tested: macOS-arm64 (full workflow, 8/8 —
check/run/fmt/db/defs-check/grammar/mcp/telemetry), linux-x64 (same suite
in docker; static musl binaries run on alpine AND debian), windows-x64
(valid PE32+ executables; needs a Windows box or wine for execution).

Known requirements / cleanups before the public tag:
- **Linux `achuk run` needs a system linker** (`gcc` or `binutils`) — the
  compiler's link step shells out. `achuk check` needs nothing. Document
  in install.sh output or vendor a linker later.
- ~~achukc ships as a debug build~~ **Done**: artifacts are ReleaseFast
  with git version stamps (`release-fast-<hash>`); linux tarball dropped
  323→85 MB, macOS 161→72 MB, windows 88→58 MB. (Toolchain-only sizes —
  measured before the model was bundled in; see the model channel above
  for shipped one-bundle sizes.)
- **zig 0.16.0's x86_64-linux toolchain SEGVs building achukc at ANY
  optimize level** (ReleaseFast/Safe/Small, -j1, 188 GB RAM box — upstream
  bug; debug builds fine; zig master rejects the build script for other
  reasons). Workaround shipped: cross-compile the linux and windows
  compilers FROM arm64 macOS (`zig build roc -Dtarget=x86_64-linux-musl
  -Doptimize=ReleaseFast` — build-time tools then run natively on arm64).
  The Drone linux pipeline is affected the same way — until a fixed zig
  lands, release achukc for linux/windows comes from a mac cross-build.
- Cross-building the compiler is impossible under qemu emulation (the
  build-time builtin_compiler miscomputes) — build on real hardware per
  target family, as the Drone runners do.
- Zig ≥0.14 tarballs are named `zig-<arch>-<os>` (already fixed in CI).
- Never run two zig builds concurrently in one checkout — the shared
  .zig-cache corrupts.

## Release checklist

1. `cargo test --workspace` green, clippy clean.
2. Bump the workspace version; update CHANGELOG.md.
3. Tag + push. Drone builds linux/windows; run `package.sh` on a Mac.
4. Verify `install.sh` + `achuk upgrade` against the new release from a
   clean machine.
5. If the model changed: attach the adapter asset + gate report.
