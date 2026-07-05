//! claw-corpus — synthetic training-corpus generator (WS-H).
//!
//! The cold-start problem is the thing that kills new AI-first languages:
//! a language with no code has no training data, so models are worst at
//! exactly the language they'd be used for. Claw's escape is to *make* the
//! data. This crate generates valid `(prompt, Def-JSON)` pairs directly
//! from a CDB — every pair is a real, in-scope, type-correct program the
//! grammar would accept — so a model can be fine-tuned toward the language
//! before any human writes a line of it.
//!
//! Generation here is property-based and self-labeling: we synthesize
//! programs that only reference symbols in the CDB (so they never
//! hallucinate by construction), pair them with a natural-language prompt,
//! and emit JSONL ready for supervised fine-tuning.
//!
//! Spec: master-plan WS-H (the 80% — the cold-start escape).

use claw_cdb::Cdb;
use claw_core::{Def, Expr, Type};
use serde::{Deserialize, Serialize};

/// One supervised training example.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Example {
    pub prompt: String,
    /// The target completion: a JSON array of one produced definition,
    /// in the exact Def-JSON protocol the benchmark runner expects.
    pub completion: String,
    /// Provenance: which in-scope symbols the completion uses. Every one
    /// is real — this corpus never teaches a hallucination.
    pub uses: Vec<String>,
}

/// Generate training examples from a CDB: for every in-scope symbol that
/// is a unary or binary function, synthesize a wrapper definition that
/// applies it, paired with a prompt describing the task. Deterministic.
pub fn generate(cdb: &Cdb) -> claw_cdb::Result<Vec<Example>> {
    let mut out = Vec::new();
    for (name, hash) in cdb.symbols()? {
        let def = cdb.get(&hash)?;
        if let Type::Fn(params, ret) = &def.ty {
            if let Some(ex) = wrapper_example(&name, params, ret, &def.effects) {
                out.push(ex);
            }
        }
    }
    Ok(out)
}

/// Synthesize `\a0.. -> name a0..` : a point-free wrapper that calls the
/// symbol on fresh params. Always type-correct and hallucination-free.
/// An effectful symbol's wrapper declares the same effect row — the corpus
/// teaches effect declaration exactly like it teaches in-scope references.
fn wrapper_example(name: &str, params: &[Type], ret: &Type, effects: &[String]) -> Option<Example> {
    if params.is_empty() || params.len() > 3 {
        return None;
    }
    // Param pool p0.. matches the Def-JSON output protocol + GBNF grammar.
    let param_names: Vec<String> = (0..params.len()).map(|i| format!("p{i}")).collect();
    // Reference the scope symbol BY NAME (Var), not by content hash (Ref):
    // a model can reproduce a name but never guess a hash. This keeps the
    // corpus in the same protocol the benchmark/eval use.
    let body = Expr::App {
        func: Box::new(Expr::Var(name.into())),
        args: param_names.iter().map(|p| Expr::Var(p.clone())).collect(),
    };
    let mut def = Def::new(
        Expr::Lam {
            params: param_names.clone(),
            body: Box::new(body),
        },
        Type::Fn(params.to_vec(), Box::new(ret.clone())),
    );
    def.effects = effects.to_vec();

    let sig = Type::Fn(params.to_vec(), Box::new(ret.clone()));
    let effect_note = if effects.is_empty() {
        String::new()
    } else {
        format!(" Declare its effects ({}).", effects.join(", "))
    };
    let prompt = format!(
        "Define a function `apply_{}` : {} that forwards its arguments to the in-scope `{}`. \
         Use only in-scope symbols.{}",
        name.replace('.', "_").to_lowercase().replace('!', ""),
        sig,
        name,
        effect_note
    );

    // Completion in the named Def-JSON protocol (with the def's own name).
    let value = serde_json::json!([{
        "name": format!("apply_{}", name.replace('.', "_").to_lowercase().replace('!', "")),
        "expr": def.expr,
        "ty": def.ty,
        "effects": def.effects,
        "deprecated": false,
        "doc": ""
    }]);

    Some(Example {
        prompt,
        completion: serde_json::to_string(&value).ok()?,
        uses: vec![name.to_string()],
    })
}

/// A built-in "standard library" scope: a rich set of typed symbols the
/// corpus can synthesize programs over, so a useful corpus exists with no
/// project to ingest. Deterministic.
pub fn stdlib_cdb() -> Cdb {
    use claw_core::{parse::parse_type, Expr, Lit};
    // Pure symbols: (name, signature). Effectful platform symbols (the `sys`
    // platform's hosted effects) are added below with their effect rows —
    // `Unit` stands for the surface `{}` return.
    let sigs: &[(&str, &str)] = &[
        ("Nat.add", "Nat, Nat -> Nat"),
        ("Nat.sub", "Nat, Nat -> Nat"),
        ("Nat.mul", "Nat, Nat -> Nat"),
        ("Nat.max", "Nat, Nat -> Nat"),
        ("Nat.min", "Nat, Nat -> Nat"),
        ("Nat.inc", "Nat -> Nat"),
        ("Nat.dec", "Nat -> Nat"),
        ("Nat.double", "Nat -> Nat"),
        ("Nat.half", "Nat -> Nat"),
        ("Nat.sqr", "Nat -> Nat"),
        ("Nat.isZero", "Nat -> Bool"),
        ("Nat.isEven", "Nat -> Bool"),
        ("Nat.isOdd", "Nat -> Bool"),
        ("Nat.isPositive", "Nat -> Bool"),
        ("Nat.eq", "Nat, Nat -> Bool"),
        ("Nat.lte", "Nat, Nat -> Bool"),
        ("Nat.toStr", "Nat -> Str"),
        ("Str.concat", "Str, Str -> Str"),
        ("Str.len", "Str -> Nat"),
        ("Str.isEmpty", "Str -> Bool"),
        ("Str.upper", "Str -> Str"),
        ("Bool.and", "Bool, Bool -> Bool"),
        ("Bool.or", "Bool, Bool -> Bool"),
        ("Bool.not", "Bool -> Bool"),
        ("Bool.if", "Bool, a, a -> a"),
        ("List.len", "List a -> Nat"),
        ("List.isEmpty", "List a -> Bool"),
        ("List.head", "List a -> Maybe a"),
        ("List.reverse", "List a -> List a"),
        ("Maybe.isSome", "Maybe a -> Bool"),
        ("Result.isOk", "Result a e -> Bool"),
    ];
    // The sys platform's hosted effects (platforms/sys): name, signature,
    // effect row. Wrappers/composes over these teach the model to reference
    // platform symbols AND declare the matching effects.
    let effectful: &[(&str, &str, &[&str])] = &[
        ("File.read!", "Str -> Str", &["Fs"]),
        ("Env.get!", "Str -> Str", &["Env"]),
        ("Stdout.line!", "Str -> Unit", &["Stdout"]),
    ];
    let mut cdb = Cdb::in_memory().expect("in-memory cdb");
    for (name, sig) in sigs {
        let ty = parse_type(sig).expect("valid stdlib sig");
        let def = Def::new(Expr::Lit(Lit::Str((*name).into())), ty);
        let h = cdb.put(&def).expect("put");
        cdb.bind(name, &h).expect("bind");
    }
    for (name, sig, effects) in effectful {
        let ty = parse_type(sig).expect("valid platform sig");
        let mut def = Def::new(Expr::Lit(Lit::Str((*name).into())), ty);
        def.effects = effects.iter().map(|e| e.to_string()).collect();
        let h = cdb.put(&def).expect("put");
        cdb.bind(name, &h).expect("bind");
    }
    cdb
}

/// Compose examples: for unary `g : A -> B` and unary `f : B -> C`, emit
/// `\p0 -> f (g p0)` : A -> C. Over the stdlib this yields many
/// type-correct, hallucination-free programs.
fn compose_examples(cdb: &Cdb) -> claw_cdb::Result<Vec<Example>> {
    let unary: Vec<(String, Type, Type, Vec<String>)> = cdb
        .symbols()?
        .into_iter()
        .filter_map(|(n, h)| {
            let d = cdb.get(&h).ok()?;
            if let Type::Fn(ps, ret) = &d.ty {
                if ps.len() == 1 {
                    return Some((n, ps[0].clone(), (**ret).clone(), d.effects.clone()));
                }
            }
            None
        })
        .collect();

    let mut out = Vec::new();
    for (gname, ga, gb, geff) in &unary {
        for (fname, fb, fc, feff) in &unary {
            // g : ga -> gb ; f : fb -> fc ; composable when gb unifies fb
            // (reference by name, not hash — see wrapper_example)
            if claw_core::unify(gb, fb).is_none() {
                continue;
            }
            let body = Expr::App {
                func: Box::new(Expr::Var(fname.clone())),
                args: vec![Expr::App {
                    func: Box::new(Expr::Var(gname.clone())),
                    args: vec![Expr::Var("p0".into())],
                }],
            };
            let ty = Type::Fn(vec![ga.clone()], Box::new(fc.clone()));
            let mut def = Def::new(
                Expr::Lam {
                    params: vec!["p0".into()],
                    body: Box::new(body),
                },
                ty.clone(),
            );
            // Effect row of a pipeline = union of its stages' rows, sorted
            // for determinism. Sound by construction (check_by_names agrees).
            let mut effects: Vec<String> = geff.iter().chain(feff.iter()).cloned().collect();
            effects.sort();
            effects.dedup();
            def.effects = effects;
            let dname = format!(
                "{}_then_{}",
                gname.replace('.', "_").to_lowercase().replace('!', ""),
                fname.replace('.', "_").to_lowercase().replace('!', "")
            );
            let effect_note = if def.effects.is_empty() {
                String::new()
            } else {
                format!(" Declare its effects ({}).", def.effects.join(", "))
            };
            let value = serde_json::json!([{
                "name": dname,
                "expr": def.expr, "ty": def.ty,
                "effects": def.effects, "deprecated": false, "doc": ""
            }]);
            out.push(Example {
                prompt: format!(
                    "Define `{dname}` : {ty} that applies `{gname}` then `{fname}`. Use only in-scope symbols.{effect_note}",
                ),
                completion: serde_json::to_string(&value).unwrap_or_default(),
                uses: vec![gname.clone(), fname.clone()],
            });
        }
    }
    Ok(out)
}

/// Literal-constant application: `\p0 -> sym(p0, K)` : Nat -> Nat, over
/// every binary Nat symbol and a spread of constants. This is the program
/// *shape* the P4 gate tasks need and the wrapper/compose corpus lacked;
/// teaching it broadly (many symbols × many K) generalizes rather than
/// memorizing any one task. Hallucination-free by construction.
fn literal_examples(cdb: &Cdb) -> claw_cdb::Result<Vec<Example>> {
    use claw_core::Lit;
    let binops: Vec<(String, Type)> = cdb
        .symbols()?
        .into_iter()
        .filter_map(|(n, h)| {
            let d = cdb.get(&h).ok()?;
            if let Type::Fn(ps, ret) = &d.ty {
                if ps.len() == 2
                    && matches!(&ps[0], Type::Named(x) if x=="Nat")
                    && matches!(&**ret, Type::Named(x) if x=="Nat")
                {
                    return Some((n, d.ty.clone()));
                }
            }
            None
        })
        .collect();

    let mut out = Vec::new();
    for (name, _ty) in &binops {
        for k in [1i64, 2, 3, 4, 5, 6, 7, 8, 9, 10, 20, 100] {
            let body = Expr::App {
                func: Box::new(Expr::Var(name.clone())),
                args: vec![Expr::Var("p0".into()), Expr::Lit(Lit::Int(k))],
            };
            let ty = Type::Fn(
                vec![Type::Named("Nat".into())],
                Box::new(Type::Named("Nat".into())),
            );
            let def = Def::new(
                Expr::Lam {
                    params: vec!["p0".into()],
                    body: Box::new(body),
                },
                ty.clone(),
            );
            let dname = format!("{}_{k}", name.replace('.', "_").to_lowercase());
            let value = serde_json::json!([{
                "name": dname, "expr": def.expr, "ty": def.ty,
                "effects": [], "deprecated": false, "doc": ""
            }]);
            out.push(Example {
                prompt: format!(
                    "Define `{dname}` : Nat -> Nat (parameter p0) that applies `{name}` to p0 and the constant {k}.",
                ),
                completion: serde_json::to_string(&value).unwrap_or_default(),
                uses: vec![name.clone()],
            });
        }
    }
    Ok(out)
}

/// Conditional programs: `\p0 -> Bool.if(q(p0), k1, k2)` : Nat -> Nat, over
/// every `Nat -> Bool` predicate `q` and a spread of constant pairs. This is
/// the branching shape the wrapper/compose/literal corpus lacked — an
/// out-of-shape class the P4 gate flagged. Requires `Bool.if` in scope;
/// hallucination-free by construction (uses q + Bool.if, both real).
fn conditional_examples(cdb: &Cdb) -> claw_cdb::Result<Vec<Example>> {
    use claw_core::Lit;
    if cdb.resolve("Bool.if").is_err() {
        return Ok(Vec::new());
    }
    // predicates: Nat -> Bool
    let preds: Vec<String> = cdb
        .symbols()?
        .into_iter()
        .filter_map(|(n, h)| {
            let d = cdb.get(&h).ok()?;
            if let Type::Fn(ps, ret) = &d.ty {
                if ps.len() == 1
                    && matches!(&ps[0], Type::Named(x) if x == "Nat")
                    && matches!(&**ret, Type::Named(x) if x == "Bool")
                {
                    return Some(n);
                }
            }
            None
        })
        .collect();

    let mut out = Vec::new();
    for q in &preds {
        for (k1, k2) in [(0i64, 1i64), (1, 0), (0, 100), (1, 100), (7, 42)] {
            let body = Expr::App {
                func: Box::new(Expr::Var("Bool.if".into())),
                args: vec![
                    Expr::App {
                        func: Box::new(Expr::Var(q.clone())),
                        args: vec![Expr::Var("p0".into())],
                    },
                    Expr::Lit(Lit::Int(k1)),
                    Expr::Lit(Lit::Int(k2)),
                ],
            };
            let ty = Type::Fn(
                vec![Type::Named("Nat".into())],
                Box::new(Type::Named("Nat".into())),
            );
            let def = Def::new(
                Expr::Lam {
                    params: vec!["p0".into()],
                    body: Box::new(body),
                },
                ty,
            );
            let dname = format!("{}_branch_{k1}_{k2}", q.replace('.', "_").to_lowercase());
            let value = serde_json::json!([{
                "name": dname, "expr": def.expr, "ty": def.ty,
                "effects": [], "deprecated": false, "doc": ""
            }]);
            out.push(Example {
                prompt: format!(
                    "Define `{dname}` : Nat -> Nat (parameter p0) that returns {k1} when `{q}` of p0 is true, else {k2}. Use `Bool.if`.",
                ),
                completion: serde_json::to_string(&value).unwrap_or_default(),
                uses: vec![q.clone(), "Bool.if".to_string()],
            });
        }
    }
    Ok(out)
}

/// Multi-definition programs: emit a two-def completion where the second def
/// calls the first — `step = \p0 -> f(p0)` then `twice = \p0 -> step(step p0)`.
/// Teaches the model to define a local helper and reference it as a sibling
/// (the other missing shape). `uses` lists only external CDB symbols; the
/// helper is defined in the same completion, so it's not a hallucination.
fn multidef_examples(cdb: &Cdb) -> claw_cdb::Result<Vec<Example>> {
    let unary: Vec<String> = cdb
        .symbols()?
        .into_iter()
        .filter_map(|(n, h)| {
            let d = cdb.get(&h).ok()?;
            if let Type::Fn(ps, ret) = &d.ty {
                if ps.len() == 1
                    && matches!(&ps[0], Type::Named(x) if x == "Nat")
                    && matches!(&**ret, Type::Named(x) if x == "Nat")
                {
                    return Some(n);
                }
            }
            None
        })
        .collect();

    let nat = || Type::Named("Nat".into());
    let unary_nat = || Type::Fn(vec![nat()], Box::new(nat()));
    let mut out = Vec::new();
    for f in &unary {
        let step = Def::new(
            Expr::Lam {
                params: vec!["p0".into()],
                body: Box::new(Expr::App {
                    func: Box::new(Expr::Var(f.clone())),
                    args: vec![Expr::Var("p0".into())],
                }),
            },
            unary_nat(),
        );
        let twice = Def::new(
            Expr::Lam {
                params: vec!["p0".into()],
                body: Box::new(Expr::App {
                    func: Box::new(Expr::Var("step".into())),
                    args: vec![Expr::App {
                        func: Box::new(Expr::Var("step".into())),
                        args: vec![Expr::Var("p0".into())],
                    }],
                }),
            },
            unary_nat(),
        );
        let value = serde_json::json!([
            {"name": "step", "expr": step.expr, "ty": step.ty, "effects": [], "deprecated": false, "doc": ""},
            {"name": "twice", "expr": twice.expr, "ty": twice.ty, "effects": [], "deprecated": false, "doc": ""}
        ]);
        out.push(Example {
            prompt: format!(
                "Define two functions: `step` : Nat -> Nat that applies `{f}` to p0, and `twice` : Nat -> Nat that applies `step` twice. Use only in-scope symbols.",
            ),
            completion: serde_json::to_string(&value).unwrap_or_default(),
            uses: vec![f.clone()],
        });
    }
    Ok(out)
}

/// Native-`If` programs: `\p0 -> if q(p0) then A else B`, with constant
/// and parameter branches. conditional_examples teaches the `Bool.if` CALL
/// shape; this teaches the lazy `If` SYNTAX — a general model needs both.
fn if_examples(cdb: &Cdb) -> claw_cdb::Result<Vec<Example>> {
    use claw_core::Lit;
    let nat = || Type::Named("Nat".into());
    let mut out = Vec::new();
    for q in &nat_preds(cdb)? {
        let cases: Vec<(Expr, Expr, &str)> = vec![
            (
                Expr::Lit(Lit::Int(1)),
                Expr::Lit(Lit::Int(0)),
                "returns 1 when it holds, else 0",
            ),
            (
                Expr::Var("p0".into()),
                Expr::Lit(Lit::Int(0)),
                "returns p0 when it holds, else 0",
            ),
        ];
        for (i, (then, els, desc)) in cases.into_iter().enumerate() {
            let def = Def::new(
                Expr::Lam {
                    params: vec!["p0".into()],
                    body: Box::new(Expr::If {
                        cond: Box::new(Expr::App {
                            func: Box::new(Expr::Var(q.clone())),
                            args: vec![Expr::Var("p0".into())],
                        }),
                        then: Box::new(then),
                        els: Box::new(els),
                    }),
                },
                Type::Fn(vec![nat()], Box::new(nat())),
            );
            let dname = format!("{}_if{i}", q.replace('.', "_").to_lowercase());
            out.push(named_example(
                &dname,
                def,
                format!("Define `{dname}` : Nat -> Nat (parameter p0) using a native `if` on `{q}` of p0: it {desc}."),
                vec![q.clone()],
            ));
        }
    }
    Ok(out)
}

/// Let-binding programs: `\p0 -> let p8 = f(p0) in g(p8)`. Binder names
/// come from the same p-pool as parameters — one pool, one protocol rule.
fn let_examples(cdb: &Cdb) -> claw_cdb::Result<Vec<Example>> {
    let unary = nat_unary(cdb)?;
    let nat = || Type::Named("Nat".into());
    let mut out = Vec::new();
    for g in &unary {
        for f in &unary {
            let def = Def::new(
                Expr::Lam {
                    params: vec!["p0".into()],
                    body: Box::new(Expr::Let {
                        name: "p8".into(),
                        value: Box::new(Expr::App {
                            func: Box::new(Expr::Var(g.clone())),
                            args: vec![Expr::Var("p0".into())],
                        }),
                        body: Box::new(Expr::App {
                            func: Box::new(Expr::Var(f.clone())),
                            args: vec![Expr::Var("p8".into())],
                        }),
                    }),
                },
                Type::Fn(vec![nat()], Box::new(nat())),
            );
            let dname = format!(
                "{}_let_{}",
                g.replace('.', "_").to_lowercase(),
                f.replace('.', "_").to_lowercase()
            );
            out.push(named_example(
                &dname,
                def,
                format!("Define `{dname}` : Nat -> Nat (parameter p0): bind p8 = `{g}` of p0 with a let, then return `{f}` of p8."),
                vec![g.clone(), f.clone()],
            ));
        }
    }
    Ok(out)
}

/// Pattern-matching over Maybe/Result plus tag construction — how a real
/// program handles absence and failure. Pattern binders use the p-pool.
fn match_examples(_cdb: &Cdb) -> claw_cdb::Result<Vec<Example>> {
    use claw_core::{Lit, Pat};
    let nat = || Type::Named("Nat".into());
    let maybe_nat = || Type::App("Maybe".into(), vec![nat()]);
    let result_nat = || Type::App("Result".into(), vec![nat(), Type::Var("e".into())]);
    let mut out = Vec::new();

    for (k, kdesc) in [(0i64, "zero"), (1, "one"), (100, "100")] {
        let def = Def::new(
            Expr::Lam {
                params: vec!["p0".into()],
                body: Box::new(Expr::Match(
                    Box::new(Expr::Var("p0".into())),
                    vec![
                        (
                            Pat::Tag("Some".into(), vec![Pat::Var("p1".into())]),
                            Expr::Var("p1".into()),
                        ),
                        (Pat::Tag("None".into(), vec![]), Expr::Lit(Lit::Int(k))),
                    ],
                )),
            },
            Type::Fn(vec![maybe_nat()], Box::new(nat())),
        );
        let dname = format!("maybe_or_{k}");
        out.push(named_example(
            &dname,
            def,
            format!("Define `{dname}` : Maybe Nat -> Nat (parameter p0): match on p0 — a Some yields its payload (bind it p1), a None yields {kdesc}."),
            vec![],
        ));

        let def = Def::new(
            Expr::Lam {
                params: vec!["p0".into()],
                body: Box::new(Expr::Match(
                    Box::new(Expr::Var("p0".into())),
                    vec![
                        (
                            Pat::Tag("Ok".into(), vec![Pat::Var("p1".into())]),
                            Expr::Var("p1".into()),
                        ),
                        (Pat::Tag("Err".into(), vec![Pat::Wild]), Expr::Lit(Lit::Int(k))),
                    ],
                )),
            },
            Type::Fn(vec![result_nat()], Box::new(nat())),
        );
        let dname = format!("ok_or_{k}");
        out.push(named_example(
            &dname,
            def,
            format!("Define `{dname}` : Result Nat e -> Nat (parameter p0): match on p0 — Ok yields its payload (bind it p1), any Err yields {kdesc}."),
            vec![],
        ));
    }

    let def = Def::new(
        Expr::Lam {
            params: vec!["p0".into()],
            body: Box::new(Expr::Tag("Some".into(), vec![Expr::Var("p0".into())])),
        },
        Type::Fn(vec![nat()], Box::new(maybe_nat())),
    );
    out.push(named_example(
        "wrap_some",
        def,
        "Define `wrap_some` : Nat -> Maybe Nat (parameter p0) that wraps p0 in the Some tag.".into(),
        vec![],
    ));
    let def = Def::new(
        Expr::Lam {
            params: vec!["p0".into()],
            body: Box::new(Expr::Tag("Ok".into(), vec![Expr::Var("p0".into())])),
        },
        Type::Fn(vec![nat()], Box::new(result_nat())),
    );
    out.push(named_example(
        "wrap_ok",
        def,
        "Define `wrap_ok` : Nat -> Result Nat e (parameter p0) that wraps p0 in the Ok tag.".into(),
        vec![],
    ));
    Ok(out)
}

/// Recursive definitions: a named def calling ITSELF — legal under the
/// named-defs protocol. Guarded by a base case; the interp is step-bounded.
fn recursion_examples(cdb: &Cdb) -> claw_cdb::Result<Vec<Example>> {
    use claw_core::Lit;
    if cdb.resolve("Nat.isZero").is_err() || cdb.resolve("Nat.dec").is_err() {
        return Ok(Vec::new());
    }
    let nat = || Type::Named("Nat".into());
    let mut out = Vec::new();
    let combos: Vec<(&str, &str, &str)> = vec![
        ("sum_to", "Nat.add", "the sum 0 + 1 + ... + p0"),
        ("count_down", "Nat.max", "the maximum along the countdown"),
    ];
    for (dname, op, desc) in combos {
        let body = Expr::If {
            cond: Box::new(Expr::App {
                func: Box::new(Expr::Var("Nat.isZero".into())),
                args: vec![Expr::Var("p0".into())],
            }),
            then: Box::new(Expr::Lit(Lit::Int(0))),
            els: Box::new(Expr::App {
                func: Box::new(Expr::Var(op.into())),
                args: vec![
                    Expr::Var("p0".into()),
                    Expr::App {
                        func: Box::new(Expr::Var(dname.into())),
                        args: vec![Expr::App {
                            func: Box::new(Expr::Var("Nat.dec".into())),
                            args: vec![Expr::Var("p0".into())],
                        }],
                    },
                ],
            }),
        };
        let def = Def::new(
            Expr::Lam {
                params: vec!["p0".into()],
                body: Box::new(body),
            },
            Type::Fn(vec![nat()], Box::new(nat())),
        );
        out.push(named_example(
            dname,
            def,
            format!("Define `{dname}` : Nat -> Nat (parameter p0) RECURSIVELY: when `Nat.isZero` of p0, return 0; otherwise combine p0 (via `{op}`) with `{dname}` of `Nat.dec` of p0 — computing {desc}."),
            vec!["Nat.isZero".into(), "Nat.dec".into(), op.into()],
        ));
    }
    Ok(out)
}

// --- small shared helpers for the shape classes ----------------------------

fn nat_preds(cdb: &Cdb) -> claw_cdb::Result<Vec<String>> {
    Ok(cdb
        .symbols()?
        .into_iter()
        .filter_map(|(n, h)| {
            let d = cdb.get(&h).ok()?;
            if let Type::Fn(ps, ret) = &d.ty {
                if ps.len() == 1
                    && matches!(&ps[0], Type::Named(x) if x == "Nat")
                    && matches!(&**ret, Type::Named(x) if x == "Bool")
                {
                    return Some(n);
                }
            }
            None
        })
        .collect())
}

fn nat_unary(cdb: &Cdb) -> claw_cdb::Result<Vec<String>> {
    Ok(cdb
        .symbols()?
        .into_iter()
        .filter_map(|(n, h)| {
            let d = cdb.get(&h).ok()?;
            if let Type::Fn(ps, ret) = &d.ty {
                if ps.len() == 1
                    && matches!(&ps[0], Type::Named(x) if x == "Nat")
                    && matches!(&**ret, Type::Named(x) if x == "Nat")
                {
                    return Some(n);
                }
            }
            None
        })
        .collect())
}

/// Package a named def as an Example in the output protocol.
fn named_example(name: &str, def: Def, prompt: String, uses: Vec<String>) -> Example {
    let value = serde_json::json!([{
        "name": name, "expr": def.expr, "ty": def.ty,
        "effects": def.effects, "deprecated": false, "doc": ""
    }]);
    Example {
        prompt,
        completion: serde_json::to_string(&value).unwrap_or_default(),
        uses,
    }
}

/// Instruction prefixes for prompt augmentation. Same target completion,
/// varied phrasing — teaches the model the output protocol robustly rather
/// than memorizing one instruction style. Standard SFT augmentation.
const PROMPT_PREFIXES: &[&str] = &[
    "",
    "In Claw, ",
    "Write Claw code: ",
    "Task — ",
    "Using only the in-scope symbols, ",
];

/// Multiply examples by re-phrasing each prompt with every prefix.
pub fn augment(examples: &[Example]) -> Vec<Example> {
    let mut out = Vec::with_capacity(examples.len() * PROMPT_PREFIXES.len());
    for ex in examples {
        for pre in PROMPT_PREFIXES {
            let prompt = if pre.is_empty() {
                ex.prompt.clone()
            } else {
                // lowercase the first letter after a prefix for readability
                let mut c = ex.prompt.chars();
                let first = c.next().map(|f| f.to_ascii_lowercase()).unwrap_or_default();
                format!("{pre}{first}{}", c.as_str())
            };
            out.push(Example {
                prompt,
                completion: ex.completion.clone(),
                uses: ex.uses.clone(),
            });
        }
    }
    out
}

/// The full synthetic corpus over the built-in stdlib: (wrappers + compose)
/// × prompt augmentation. This is what `claw corpus gen --stdlib` emits —
/// the training seed for the bundled model.
pub fn generate_stdlib() -> claw_cdb::Result<Vec<Example>> {
    let cdb = stdlib_cdb();
    let mut base = generate(&cdb)?;
    base.extend(compose_examples(&cdb)?);
    base.extend(literal_examples(&cdb)?);
    base.extend(conditional_examples(&cdb)?);
    base.extend(multidef_examples(&cdb)?);
    base.extend(if_examples(&cdb)?);
    base.extend(let_examples(&cdb)?);
    base.extend(match_examples(&cdb)?);
    base.extend(recursion_examples(&cdb)?);
    Ok(augment(&base))
}

/// Serialize examples to JSONL (one JSON object per line) — the standard
/// supervised-fine-tuning input format.
pub fn to_jsonl(examples: &[Example]) -> String {
    examples
        .iter()
        .filter_map(|e| serde_json::to_string(e).ok())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use claw_core::Lit;

    fn named(n: &str) -> Type {
        Type::Named(n.into())
    }

    fn seed_cdb() -> Cdb {
        let mut cdb = Cdb::in_memory().unwrap();
        // Nat.add : Nat, Nat -> Nat
        let add = Def::new(
            Expr::Lit(Lit::Int(0)),
            Type::Fn(vec![named("Nat"), named("Nat")], Box::new(named("Nat"))),
        );
        let h = cdb.put(&add).unwrap();
        cdb.bind("Nat.add", &h).unwrap();
        // Nat.zero : Nat  (not a function → skipped)
        let z = Def::new(Expr::Lit(Lit::Int(0)), named("Nat"));
        let zh = cdb.put(&z).unwrap();
        cdb.bind("Nat.zero", &zh).unwrap();
        cdb
    }

    #[test]
    fn generates_wrapper_for_functions_only() {
        let cdb = seed_cdb();
        let examples = generate(&cdb).unwrap();
        assert_eq!(examples.len(), 1, "only Nat.add is a function");
        assert_eq!(examples[0].uses, vec!["Nat.add"]);
        assert!(examples[0].prompt.contains("Nat.add"));
    }

    #[test]
    fn completion_is_valid_named_def_json() {
        let cdb = seed_cdb();
        let ex = &generate(&cdb).unwrap()[0];
        // must parse as an array with a name + expr + ty
        let v: serde_json::Value = serde_json::from_str(&ex.completion).unwrap();
        assert!(v.is_array());
        assert_eq!(v[0]["name"], "apply_nat_add");
        assert!(v[0]["expr"].get("Lam").is_some());
    }

    #[test]
    fn corpus_only_references_real_symbols() {
        // The whole point: no synthesized example teaches a hallucination.
        let cdb = seed_cdb();
        let known: std::collections::BTreeSet<String> =
            cdb.symbols().unwrap().into_iter().map(|(n, _)| n).collect();
        for ex in generate(&cdb).unwrap() {
            for u in &ex.uses {
                assert!(known.contains(u), "corpus used unknown symbol {u}");
            }
        }
    }

    #[test]
    fn effectful_wrappers_declare_their_effects() {
        let examples = generate_stdlib().unwrap();
        // File.read! wrapper: declares Fs, prompt asks for the declaration.
        let read = examples
            .iter()
            .find(|e| e.uses == vec!["File.read!".to_string()] && e.prompt.contains("forwards"))
            .expect("File.read! wrapper example");
        let v: serde_json::Value = serde_json::from_str(&read.completion).unwrap();
        assert_eq!(v[0]["effects"], serde_json::json!(["Fs"]));
        assert_eq!(v[0]["name"], "apply_file_read");
        assert!(read.prompt.contains("Declare its effects (Fs)"));
        // read-then-print pipeline: effect row is the sorted union.
        let pipe = examples
            .iter()
            .find(|e| {
                e.uses.contains(&"File.read!".to_string())
                    && e.uses.contains(&"Stdout.line!".to_string())
            })
            .expect("File.read! -> Stdout.line! compose example");
        let v: serde_json::Value = serde_json::from_str(&pipe.completion).unwrap();
        assert_eq!(v[0]["effects"], serde_json::json!(["Fs", "Stdout"]));
        // No bang leaks into produced def names.
        for ex in &examples {
            let v: serde_json::Value = serde_json::from_str(&ex.completion).unwrap();
            for d in v.as_array().unwrap() {
                assert!(!d["name"].as_str().unwrap().contains('!'));
            }
        }
    }

    #[test]
    fn stdlib_corpus_is_large_and_clean() {
        let examples = generate_stdlib().unwrap();
        // (wrappers + compositions) × 5 prompt variants
        assert!(
            examples.len() > 250,
            "expected a sizeable corpus, got {}",
            examples.len()
        );
        // every example references only real stdlib symbols
        let cdb = stdlib_cdb();
        let known: std::collections::BTreeSet<String> =
            cdb.symbols().unwrap().into_iter().map(|(n, _)| n).collect();
        for ex in &examples {
            for u in &ex.uses {
                assert!(known.contains(u), "corpus used unknown symbol {u}");
            }
        }
    }

    #[test]
    fn jsonl_is_one_object_per_line() {
        let cdb = seed_cdb();
        let jsonl = to_jsonl(&generate(&cdb).unwrap());
        for line in jsonl.lines() {
            let _: Example = serde_json::from_str(line).expect("each line is an Example");
        }
    }
}
