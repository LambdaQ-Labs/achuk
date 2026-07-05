//! Server-rendered web UI: index + search + package pages.
//!
//! Same visual language as clawlang.dev. Every package page renders the
//! definitions the MCP-compat gate required at publish — so the page IS
//! the documentation, generated from the same payload the AI consumes.

use axum::extract::{Path, Query, State};
use axum::response::Html;
use serde::Deserialize;
use sqlx::Row;

use crate::AppState;

/// Escape untrusted text for HTML contexts. Package names, docs, and type
/// strings are publisher-controlled — never interpolate them raw.
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn shell(title: &str, body: &str) -> Html<String> {
    Html(format!(
        r##"<!doctype html><html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<link rel="icon" href="data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 32 32'%3E%3Cpath d='M6 26 Q10 12 16 6 Q14 16 12 26 Z M13 26 Q17 13 23 7 Q20 17 18 26 Z M19 26 Q23 15 28 10 Q25 19 24 26 Z' fill='%23b45309'/%3E%3C/svg%3E">
<style>
:root{{--bg:#faf7f2;--panel:#fff;--panel2:#f1ece4;--line:#e5ddd0;--ink:#292420;--dim:#6b6259;--acc:#ea580c;--copper:#b45309;--good:#15803d;
--mono:ui-monospace,'SF Mono',Menlo,Consolas,monospace;--sans:'Avenir Next','Segoe UI',-apple-system,sans-serif}}
@media(prefers-color-scheme:dark){{:root{{--bg:#151210;--panel:#201b17;--panel2:#1a1613;--line:#332c25;--ink:#ede9e4;--dim:#a39a8e;--acc:#fb923c;--copper:#e08a2e;--good:#4ade80}}}}
*{{margin:0;padding:0;box-sizing:border-box}}
body{{background:var(--bg);color:var(--ink);font-family:var(--sans);font-size:16px;line-height:1.65}}
a{{color:var(--acc);text-decoration:none}}a:hover{{text-decoration:underline}}
.wrap{{max-width:860px;margin:0 auto;padding:0 22px}}
header{{padding:14px 0;border-bottom:1px solid var(--line)}}
header .wrap{{display:flex;align-items:center;gap:10px}}
.logo{{display:flex;align-items:center;gap:8px;font-weight:700;font-size:18px;color:var(--ink)}}
header nav{{margin-left:auto;display:flex;gap:16px;font-size:14px}}header nav a{{color:var(--dim)}}
h1{{font-size:26px;letter-spacing:-.02em;margin:30px 0 6px}}
.sub{{color:var(--dim);margin-bottom:22px}}
input[type=search]{{width:100%;font:15px var(--mono);padding:12px 16px;border:1.5px solid var(--line);border-radius:10px;background:var(--panel);color:var(--ink)}}
input:focus{{outline:2px solid var(--acc);border-color:var(--acc)}}
.card{{background:var(--panel);border:1px solid var(--line);border-radius:10px;padding:16px 20px;margin:12px 0}}
.card h3{{font-size:17px}}.card h3 a{{color:var(--ink)}}
.meta{{font-size:13px;color:var(--dim);font-family:var(--mono)}}
.badge{{display:inline-block;font:600 11.5px var(--sans);color:var(--good);border:1px solid var(--good);border-radius:20px;padding:1px 10px;vertical-align:2px;margin-left:8px}}
pre{{background:var(--panel);border:1px solid var(--line);border-radius:10px;padding:14px 16px;font:13.5px/1.6 var(--mono);overflow-x:auto;margin:12px 0}}
table{{width:100%;border-collapse:collapse;font-size:14px;margin:12px 0}}
th,td{{text-align:left;padding:8px 10px;border-bottom:1px solid var(--line);vertical-align:top}}
th{{color:var(--dim);font-size:11.5px;text-transform:uppercase;letter-spacing:.06em}}
td code{{font-family:var(--mono);font-size:13px}}
.eff{{color:var(--copper);font-family:var(--mono);font-size:12.5px}}
footer{{padding:34px 0;color:var(--dim);font-size:13.5px}}
</style></head><body>
<header><div class="wrap">
<a class="logo" href="/"><svg width="22" height="22" viewBox="0 0 32 32"><path d="M6 26 Q10 12 16 6 Q14 16 12 26 Z M13 26 Q17 13 23 7 Q20 17 18 26 Z M19 26 Q23 15 28 10 Q25 19 24 26 Z" fill="#b45309"/></svg>Claw <span style="color:var(--dim);font-weight:400">Registry</span></a>
<nav><a href="https://clawlang.dev">clawlang.dev</a><a href="https://clawlang.dev/docs.html">Docs</a><a href="https://github.com/LambdaQ-Labs/claw">GitHub</a></nav>
</div></header>
<div class="wrap">{body}</div>
<footer><div class="wrap">Every package here is AI-legible by rule: it publishes its definitions
(names, types, effects, docs), your tools ingest them on <code style="font-family:var(--mono)">claw add</code>.
· <a href="https://clawlang.dev/docs.html#packages">How packages work</a></div></footer>
</body></html>"##
    ))
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
pub async fn index_html(State(st): State<AppState>, Query(q): Query<SearchQ>) -> Html<String> {
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
        "Claw Registry — packages your AI understands",
        &format!(
            r#"<h1>Packages your AI understands</h1>
<p class="sub">Every package publishes its definitions — install one and your editor,
your assistant, and <code style="font-family:var(--mono)">claw ai</code> know it instantly.</p>
<form action="/" method="get"><input type="search" name="q" value="{}" placeholder="search packages…" aria-label="search packages"></form>
{results}
<pre># publish yours
claw publish        # exports your definitions automatically — that's the whole requirement</pre>"#,
            esc(&needle)
        ),
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
    Path(name): Path<String>,
) -> Html<String> {
    let rows = sqlx::query(
        "SELECT version, hash, size, published_at::text AS pub
         FROM packages WHERE name = $1 ORDER BY published_at DESC",
    )
    .bind(&name)
    .fetch_all(&st.pool)
    .await
    .unwrap_or_default();

    if rows.is_empty() {
        return shell(
            "not found — Claw Registry",
            &format!("<h1>{}</h1><p class=\"sub\">No such package.</p>", esc(&name)),
        );
    }

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
        &format!("{} — Claw Registry", esc(&name)),
        &format!(
            r#"<h1>{n}<span class="badge">✓ AI-legible</span></h1>
<div class="meta">latest {v} · content hash <code>{h}</code></div>
<pre>claw add {n}</pre>
{defs_html}
<h2 style="font-size:20px;margin-top:30px">Versions</h2>
<table><tr><th>version</th><th>size</th><th>published</th></tr>{versions}</table>"#,
            n = esc(&name),
            v = esc(&latest),
            h = esc(&hash.chars().take(16).collect::<String>()),
        ),
    )
}
