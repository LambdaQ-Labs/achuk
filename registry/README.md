# Achuk package registry

An npm/npmjs.com-style registry for Achuk packages. Achuk bundles are
content-addressed `.tar.zst` archives (`achuk bundle` names them by their
BLAKE3 hash); this registry stores them by name+version and serves each
bundle at a stable URL the Achuk compiler fetches and hash-verifies.

The hosted instance is **https://registry.achuk.dev** — the CLI's
default (`ACHUK_REGISTRY` overrides it, e.g. to point at a local one).

MCP-compatibility is enforced at the door: a publish must include the
package's definitions (name/type/effects/doc per def — `achuk publish`
generates them from the entry file) or it is rejected; they're served at
`GET /defs/:name/:version`, and `achuk add` ingests them into the
project's `achuk.cdb` so the AI layer knows the package on install.

## Run (local dev)

```sh
createdb achuk_registry
cd service
DATABASE_URL="postgres://$USER@localhost:5432/achuk_registry" cargo run
# → http://127.0.0.1:8888  (index page lists packages)
```

## Use it

```sh
# publish a library package (a dir with a `package [..] {}` main + modules)
cd mylib && achuk publish

# add + use it in another project
achuk add mylib
# then: import mylib.Module ...   (achuk run fetches it)
```

The trust model is content-addressing: the URL's last segment is the
bundle's BLAKE3 hash, and the compiler recomputes it on download — no
signing or registry auth needed. Loopback HTTP is allowed; a public
registry needs HTTPS.
