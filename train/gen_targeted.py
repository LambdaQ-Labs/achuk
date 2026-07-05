#!/usr/bin/env python3
"""Targeted corpus examples for the three re-gate failure classes.

The 1030-example corpus taught op+literal shapes, but never the benchmark's
phrasings — "(parameter p0 named x)" and "computes Op of x and K" — so the
tuned model emitted {"Var":"10"} for literals, leaked prompt names (a, b, c)
into bodies, and invented Nat.clamp instead of composing min/max.

Generates bench-STYLE (phrasing/shape) but not bench-CONTENT examples:
function names and (name, op, constant) combos are disjoint from every
bench/tasks-large family, so the gate still measures generalization.

    python gen_targeted.py > targeted.jsonl   # merged into the corpus-v3 build
"""
import json

OPS = {
    "Nat.max": "Nat.max", "Nat.min": "Nat.min",
    "Nat.add": "Nat.add", "Nat.mul": "Nat.mul", "Nat.sub": "Nat.sub",
}
NAT = {"Named": "Nat"}


def fn_ty(n):
    return {"Fn": [[NAT] * n, NAT]}


def lam(params, body):
    return {"Lam": {"params": params, "body": body}}


def app(f, args):
    return {"App": {"func": {"Var": f}, "args": args}}


def var(n):
    return {"Var": n}


def lit(k):
    return {"Lit": {"Int": k}}


def d(name, expr, ty):
    return [{"deprecated": False, "doc": "", "effects": [], "expr": expr,
             "name": name, "ty": ty}]


out = []


def emit(prompt, defs, uses):
    out.append({"prompt": prompt, "completion":
                json.dumps(defs, sort_keys=True, separators=(",", ":")),
                "uses": uses})


# --- Class A: op(param, literal) under the bench phrasings -----------------
# Bench families use constants {1,2,3,4,5,7,10,100} with names atleast/atmost/
# add/mul — our names differ AND constants differ, so no (name,op,K) overlap.
A_NAMES = {
    "Nat.max": "floor_at", "Nat.min": "cap_at",
    "Nat.add": "plus", "Nat.mul": "times", "Nat.sub": "minus",
}
PHRASINGS = [
    "Define `{name}` : Nat -> Nat (parameter p0 named x) that computes {op} of x and {k}.",
    "Define `{name}` : Nat -> Nat (parameter p0 named n) that computes {op} of n and {k}.",
    "Define `{name}` : Nat -> Nat (parameter p0 named x) that applies {op} to x and the constant {k}.",
    "Define `{name}` : Nat -> Nat (parameter p0 named value) that returns {op} of value and {k}. Use only in-scope symbols.",
]
for op, base in A_NAMES.items():
    for k in (6, 8, 9, 11, 12, 15, 20, 25, 30, 42, 50, 64):
        name = f"{base}_{k}"
        for i, ph in enumerate(PHRASINGS):
            # vary literal position on half the examples (op arg order)
            args = [var("p0"), lit(k)] if i % 2 == 0 else [lit(k), var("p0")]
            emit(ph.format(name=name, op=op, k=k),
                 d(name, lam(["p0"], app(op, args)), fn_ty(1)), [op])

# --- Class B: 3-param nested composition with "named a, b, c" ---------------
# Teaches: p-pool in the body despite human names in the prompt, and that
# 3-ary results come from NESTING the binary op (no flat 3-arg App).
TRIPLES = [
    ("biggest3", "Nat.max", "returns the largest of the three arguments"),
    ("smallest3", "Nat.min", "returns the smallest of the three arguments"),
    ("total3", "Nat.add", "sums the three arguments"),
    ("product3", "Nat.mul", "multiplies the three arguments"),
]
NAMESETS = [("a", "b", "c"), ("x", "y", "z"), ("m", "n", "o"), ("first", "second", "third")]
for name, op, desc in TRIPLES:
    for ns in NAMESETS:
        prompt = (f"Define `{name}` : Nat, Nat, Nat -> Nat (parameters p0, p1, p2 "
                  f"named {ns[0]}, {ns[1]}, {ns[2]}) that {desc} using only in-scope symbols.")
        body = app(op, [app(op, [var("p0"), var("p1")]), var("p2")])
        emit(prompt, d(name, lam(["p0", "p1", "p2"], body), fn_ty(3)), [op])

# --- Class B2: bound/restrict = min(max(x, lo), hi) — the clamp SHAPE -------
# (bench's task is named `clamp`; these teach the composition under other
# names so `Nat.clamp` stops being the most likely continuation)
CLAMPISH = [
    ("bound", ("x", "lo", "hi")), ("restrict", ("v", "low", "high")),
    ("between", ("n", "least", "most")), ("limit", ("x", "floor", "ceil")),
    ("confine", ("x", "lo", "hi")), ("pin", ("value", "lo", "hi")),
    ("hold_within", ("n", "lo", "hi")), ("snap_range", ("x", "min", "max")),
]
# Two phrasings per name: the compositional spell-out and the bench-style
# "clamps X into [lo, hi]" wording (v3 re-gate showed the model regresses to
# a hallucinated Nat.clamp exactly when it sees the "clamps into" phrasing
# it never trained on — the names stay disjoint from the bench's `clamp`).
for name, ns in CLAMPISH:
    for flip in (False, True):
        if flip:
            body = app("Nat.max", [app("Nat.min", [var("p0"), var("p2")]), var("p1")])
            how = f"computes Nat.max of (Nat.min of {ns[0]} and {ns[2]}) and {ns[1]}"
        else:
            body = app("Nat.min", [app("Nat.max", [var("p0"), var("p1")]), var("p2")])
            how = f"computes Nat.min of (Nat.max of {ns[0]} and {ns[1]}) and {ns[2]}"
        spelled = (f"Define `{name}` : Nat, Nat, Nat -> Nat (parameters p0, p1, p2 "
                   f"named {ns[0]}, {ns[1]}, {ns[2]}) that keeps {ns[0]} within "
                   f"[{ns[1]}, {ns[2]}]: it {how}. Use only in-scope symbols.")
        clampy = (f"Define `{name}` : Nat, Nat, Nat -> Nat (parameters p0, p1, p2 "
                  f"named {ns[0]}, {ns[1]}, {ns[2]}) that clamps {ns[0]} into "
                  f"[{ns[1]}, {ns[2]}]. Assume {ns[1]} <= {ns[2]}. Use only in-scope symbols.")
        defs = d(name, lam(["p0", "p1", "p2"], body), fn_ty(3))
        emit(spelled, defs, ["Nat.max", "Nat.min"])
        emit(clampy, defs, ["Nat.max", "Nat.min"])

# --- Class D: platform effects under bench-style phrasing -------------------
# The sys platform's hosted effects. Names are disjoint from the bench's
# gen-plat-* tasks (print_file vs cat_file, dump_env vs show_env, ...) so the
# gate measures the SHAPE (effectful call + declared row), not memorization.
EFF = {"File.read!": ["Fs"], "Env.get!": ["Env"], "Stdout.line!": ["Stdout"]}
STR, UNIT = {"Named": "Str"}, {"Named": "Unit"}


def deff(name, expr, ty, effects):
    return [{"deprecated": False, "doc": "", "effects": sorted(effects),
             "expr": expr, "name": name, "ty": ty}]


PLAT_TARGETED = [
    # wrappers, varied names/param-names
    ("load_file", "path", "File.read!", STR, STR,
     "reads the file at {p} with `File.read!`"),
    ("fetch_env", "key", "Env.get!", STR, STR,
     "reads the environment variable named {p} with `Env.get!`"),
    ("emit_line", "text", "Stdout.line!", STR, UNIT,
     "prints {p} with `Stdout.line!`"),
]
for name, p, sym, a, r, how in PLAT_TARGETED:
    prompt = (f"Define `{name}` : Str -> {'Unit' if r is UNIT else 'Str'} "
              f"(parameter p0 named {p}) that {how.format(p=p)}. "
              f"Declare its effects ({', '.join(EFF[sym])}).")
    emit(prompt, deff(name, lam(["p0"], app(sym, [var("p0")])),
                      {"Fn": [[a], r]}, EFF[sym]), [sym])

PLAT_PIPES = [
    ("print_file", "path", "File.read!", "Stdout.line!", UNIT,
     "reads the file at {p} with `File.read!` and prints the contents with `Stdout.line!`"),
    ("dump_env", "key", "Env.get!", "Stdout.line!", UNIT,
     "reads the environment variable {p} with `Env.get!` and prints it with `Stdout.line!`"),
    ("load_upper", "path", "File.read!", "Str.upper", STR,
     "reads the file at {p} with `File.read!` and uppercases the contents with `Str.upper`"),
]
for name, p, g, f, r, how in PLAT_PIPES:
    effects = sorted(set(EFF.get(g, []) + EFF.get(f, [])))
    prompt = (f"Define `{name}` : Str -> {'Unit' if r is UNIT else 'Str'} "
              f"(parameter p0 named {p}) that {how.format(p=p)}. "
              f"Declare its effects ({', '.join(effects)}).")
    body = app(f, [app(g, [var("p0")])])
    emit(prompt, deff(name, lam(["p0"], body), {"Fn": [[STR], r]}, effects),
         [g, f])

# pure Str shape with an explicitly-empty effect row (the negative case)
for name, ns in [("glue2", ("a", "b")), ("join_pair", ("left", "right"))]:
    prompt = (f"Define `{name}` : Str, Str -> Str (parameters p0, p1 named "
              f"{ns[0]}, {ns[1]}) that concatenates {ns[0]} and {ns[1]} with the "
              f"in-scope `Str.concat`. It performs no effects — declare an empty effect row.")
    emit(prompt, deff(name, lam(["p0", "p1"], app("Str.concat", [var("p0"), var("p1")])),
                      {"Fn": [[STR, STR], STR]}, []), ["Str.concat"])

# --- Class C: 2-param forwards with "named" phrasing (p-pool reinforcement) -
for op in OPS:
    for ns in (("a", "b"), ("x", "y"), ("left", "right")):
        name = "combine_" + op.split(".")[1]
        prompt = (f"Define `{name}` : Nat, Nat -> Nat (parameters p0, p1 named "
                  f"{ns[0]}, {ns[1]}) that forwards {ns[0]} and {ns[1]} to the "
                  f"in-scope `{op}`.")
        emit(prompt, d(name, lam(["p0", "p1"], app(op, [var("p0"), var("p1")])), fn_ty(2)), [op])

for r in out:
    print(json.dumps(r, sort_keys=True, separators=(",", ":")))
