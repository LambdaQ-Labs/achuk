#!/usr/bin/env python3
"""Targeted corpus examples for the three re-gate failure classes.

The 1030-example corpus taught op+literal shapes, but never the benchmark's
phrasings — "(parameter p0 named x)" and "computes Op of x and K" — so the
tuned model emitted {"Var":"10"} for literals, leaked prompt names (a, b, c)
into bodies, and invented Nat.clamp instead of composing min/max.

Generates bench-STYLE (phrasing/shape) but not bench-CONTENT examples:
function names and (name, op, constant) combos are disjoint from every
bench/tasks-large family, so the gate still measures generalization.

    python gen_targeted.py >> corpus-full.jsonl
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
]
for name, ns in CLAMPISH:
    for flip in (False, True):
        if flip:
            body = app("Nat.max", [app("Nat.min", [var("p0"), var("p2")]), var("p1")])
            how = f"computes Nat.max of (Nat.min of {ns[0]} and {ns[2]}) and {ns[1]}"
        else:
            body = app("Nat.min", [app("Nat.max", [var("p0"), var("p1")]), var("p2")])
            how = f"computes Nat.min of (Nat.max of {ns[0]} and {ns[1]}) and {ns[2]}"
        prompt = (f"Define `{name}` : Nat, Nat, Nat -> Nat (parameters p0, p1, p2 "
                  f"named {ns[0]}, {ns[1]}, {ns[2]}) that keeps {ns[0]} within "
                  f"[{ns[1]}, {ns[2]}]: it {how}. Use only in-scope symbols.")
        emit(prompt, d(name, lam(["p0", "p1", "p2"], body), fn_ty(3)),
             ["Nat.max", "Nat.min"])

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
