//! Server-rendered web UI: index + search + package pages.
//!
//! Same visual language as achuk.dev. Every package page renders the
//! definitions the MCP-compat gate required at publish — so the page IS
//! the documentation, generated from the same payload the AI consumes.

use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::response::Html;
use serde::Deserialize;
use sqlx::Row;

use crate::AppState;

/// Escape untrusted text for HTML contexts. Package names, docs, and type
/// strings are publisher-controlled — never interpolate them raw.
pub fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Page shell with no auth state (errors, static pages).
pub fn shell_page(title: &str, body: &str) -> Html<String> {
    shell(title, body, None)
}

/// Page shell. `user` = the logged-in username, if any (drives the header).
pub fn shell(title: &str, body: &str, user: Option<&str>) -> Html<String> {
    let account = match user {
        Some(u) => format!(
            r#"<a href="/u/{u}">{u}</a><form action="/logout" method="post" style="display:inline"><button class="linkbtn" type="submit">log out</button></form>"#,
            u = esc(u)
        ),
        None => r#"<a href="/login">Log in</a><a href="/signup" class="cta">Sign up</a>"#.into(),
    };
    Html(format!(
        r##"<!doctype html><html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<link rel="preconnect" href="https://fonts.googleapis.com"><link rel="preconnect" href="https://fonts.gstatic.com" crossorigin><link href="https://fonts.googleapis.com/css2?family=Fraunces:opsz,wght@9..144,400;9..144,500;9..144,600&family=Inter:wght@400;500;600;700&display=swap" rel="stylesheet">
<title>{title}</title>
<link rel="icon" href="data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 32 32'%3E%3Ccircle cx='16' cy='16' r='11' fill='none' stroke='%23c75b39' stroke-width='3' stroke-dasharray='52 17' stroke-linecap='round' transform='rotate(-42 16 16)'/%3E%3Ccircle cx='16' cy='16' r='3.6' fill='%23c75b39'/%3E%3C/svg%3E">
<style>
:root{{--bg:#faf6ec;--panel:#fffdf7;--panel2:#f0e9d8;--line:#e4d8c4;--ink:#1a1814;--dim:#6b6358;--acc:#c75b39;--copper:#a84a2c;--good:#3f7d55;
--mono:ui-monospace,'SF Mono',Menlo,Consolas,monospace;--sans:'Inter',system-ui,-apple-system,sans-serif;--serif:'Fraunces',Georgia,serif}}
@media(prefers-color-scheme:dark){{:root{{--bg:#1a1814;--panel:#242019;--panel2:#1f1b15;--line:#3a3428;--ink:#e4d8c4;--dim:#a89e8c;--acc:#e07a52;--copper:#e89b78;--good:#6fae82}}}}
*{{margin:0;padding:0;box-sizing:border-box}}
body{{background:var(--bg);color:var(--ink);font-family:var(--sans);font-size:16px;line-height:1.65}}
a{{color:var(--acc);text-decoration:none}}a:hover{{text-decoration:underline}}
.wrap{{max-width:860px;margin:0 auto;padding:0 22px}}
header{{padding:14px 0;border-bottom:1px solid var(--line)}}
header .wrap{{display:flex;align-items:center;gap:12px}}
.logo{{display:flex;align-items:center;gap:8px;font-weight:700;font-size:18px;color:var(--ink);font-family:var(--serif)}}
header nav{{margin-left:auto;display:flex;gap:16px;font-size:14px;align-items:center}}header nav a{{color:var(--dim)}}
.cta{{background:var(--acc);color:#fff!important;padding:6px 14px;border-radius:7px;font-weight:600}}
.cta:hover{{text-decoration:none;filter:brightness(1.05)}}
.linkbtn{{background:none;border:none;color:var(--dim);font:inherit;font-size:14px;cursor:pointer;padding:0}}
.linkbtn:hover{{color:var(--acc)}}
h1{{font-family:var(--serif);font-size:30px;letter-spacing:-.01em;margin:30px 0 6px;font-weight:600}}
.sub{{color:var(--dim);margin-bottom:22px}}
input[type=search],input[type=text],input[type=email],input[type=password]{{width:100%;font:15px var(--sans);padding:12px 16px;border:1.5px solid var(--line);border-radius:10px;background:var(--panel);color:var(--ink);margin:6px 0}}
input:focus{{outline:2px solid var(--acc);border-color:var(--acc)}}
.card{{background:var(--panel);border:1px solid var(--line);border-radius:10px;padding:16px 20px;margin:12px 0}}
.card h3{{font-size:17px}}.card h3 a{{color:var(--ink)}}
.meta{{font-size:13px;color:var(--dim);font-family:var(--mono)}}
.badge{{display:inline-block;font:600 11.5px var(--sans);color:var(--good);border:1px solid var(--good);border-radius:20px;padding:1px 10px;vertical-align:2px;margin-left:8px}}
.btn{{background:var(--acc);color:#fff;border:none;border-radius:8px;padding:11px 20px;font:600 15px var(--sans);cursor:pointer;margin-top:8px}}
.btn:hover{{filter:brightness(1.05)}}
.form{{max-width:380px}}
pre{{background:var(--panel);border:1px solid var(--line);border-radius:10px;padding:14px 16px;font:13.5px/1.6 var(--mono);overflow-x:auto;margin:12px 0}}
table{{width:100%;border-collapse:collapse;font-size:14px;margin:12px 0}}
th,td{{text-align:left;padding:8px 10px;border-bottom:1px solid var(--line);vertical-align:top}}
th{{color:var(--dim);font-size:11.5px;text-transform:uppercase;letter-spacing:.06em}}
td code{{font-family:var(--mono);font-size:13px}}
.eff{{color:var(--copper);font-family:var(--mono);font-size:12.5px}}
footer{{padding:34px 0;color:var(--dim);font-size:13.5px}}
</style></head><body>
<header><div class="wrap">
<a class="logo" href="/"><svg width="22" height="22" viewBox="0 0 32 32"><circle cx="16" cy="16" r="11" fill="none" stroke="#c75b39" stroke-width="3" stroke-dasharray="52 17" stroke-linecap="round" transform="rotate(-42 16 16)"/><circle cx="16" cy="16" r="3.6" fill="#c75b39"/></svg>Achuk <span style="color:var(--dim);font-weight:400">Registry</span></a>
<nav><a href="/browse">Browse</a><a href="https://achuk.dev/docs.html">Docs</a>{account}</nav>
</div></header>
<div class="wrap">{body}</div>
<footer><div class="wrap">Every package here is AI-legible by rule: it publishes its definitions
(names, types, effects, docs), your tools ingest them on <code style="font-family:var(--mono)">achuk add</code>.
· <a href="https://achuk.dev/docs.html#packages">How packages work</a></div></footer>
</body></html>"##
    ))
}

/// `GET /signup` — the form.
pub async fn signup_page() -> Html<String> {
    shell(
        "Sign up — Achuk Registry",
        r#"<h1>Create an account</h1>
<p class="sub">Publish packages the whole ecosystem — and every AI — can read.</p>
<form class="form" action="/signup" method="post">
<input type="text" name="username" placeholder="username" autocomplete="username" required>
<input type="email" name="email" placeholder="email" autocomplete="email" required>
<input type="password" name="password" placeholder="password (8+ chars)" autocomplete="new-password" required>
<button class="btn" type="submit">Sign up</button>
</form>
<p class="sub" style="margin-top:16px">Already have an account? <a href="/login">Log in</a>.</p>"#,
        None,
    )
}

/// `GET /login` — the form.
pub async fn login_page() -> Html<String> {
    shell(
        "Log in — Achuk Registry",
        r#"<h1>Log in</h1>
<form class="form" action="/login" method="post">
<input type="text" name="username" placeholder="username" autocomplete="username" required>
<input type="password" name="password" placeholder="password" autocomplete="current-password" required>
<button class="btn" type="submit">Log in</button>
</form>
<p class="sub" style="margin-top:16px">New here? <a href="/signup">Create an account</a>.</p>
<p class="sub">From the CLI: <code>achuk login</code>.</p>"#,
        None,
    )
}

/// `GET /u/:username` — a publisher's profile and packages.
pub async fn profile_page(
    State(st): State<AppState>,
    headers: HeaderMap,
    Path(username): Path<String>,
) -> Html<String> {
    let me = crate::auth::current_user(&st, &headers).await;
    let exists = sqlx::query("SELECT 1 FROM users WHERE username = $1")
        .bind(&username)
        .fetch_optional(&st.pool)
        .await
        .ok()
        .flatten()
        .is_some();
    if !exists {
        return shell(
            "not found — Achuk Registry",
            &format!("<h1>{}</h1><p class=\"sub\">No such user.</p>", esc(&username)),
            me.as_deref(),
        );
    }
    let rows = sqlx::query(
        "SELECT DISTINCT ON (name) name, version, published_at::text AS pub
         FROM packages WHERE owner = $1 ORDER BY name, published_at DESC",
    )
    .bind(&username)
    .fetch_all(&st.pool)
    .await
    .unwrap_or_default();
    let cards: String = rows
        .iter()
        .map(|r| {
            let name: String = r.get("name");
            let version: String = r.get("version");
            let published: String = r.get::<String, _>("pub").chars().take(10).collect();
            let n = defs_count(&st, &name, &version);
            pkg_card(&name, &version, &published, n)
        })
        .collect();
    // Show the CLI token only to the owner viewing their own page.
    let token_box = if me.as_deref() == Some(username.as_str()) {
        let tok = sqlx::query("SELECT token FROM users WHERE username = $1")
            .bind(&username)
            .fetch_optional(&st.pool)
            .await
            .ok()
            .flatten()
            .map(|r| r.get::<String, _>("token"))
            .unwrap_or_default();
        format!(
            r#"<div class="card"><h3>Your CLI token</h3>
<p class="sub" style="margin:4px 0 8px">Log in from the terminal to publish:</p>
<pre>achuk login {}</pre>
<p class="meta">Keep this secret. It grants publish access to your packages.</p></div>"#,
            esc(&tok)
        )
    } else {
        String::new()
    };
    let heading = format!("<h1>{}</h1>", esc(&username));
    let body = if rows.is_empty() {
        format!(
            "{heading}{token_box}<p class=\"sub\">No packages published yet.</p>"
        )
    } else {
        format!(
            "{heading}{token_box}<p class=\"sub\">{} package(s)</p>{cards}",
            rows.len()
        )
    };
    shell(&format!("{} — Achuk Registry", esc(&username)), &body, me.as_deref())
}

fn pkg_card(name: &str, version: &str, published: &str, n_defs: Option<i64>) -> String {
    let defs_note = n_defs
        .map(|n| format!("{n} definitions"))
        .unwrap_or_else(|| "definitions published".into());
    format!(
        r#"<div class="card"><h3><a href="/p/{n}">{n}</a><span class="badge">✓ AI-legible</span></h3>
<div class="meta">{v} · {defs_note} · published {p}</div></div>"#,
        n = esc(name),
        v = esc(version),
        p = esc(published),
    )
}

#[derive(Deserialize)]
pub struct SearchQ {
    pub q: Option<String>,
}

/// `GET /` — landing: search + recent packages.
pub async fn index_html(
    State(st): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<SearchQ>,
) -> Html<String> {
    let me = crate::auth::current_user(&st, &headers).await;
    let needle = q.q.unwrap_or_default();
    let rows = if needle.is_empty() {
        sqlx::query(
            "SELECT DISTINCT ON (name) name, version, published_at::text AS pub
             FROM packages ORDER BY name, published_at DESC LIMIT 50",
        )
        .fetch_all(&st.pool)
        .await
    } else {
        sqlx::query(
            "SELECT DISTINCT ON (name) name, version, published_at::text AS pub
             FROM packages WHERE name ILIKE '%' || $1 || '%'
             ORDER BY name, published_at DESC LIMIT 50",
        )
        .bind(&needle)
        .fetch_all(&st.pool)
        .await
    }
    .unwrap_or_default();

    let cards: String = rows
        .iter()
        .map(|r| {
            let name: String = r.get("name");
            let version: String = r.get("version");
            let published: String = r.get::<String, _>("pub").chars().take(10).collect();
            let n_defs = defs_count(&st, &name, &version);
            pkg_card(&name, &version, &published, n_defs)
        })
        .collect();

    let results = if rows.is_empty() && !needle.is_empty() {
        format!("<p class=\"sub\">Nothing matches <b>{}</b> yet.</p>", esc(&needle))
    } else {
        cards
    };

    shell(
        "Achuk Registry — packages your AI understands",
        &format!(
            r#"<h1>Packages your AI understands</h1>
<p class="sub">Every package publishes its definitions — install one and your editor,
your assistant, and <code style="font-family:var(--mono)">achuk ai</code> know it instantly.</p>
<form action="/" method="get"><input type="search" name="q" value="{}" placeholder="search packages…" aria-label="search packages"></form>
{results}
<pre># publish yours
achuk login
achuk publish        # exports your definitions automatically — that's the whole requirement</pre>"#,
            esc(&needle)
        ),
        me.as_deref(),
    )
}

fn defs_count(st: &AppState, name: &str, version: &str) -> Option<i64> {
    let p = st.blobs.join(format!("{name}-{version}.defs.json"));
    let b = std::fs::read(p).ok()?;
    serde_json::from_slice::<Vec<serde_json::Value>>(&b)
        .ok()
        .map(|v| v.len() as i64)
}

/// `GET /p/:name` — the package page: install, versions, definitions.
pub async fn package_html(
    State(st): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Html<String> {
    let me = crate::auth::current_user(&st, &headers).await;
    let rows = sqlx::query(
        "SELECT version, hash, size, owner, published_at::text AS pub
         FROM packages WHERE name = $1 ORDER BY published_at DESC",
    )
    .bind(&name)
    .fetch_all(&st.pool)
    .await
    .unwrap_or_default();

    if rows.is_empty() {
        return shell(
            "not found — Achuk Registry",
            &format!("<h1>{}</h1><p class=\"sub\">No such package.</p>", esc(&name)),
            me.as_deref(),
        );
    }
    let owner: Option<String> = rows[0].try_get("owner").ok();
    let by = owner
        .filter(|o| !o.is_empty())
        .map(|o| format!(" · by <a href=\"/u/{o}\">{o}</a>", o = esc(&o)))
        .unwrap_or_default();

    let latest: String = rows[0].get("version");
    let hash: String = rows[0].get("hash");

    let defs_html = match std::fs::read(st.blobs.join(format!("{name}-{latest}.defs.json")))
        .ok()
        .and_then(|b| serde_json::from_slice::<Vec<serde_json::Value>>(&b).ok())
    {
        Some(defs) => {
            let rows: String = defs
                .iter()
                .map(|d| {
                    let n = d["name"].as_str().unwrap_or("?");
                    let t = d["ty"].as_str().unwrap_or("?");
                    let doc = d["doc"].as_str().unwrap_or("");
                    let effs = d["effects"]
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(|e| e.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_default();
                    let eff_html = if effs.is_empty() {
                        String::new()
                    } else {
                        format!("<div class=\"eff\">effects: {}</div>", esc(&effs))
                    };
                    format!(
                        "<tr><td><code>{}</code></td><td><code>{}</code>{}</td><td>{}</td></tr>",
                        esc(n),
                        esc(t),
                        eff_html,
                        esc(doc)
                    )
                })
                .collect();
            format!(
                r#"<h2 style="font-size:20px;margin-top:30px">Definitions <span class="badge">✓ AI-legible</span></h2>
<p class="sub" style="margin-bottom:4px">What your code database learns on install — names, types, effect rows.</p>
<table><tr><th>definition</th><th>type</th><th>doc</th></tr>{rows}</table>"#
            )
        }
        None => "<p class=\"sub\">Definitions unavailable for the latest version.</p>".into(),
    };

    let versions: String = rows
        .iter()
        .map(|r| {
            let v: String = r.get("version");
            let s: i64 = r.get("size");
            let p: String = r.get::<String, _>("pub").chars().take(10).collect();
            format!(
                "<tr><td><code>{}</code></td><td>{} KB</td><td>{}</td></tr>",
                esc(&v),
                s / 1024,
                esc(&p)
            )
        })
        .collect();

    shell(
        &format!("{} — Achuk Registry", esc(&name)),
        &format!(
            r#"<h1>{n}<span class="badge">✓ AI-legible</span></h1>
<div class="meta">latest {v} · content hash <code>{h}</code>{by}</div>
<pre>achuk add {n}</pre>
{defs_html}
<h2 style="font-size:20px;margin-top:30px">Versions</h2>
<table><tr><th>version</th><th>size</th><th>published</th></tr>{versions}</table>"#,
            n = esc(&name),
            v = esc(&latest),
            h = esc(&hash.chars().take(16).collect::<String>()),
        ),
        me.as_deref(),
    )
}
