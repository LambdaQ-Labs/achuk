#!/usr/bin/env python3
"""A2 arm: grammar-constrained generation via llama.cpp, scored like the gate.

For each task, POSTs the A1 prompt to a llama-server with the task's GBNF
grammar (bench/grammars/<id>.gbnf) so out-of-scope symbols and undeclared
effect names are ungeneratable at the token level. Scores valid-JSON /
hallucination-free / effects-sound with the same rules as
eval_gate_batched.py, and dumps outputs-a2.jsonl for real-compile grading.

    LLAMA_URL=http://127.0.0.1:8899 CLAW_TASKS=../bench/tasks-large \
    GRAMMARS=../bench/grammars python a2_eval.py
"""
import glob, json, os, re, urllib.request

URL = os.environ.get("LLAMA_URL", "http://127.0.0.1:8899")
TASKS_DIR = os.environ.get("CLAW_TASKS", "../bench/tasks-large")
GRAMMARS = os.environ.get("GRAMMARS", "../bench/grammars")
PROTO = open("train.py").read().split('PROTOCOL = """')[1].split('"""')[0]


def vars_of(x):
    o = []
    if isinstance(x, dict):
        for k, v in x.items():
            o += [v] if (k == "Var" and isinstance(v, str)) else vars_of(v)
    elif isinstance(x, list):
        for v in x:
            o += vars_of(v)
    return o


def _pat_vars(p):
    if isinstance(p, dict):
        if isinstance(p.get("Var"), str):
            return {p["Var"]}
        if "Tag" in p and isinstance(p["Tag"], list) and len(p["Tag"]) == 2:
            out = set()
            for sp in p["Tag"][1]:
                out |= _pat_vars(sp)
            return out
    return set()


def _free(e, bound):
    """Free Var names in an Expr JSON, respecting Lam/Let/Match binders."""
    if isinstance(e, list):
        out = []
        for x in e:
            out += _free(x, bound)
        return out
    if not isinstance(e, dict):
        return []
    if isinstance(e.get("Var"), str):
        return [] if e["Var"] in bound else [e["Var"]]
    if isinstance(e.get("Lam"), dict):
        l = e["Lam"]
        return _free(l.get("body"), bound | set(l.get("params") or []))
    if isinstance(e.get("Let"), dict):
        l = e["Let"]
        return _free(l.get("value"), bound) + _free(l.get("body"), bound | {l.get("name")})
    if isinstance(e.get("Match"), list) and len(e["Match"]) == 2:
        scrut, arms = e["Match"]
        out = _free(scrut, bound)
        for arm in arms:
            if isinstance(arm, list) and len(arm) == 2:
                out += _free(arm[1], bound | _pat_vars(arm[0]))
        return out
    out = []
    for v in e.values():
        out += _free(v, bound)
    return out


def expr_vars(defs):
    """Free names across the defs\' EXPRS (binders excluded; a def\'s own
    name and its siblings\' names are in scope — recursion is legal).
    Type::Var also serializes as {"Var": ...}, so "ty" is never walked."""
    defs = defs if isinstance(defs, list) else [defs]
    defined = {d.get("name") for d in defs if isinstance(d, dict) and d.get("name")}
    out = []
    for d in defs:
        if isinstance(d, dict):
            out += [v for v in _free(d.get("expr"), set(defined))]
    return out

def check(raw, scope):
    try:
        j = json.loads(raw.strip().strip('`').replace('json', '', 1).strip())
    except Exception:
        return (False, False, False)
    names = set(n for n, _, _ in scope)
    used = [v for v in expr_vars(j) if not re.match(r'^p\d+$', v)]
    hall = [v for v in used if v not in names]
    required = set()
    for n, _, eff in scope:
        if n in used:
            required.update(eff)
    declared = set()
    for d in (j if isinstance(j, list) else [j]):
        if isinstance(d, dict):
            declared.update(d.get("effects") or [])
    return (True, len(hall) == 0, required <= declared)


def gen(prompt, grammar):
    body = json.dumps({
        "messages": [{"role": "user", "content": prompt + "\n\n" + PROTO}],
        "grammar": grammar,
        "temperature": 0,
        "max_tokens": 220,
    }).encode()
    req = urllib.request.Request(
        URL + "/v1/chat/completions", data=body,
        headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=600) as r:
        return json.load(r)["choices"][0]["message"]["content"]


files = sorted(glob.glob(TASKS_DIR + "/*.json"))
n = len(files)
v = c = e = clean = 0
with open("outputs-a2.jsonl", "w") as fh:
    for i, f in enumerate(files, 1):
        t = json.load(open(f))
        scope = [(s["name"], s["ty"], s.get("effects", [])) for s in t.get("scope", [])]
        scopeln = "\n".join(
            f"  {nm} : {ty}" + (f"  [effects: {', '.join(ef)}]" if ef else "")
            for nm, ty, ef in scope)
        prompt = (f"Task: {t['prompt']}\n\nIn-scope symbols (the ONLY callable "
                  f"definitions):\n{scopeln}")
        gid = os.path.basename(f)[:-5]
        grammar = open(os.path.join(GRAMMARS, gid + ".gbnf")).read()
        raw = gen(prompt, grammar)
        vi, ci, ei = check(raw, scope)
        v += vi; c += ci; e += ei; clean += (ci and ei)
        try:
            defs = json.loads(raw.strip())
        except Exception:
            defs = None
        fh.write(json.dumps({"task": f, "defs": defs, "raw": raw[:400]}) + "\n")
        print(f"[{i}/{n}] {gid}: valid={bool(vi)} halluc_free={bool(ci)} "
              f"effects={bool(ei)}", flush=True)

print(f"A2: valid_json={v}/{n} ({100 * v // n}%)  no_halluc={c}/{n} ({100 * c // n}%)  "
      f"effects_sound={e}/{n} ({100 * e // n}%)  clean={clean}/{n} ({100 * clean // n}%)")
