//! achuk-play — the REAL Achuk engine, compiled to WebAssembly.
//!
//! The playground used to ship a hand-written JavaScript mirror of the
//! type engine; this crate replaces it with the actual achuk-core parser,
//! unifier, and step-bounded interpreter plus the actual achuk-constraint
//! grammar projection — running in the visitor's browser. No servers, no
//! drift between the demo and the language.
//!
//! Scope is held in-memory (the CDB proper is SQLite and stays native);
//! candidates re-implements the CDB's freshen+unify query over that
//! in-memory scope — same semantics, ~20 lines.

use achuk_constraint::{gbnf, Continuation};
use achuk_core::interp::{self, Resolver, Value};
use achuk_core::render::render_def;
use achuk_core::{freshen, parse::parse_type, unify, Def, Expr, Hash, Lit};
use serde::Deserialize;
use std::cell::RefCell;
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

thread_local! {
    static SCOPE: RefCell<Vec<(String, Def)>> = const { RefCell::new(Vec::new()) };
}

#[derive(Deserialize)]
struct ScopeEntry {
    name: String,
    ty: String,
    #[serde(default)]
    effects: Vec<String>,
}

#[derive(Deserialize)]
struct NamedDef {
    #[serde(default)]
    name: Option<String>,
    #[serde(flatten)]
    def: Def,
}

fn err(e: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&e.to_string())
}

/// Load the scope: a JSON array of { name, ty, effects? }.
#[wasm_bindgen]
pub fn set_scope(json: &str) -> Result<usize, JsValue> {
    let entries: Vec<ScopeEntry> = serde_json::from_str(json).map_err(err)?;
    let mut out = Vec::with_capacity(entries.len());
    for e in entries {
        let ty = parse_type(&e.ty).map_err(|er| err(format!("{}: {er}", e.name)))?;
        let mut def = Def::new(Expr::Lit(Lit::Str(e.name.clone())), ty);
        def.effects = e.effects;
        out.push((e.name, def));
    }
    let n = out.len();
    SCOPE.with(|s| *s.borrow_mut() = out);
    Ok(n)
}

/// Every symbol in scope, `name : type` per line.
#[wasm_bindgen]
pub fn symbols() -> String {
    SCOPE.with(|s| {
        s.borrow()
            .iter()
            .map(|(n, d)| {
                let eff = if d.effects.is_empty() {
                    String::new()
                } else {
                    format!("  [effects: {}]", d.effects.join(", "))
                };
                format!("{n} : {}{eff}", d.ty)
            })
            .collect::<Vec<_>>()
            .join("\n")
    })
}

/// Type-directed search: which scope symbols unify with this signature?
/// The same freshen+unify query the native CDB runs.
#[wasm_bindgen]
pub fn candidates(sig: &str) -> Result<String, JsValue> {
    let query = parse_type(sig).map_err(err)?;
    let hits = SCOPE.with(|s| {
        s.borrow()
            .iter()
            .filter(|(_, d)| unify(&query, &freshen(&d.ty, "$c.")).is_some())
            .map(|(n, d)| format!("{n} : {}", d.ty))
            .collect::<Vec<_>>()
    });
    Ok(if hits.is_empty() {
        "(nothing in scope fits)".into()
    } else {
        hits.join("\n")
    })
}

/// The decode grammar for the current scope — the actual GBNF projection
/// llama.cpp consumes, from the actual achuk-constraint crate.
#[wasm_bindgen]
pub fn grammar() -> String {
    let conts = SCOPE.with(|s| {
        s.borrow()
            .iter()
            .map(|(n, d)| Continuation {
                name: n.clone(),
                hash: d.hash(),
                ty: d.ty.clone(),
                effects: d.effects.clone(),
                subst: Default::default(),
            })
            .collect::<Vec<_>>()
    });
    gbnf::def_json_grammar(&conts)
}

/// Check a Def-JSON array against the scope: hallucinated references and
/// effect-row soundness (name-based, mirroring the native grader).
#[wasm_bindgen]
pub fn check_defs(json: &str) -> Result<String, JsValue> {
    let defs: Vec<NamedDef> = serde_json::from_str(json).map_err(err)?;
    let scope: BTreeMap<String, Vec<String>> = SCOPE.with(|s| {
        s.borrow()
            .iter()
            .map(|(n, d)| (n.clone(), d.effects.clone()))
            .collect()
    });
    let defined: Vec<String> = defs.iter().filter_map(|d| d.name.clone()).collect();

    let mut halluc = Vec::new();
    let mut required: Vec<String> = Vec::new();
    let mut declared: Vec<String> = Vec::new();
    for d in &defs {
        declared.extend(d.def.effects.iter().cloned());
        for v in d.def.expr.free_vars() {
            if let Some(effs) = scope.get(&v) {
                required.extend(effs.iter().cloned());
            } else if !defined.contains(&v) {
                halluc.push(v);
            }
        }
    }
    halluc.sort();
    halluc.dedup();
    required.sort();
    required.dedup();
    let missing: Vec<String> = required
        .into_iter()
        .filter(|r| !declared.contains(r))
        .collect();

    Ok(if halluc.is_empty() && missing.is_empty() {
        "OK — every reference exists, effect rows are sound".to_string()
    } else {
        let mut msg = String::new();
        if !halluc.is_empty() {
            msg.push_str(&format!("hallucinated: {}\n", halluc.join(", ")));
        }
        if !missing.is_empty() {
            msg.push_str(&format!("undeclared effects: {}", missing.join(", ")));
        }
        msg.trim_end().to_string()
    })
}

/// Render a Def-JSON array as .achuk source.
#[wasm_bindgen]
pub fn render(json: &str) -> Result<String, JsValue> {
    let defs: Vec<NamedDef> = serde_json::from_str(json).map_err(err)?;
    Ok(defs
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let name = d.name.clone().unwrap_or_else(|| format!("def{i}"));
            render_def(&name, &d.def)
        })
        .collect::<Vec<_>>()
        .join("\n\n"))
}

struct PlayResolver {
    defs: BTreeMap<String, Expr>,
}

impl Resolver for PlayResolver {
    fn resolve(&self, _h: &Hash) -> Option<Expr> {
        None
    }
    fn name_of(&self, _h: &Hash) -> Option<String> {
        None
    }
    fn resolve_name(&self, name: &str) -> Option<Expr> {
        self.defs.get(name).cloned()
    }
}

fn show(v: &Value) -> String {
    match v {
        Value::Int(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Str(s) => format!("{s:?}"),
        Value::List(xs) => format!("[{}]", xs.iter().map(show).collect::<Vec<_>>().join(", ")),
        Value::Ok(x) => format!("Ok({})", show(x)),
        Value::Err(x) => format!("Err({})", show(x)),
        Value::Tag(n, args) if args.is_empty() => n.clone(),
        Value::Tag(n, args) => {
            format!("{n}({})", args.iter().map(show).collect::<Vec<_>>().join(", "))
        }
        Value::Record(fs) => format!(
            "{{ {} }}",
            fs.iter()
                .map(|(k, v)| format!("{k}: {}", show(v)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Value::Closure(..) => "<function>".into(),
        Value::Builtin(n) => format!("<builtin {n}>"),
    }
}

/// Run `name(args…)` over a Def-JSON array with the REAL step-bounded
/// interpreter. Args: JSON array of ints/strings/bools.
#[wasm_bindgen]
pub fn run(defs_json: &str, name: &str, args_json: &str) -> Result<String, JsValue> {
    let defs: Vec<NamedDef> = serde_json::from_str(defs_json).map_err(err)?;
    let args: Vec<serde_json::Value> = serde_json::from_str(args_json).map_err(err)?;

    let mut map = BTreeMap::new();
    for (i, d) in defs.iter().enumerate() {
        map.insert(
            d.name.clone().unwrap_or_else(|| format!("def{i}")),
            d.def.expr.clone(),
        );
    }
    let resolver = PlayResolver { defs: map };

    let call = Expr::App {
        func: Box::new(Expr::Var(name.to_string())),
        args: args
            .iter()
            .map(|a| match a {
                serde_json::Value::Number(n) => Expr::Lit(Lit::Int(n.as_i64().unwrap_or(0))),
                serde_json::Value::String(s) => Expr::Lit(Lit::Str(s.clone())),
                serde_json::Value::Bool(b) => Expr::Tag(
                    if *b { "True".into() } else { "False".into() },
                    vec![],
                ),
                other => Expr::Lit(Lit::Str(other.to_string())),
            })
            .collect(),
    };
    match interp::eval(&call, &Default::default(), &resolver) {
        Ok(v) => Ok(show(&v)),
        Err(e) => Err(err(format!("{e:?}"))),
    }
}

/// Parse and echo a type signature — the "is this a valid type?" probe.
#[wasm_bindgen]
pub fn parse_sig(src: &str) -> Result<String, JsValue> {
    parse_type(src).map(|t| t.to_string()).map_err(err)
}
