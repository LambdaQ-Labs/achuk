#!/usr/bin/env python3
"""Grade the parity completions: functional Pass@1, five languages.

For every task: enumerate the same test cases the Achuk grader uses (an
integer grid 0..=3 per parameter, preconditions filtering), run the
generated function on each case, and check every contract with the actual
result. Achuk defs are graded by `achuk defs-grade` (compile + executed
contracts); Python/JS run under their interpreters; Go/Rust are compiled
per task.

    python parity_grade.py     # in train/, after pulling parity-*.jsonl
"""
import itertools, json, os, re, subprocess, sys, tempfile

ACHUK = os.environ.get("ACHUK_BIN", "../target/debug/achuk")
BOUND = 3
TIMEOUT = 20


def cases(task):
    names = [p["name"] for p in task["params"]]
    out = []
    for combo in itertools.product(range(BOUND + 1), repeat=len(names)):
        env = dict(zip(names, combo))
        try:
            if all(eval(r, {}, dict(env)) for r in task["grade"]["requires"]):
                out.append(env)
        except Exception:
            return []
    return out


def contracts_hold(task, env, result):
    scope = dict(env)
    scope["result"] = result
    try:
        return all(eval(c, {}, dict(scope)) for c in task["grade"]["contracts"])
    except Exception:
        return False


def strip_fences(raw):
    m = re.search(r"```[a-z]*\n(.*?)```", raw, re.S)
    if m:
        return m.group(1).strip()
    # No closing fence (often truncation): take everything after an opening
    # fence if present, else the raw text.
    m = re.search(r"```[a-z]*\n(.*)$", raw, re.S)
    return (m.group(1) if m else raw).strip()


def balance_truncate(code):
    """Cut to the last position where all braces are closed — drops the
    half-emitted trailing item a max_tokens cut leaves behind. Applied to
    every brace language equally; wrong-but-complete code is untouched."""
    depth = 0
    last = 0
    for i, ch in enumerate(code):
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                last = i + 1
    return code[:last] if last else code


def py_trim(code):
    """Drop trailing lines until the module parses (truncation repair)."""
    import ast
    lines = code.splitlines()
    while lines:
        try:
            ast.parse("\n".join(lines))
            return "\n".join(lines)
        except SyntaxError:
            lines.pop()
    return code


def go_normalize(code):
    """Remove package decls (the harness supplies package main) and merge
    the model's imports with fmt. Unused imports remain the model's error
    only if IT chose them; single-import lines are pruned when unused —
    matching what gofmt-era tooling does automatically for developers."""
    lines = [l for l in code.splitlines() if not l.strip().startswith("package ")]
    imports = set()
    body = []
    in_block = False
    for l in lines:
        s = l.strip()
        if in_block:
            if s == ")":
                in_block = False
            elif s:
                imports.add(s.strip('"'))
            continue
        if s.startswith("import ("):
            in_block = True
            continue
        if s.startswith("import "):
            imports.add(s.split()[-1].strip('"'))
            continue
        body.append(l)
    src = "\n".join(body)
    imports.add("fmt")
    used = {imp for imp in imports if imp == "fmt" or re.search(r"\b" + re.escape(imp.split("/")[-1]) + r"\.", src)}
    header = "\n".join(f"import \"{i}\"" for i in sorted(used))
    return header + "\n\n" + src


def run(cmd, cwd=None):
    return subprocess.run(cmd, capture_output=True, text=True, timeout=TIMEOUT, cwd=cwd)


def grade_lang(lang, line):
    task = json.load(open(line["task"]))
    cs = cases(task)
    if not cs:
        return None  # ungradeable (no cases survive preconditions)
    code = strip_fences(line["raw"])
    if lang == "py":
        code = py_trim(code)
    elif lang in ("js", "go", "rs"):
        code = balance_truncate(code)
    if lang == "go":
        code = go_normalize(code)
    name = line["name"]
    names = [p["name"] for p in task["params"]]
    args_rows = [[c[n] for n in names] for c in cs]

    with tempfile.TemporaryDirectory() as td:
        try:
            if lang == "py":
                harness = (code + "\n\nimport json\nfor row in json.loads('" +
                           json.dumps(args_rows) + "'):\n    print(" + name + "(*row))\n")
                p = run([sys.executable, write(td, "s.py", harness)])
            elif lang == "js":
                harness = (code + "\nconst rows = " + json.dumps(args_rows) +
                           ";\nfor (const r of rows) console.log(" + name + "(...r));\n")
                p = run(["node", write(td, "s.js", harness)])
            elif lang == "go":
                calls = "\n".join(
                    f"\tfmt.Println({name}({', '.join(f'int64({v})' for v in row)}))"
                    for row in args_rows)
                harness = f"package main\n\n{code}\n\nfunc main() {{\n{calls}\n}}\n"
                p = run(["go", "run", write(td, "s.go", harness)])
            elif lang == "rs":
                calls = "\n".join(
                    f"    println!(\"{{}}\", {name}({', '.join(f'{v}i64' for v in row)}));"
                    for row in args_rows)
                harness = f"{code}\n\nfn main() {{\n{calls}\n}}\n"
                src = write(td, "s.rs", harness)
                c = run(["rustc", "-O", "-o", os.path.join(td, "s"), src])
                if c.returncode != 0:
                    return False
                p = run([os.path.join(td, "s")])
            else:
                raise ValueError(lang)
        except subprocess.TimeoutExpired:
            return False
    if p.returncode != 0:
        return False
    lines = [l.strip() for l in p.stdout.strip().splitlines() if l.strip()]
    if len(lines) != len(cs):
        return False
    for env, out in zip(cs, lines):
        try:
            val = int(out) if out not in ("true", "false", "True", "False") else out.lower() == "true"
        except ValueError:
            return False
        if not contracts_hold(task, env, val):
            return False
    return True


def write(td, fname, content):
    p = os.path.join(td, fname)
    with open(p, "w") as f:
        f.write(content)
    return p


def grade_achuk(line):
    if line["defs"] is None:
        return False
    task = json.load(open(line["task"]))
    if not cases(task):
        return None
    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as f:
        json.dump(line["defs"], f)
        defs_path = f.name
    try:
        p = run([ACHUK, "defs-grade", defs_path, line["task"]])
        if p.returncode != 0:
            return False
        r = json.loads(p.stdout)
        held, total = r["contracts_held"]
        return bool(r["compiled"]) and total > 0 and held == total
    finally:
        os.unlink(defs_path)


results = {}
for arm, path, fn in [
    ("achuk-tuned", "parity-achuk.jsonl", grade_achuk),
    ("python", "parity-py.jsonl", lambda l: grade_lang("py", l)),
    ("javascript", "parity-js.jsonl", lambda l: grade_lang("js", l)),
    ("go", "parity-go.jsonl", lambda l: grade_lang("go", l)),
    ("rust", "parity-rs.jsonl", lambda l: grade_lang("rs", l)),
]:
    if not os.path.exists(path):
        print(f"{arm}: {path} missing, skipped")
        continue
    passed = failed = skipped = 0
    fails = []
    for raw in open(path):
        line = json.loads(raw)
        v = fn(line)
        if v is None:
            skipped += 1
        elif v:
            passed += 1
        else:
            failed += 1
            fails.append(os.path.basename(line["task"]))
    n = passed + failed
    results[arm] = (passed, n)
    pct = 100 * passed // n if n else 0
    print(f"{arm:12} pass@1 = {passed}/{n} ({pct}%)  [skipped {skipped}]")
    if fails[:6]:
        print(f"{'':12} fails: {', '.join(fails[:6])}{' …' if len(fails) > 6 else ''}")

print()
if "achuk-tuned" in results and "python" in results:
    c, cn = results["achuk-tuned"]
    p, pn = results["python"]
    verdict = "PASSES" if cn and pn and (c / cn) >= (p / pn) else "does not pass"
    print(f"P4 gate (tuned-Achuk >= stock-Python): {verdict}")
