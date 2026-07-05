#!/usr/bin/env python3
"""Procedurally generate a large set of Achuk benchmark tasks.

Emits scalar-arithmetic tasks with EXECUTABLE contracts (so they can
functionally pass) plus wrapper/compose tasks, into a target dir. Used
both to grow the benchmark and as the prompt set for distillation.

    python bench/gen_tasks.py bench/tasks-large
"""
import json, os, sys, itertools

OUT = sys.argv[1] if len(sys.argv) > 1 else "bench/tasks-large"
os.makedirs(OUT, exist_ok=True)

NAT2 = {  # binary Nat->Nat symbols and a checkable postcondition template
    "Nat.add": ("adds", ["result == a + b", "result >= a"]),
    "Nat.mul": ("multiplies", ["result == a * b"]),
    "Nat.max": ("takes the maximum of", ["result >= a", "result >= b"]),
    "Nat.min": ("takes the minimum of", ["result <= a", "result <= b"]),
}
SCOPE2 = [{"name": n, "ty": "Nat, Nat -> Nat"} for n in NAT2]

def task(tid, prompt, scope, params, requires, contracts):
    return {
        "id": tid, "category": "contract", "prompt": prompt,
        "scope": scope, "params": params,
        "grade": {"compile": True, "requires": requires,
                  "contracts": contracts, "forbidden": ["hallucinated-symbol"]},
    }

n = 0
def write(t):
    global n
    with open(os.path.join(OUT, t["id"] + ".json"), "w") as f:
        json.dump(t, f, indent=2)
    n += 1

# 1) binary arithmetic — each op, params p0,p1 named a,b
for sym, (verb, contracts) in NAT2.items():
    base = sym.replace(".", "_").lower()
    write(task(
        f"gen-{base}",
        f"Define `{base}` : Nat, Nat -> Nat (parameters p0, p1 named a, b) that {verb} its two arguments using in-scope `{sym}`.",
        SCOPE2, [{"name": "a"}, {"name": "b"}], [], contracts))

# 2) clamp family — value clamped into [lo, hi]
for lo_first in (True, False):
    tid = "gen-clamp-" + ("lohi" if lo_first else "hilo")
    write(task(
        tid,
        "Define `clamp` : Nat, Nat, Nat -> Nat (parameters p0, p1, p2 named x, lo, hi) that clamps x into [lo, hi]. Assume lo <= hi. Use only in-scope symbols.",
        SCOPE2, [{"name": "x"}, {"name": "lo"}, {"name": "hi"}],
        ["lo <= hi"], ["result >= lo", "result <= hi"]))

# 3) offset/scale: f(x) = op(x, k) for constant-ish via second param
for sym, (verb, _) in NAT2.items():
    base = sym.replace(".", "_").lower()
    write(task(
        f"gen-{base}-comm",
        f"Define `{base}_comm` : Nat, Nat -> Nat (parameters p0, p1 named a, b) that {verb} the arguments (order may matter) using `{sym}`.",
        SCOPE2, [{"name": "a"}, {"name": "b"}], [], []))

# 4) nested: max(add(a,b), a) style compositions with contracts
combos = [
    ("addmax", "Nat.add", "Nat.max", ["result >= a"]),
    ("mulmin", "Nat.mul", "Nat.min", []),
    ("addmin", "Nat.add", "Nat.min", ["result >= a"]),
]
for name, s1, s2, contracts in combos:
    write(task(
        f"gen-{name}",
        f"Define `{name}` : Nat, Nat -> Nat (parameters p0, p1 named a, b) combining `{s1}` and `{s2}`.",
        SCOPE2, [{"name": "a"}, {"name": "b"}], [], contracts))

# 5) three-arg arithmetic (sum of three, max of three)
SCOPE3 = SCOPE2
for name, verb, contracts in [
    ("sum3", "sums the three arguments", ["result >= a", "result >= b"]),
    ("max3", "returns the maximum of three arguments", ["result >= a", "result >= b", "result >= c"]),
]:
    write(task(
        f"gen-{name}",
        f"Define `{name}` : Nat, Nat, Nat -> Nat (parameters p0, p1, p2 named a, b, c) that {verb} using only in-scope symbols.",
        SCOPE3, [{"name": "a"}, {"name": "b"}, {"name": "c"}], [], contracts))

# 6) unary-with-constant family (the bulk): f(x) = op(x, k). Multiple ops,
#    constants, and prompt phrasings — the diverse volume for distillation.
UNARY = [
    ("add", "Nat.add", "+", ["result == x + {k}", "result >= x"]),
    ("mul", "Nat.mul", "*", ["result == x * {k}"]),
    ("atleast", "Nat.max", "max-with", ["result >= x", "result >= {k}"]),
    ("atmost", "Nat.min", "min-with", ["result <= x", "result <= {k}"]),
]
PHRASINGS = [
    "Define `{name}` : Nat -> Nat (parameter p0 named x) that computes {sym} of x and {k}.",
    "In Achuk, define `{name}` (parameter p0 = x) : Nat -> Nat applying `{sym}` to x and the constant {k}.",
    "Write `{name}` : Nat -> Nat using only in-scope `{sym}`; parameter p0 is x, combine it with {k}.",
]
for opname, sym, _desc, ctpl in UNARY:
    for k in (1, 2, 3, 4, 5, 7, 10, 100):
        for pi, phr in enumerate(PHRASINGS):
            tid = f"gen-{opname}{k}-v{pi}"
            contracts = [c.format(k=k) for c in ctpl]
            prompt = phr.format(name=f"{opname}{k}", sym=sym, k=k)
            write(task(tid, prompt, [{"name": sym, "ty": "Nat, Nat -> Nat"}],
                       [{"name": "x"}], [], contracts))

# 7) platform effects — the sys platform's hosted effects in scope. Graded
#    for hallucination-free references AND effect soundness (declared row
#    must cover the used symbols' rows — "effect-unsound" is forbidden).
#    Contracts don't execute (no scalar params) — reference + effects only.
PLAT = [
    {"name": "File.read!", "ty": "Str -> Str", "effects": ["Fs"]},
    {"name": "Env.get!", "ty": "Str -> Str", "effects": ["Env"]},
    {"name": "Stdout.line!", "ty": "Str -> Unit", "effects": ["Stdout"]},
    {"name": "Str.concat", "ty": "Str, Str -> Str"},
    {"name": "Str.upper", "ty": "Str -> Str"},
]

def plat_task(tid, prompt):
    return {
        "id": tid, "category": "effect", "prompt": prompt,
        "scope": PLAT, "params": [],
        "grade": {"compile": True, "requires": [], "contracts": [],
                  "forbidden": ["hallucinated-symbol", "effect-unsound"]},
    }

PLAT_TASKS = [
    ("gen-plat-readfile", "Define `read_file` : Str -> Str (parameter p0 named path) that forwards path to the in-scope `File.read!`. Declare its effects (Fs)."),
    ("gen-plat-getenv", "Define `get_env` : Str -> Str (parameter p0 named key) that forwards key to the in-scope `Env.get!`. Declare its effects (Env)."),
    ("gen-plat-println", "Define `println` : Str -> Unit (parameter p0 named msg) that forwards msg to the in-scope `Stdout.line!`. Declare its effects (Stdout)."),
    ("gen-plat-catfile", "Define `cat_file` : Str -> Unit (parameter p0 named path) that reads the file at path with `File.read!` and prints the contents with `Stdout.line!`. Declare its effects (Fs, Stdout)."),
    ("gen-plat-showenv", "Define `show_env` : Str -> Unit (parameter p0 named key) that reads the environment variable with `Env.get!` and prints it with `Stdout.line!`. Declare its effects (Env, Stdout)."),
    ("gen-plat-readupper", "Define `read_upper` : Str -> Str (parameter p0 named path) that reads the file at path with `File.read!` and uppercases the contents with `Str.upper`. Declare its effects (Fs)."),
    ("gen-plat-envupper", "Define `env_upper` : Str -> Str (parameter p0 named key) that reads the environment variable with `Env.get!` then applies `Str.upper`. Declare its effects (Env)."),
    ("gen-plat-shoutfile", "Define `shout_file` : Str -> Unit (parameter p0 named path) that reads the file at path with `File.read!`, uppercases it with `Str.upper`, and prints it with `Stdout.line!`. Declare its effects (Fs, Stdout)."),
    ("gen-plat-purecat", "Define `join2` : Str, Str -> Str (parameters p0, p1 named a, b) that concatenates a and b with the in-scope `Str.concat`. It performs no effects — declare an empty effect row."),
    ("gen-plat-readboth", "Define `read_both` : Str, Str -> Str (parameters p0, p1 named path1, path2) that reads both files with `File.read!` and concatenates the contents with `Str.concat`. Declare its effects (Fs)."),
]
for tid, prompt in PLAT_TASKS:
    write(plat_task(tid, prompt))

print(f"wrote {n} tasks to {OUT}")
