# The `sys` platform

A Claw platform providing real system I/O — proof that heavy lifting lives in
the platform host, not the language. The language has zero I/O; this host
(self-contained Zig, `std`/`builtin` only) implements the effects.

## Effects
- `File.read! : Str => Str` — read a file's contents
- `Env.get!  : Str => Str` — read an environment variable
- `Stdout.line! : Str => {}` — print a line

## How it works
`platform/main.roc` maps host C-symbols to typed Claw effects:
`hosted { "roc_file_read": File.read!, "roc_env_get": Env.get!, ... }`.
`platform/host.zig` implements `roc_file_read(path: ClawStr) ClawStr` etc.,
marshalling `ClawStr` (refcounted) across the C ABI. Build the host:

```sh
zig build-lib platform/host.zig -target aarch64-macos -O ReleaseSmall -lc \
  -femit-bin=platform/targets/arm64mac/libhost.a
```

Then a consumer app:
```claw
app [main!] { pf: platform "./platform/main.roc" }
import pf.File
import pf.Stdout
main! = |_args| {
    Stdout.line!("file says: ${File.read!("data.txt")}")
    Ok({})
}
```

Verified: reads a real file + `HOME` env var and prints them.

## Next
The same ABI works for a **Rust host** (link crates.io) — Postgres via sqlx,
HTTP via reqwest, etc. This Zig host proves the boundary; the Rust host is
the ecosystem multiplier.
