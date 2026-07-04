# Claw package registry

An npm/npmjs.com-style registry for Claw packages. Claw bundles are
content-addressed `.tar.zst` archives (`claw bundle` names them by their
BLAKE3 hash); this registry stores them by name+version and serves each
bundle at a stable URL the Claw compiler fetches and hash-verifies.

## Run

```sh
createdb claw_registry
cd service
DATABASE_URL="postgres://$USER@localhost:5432/claw_registry" cargo run
# → http://127.0.0.1:8888  (index page lists packages)
```

## Use it

```sh
# publish a library package (a dir with a `package [..] {}` main + modules)
cd mylib && claw publish

# add + use it in another project
claw add mylib
# then: import mylib.Module ...   (claw run fetches it)
```

The trust model is content-addressing: the URL's last segment is the
bundle's BLAKE3 hash, and the compiler recomputes it on download — no
signing or registry auth needed. Loopback HTTP is allowed; a public
registry needs HTTPS.
