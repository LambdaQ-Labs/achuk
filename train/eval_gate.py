#!/usr/bin/env python3
"""P4 gate eval: base-vs-tuned hallucination-free rate on the benchmark.

Loads the base model + the LoRA adapter as ONE model and toggles the
adapter (`disable_adapter()`) so the only variable is the fine-tune —
avoiding the PEFT gotcha where wrapping mutates `base` in place. For each
task, both generate under the A1 prompt (task + in-scope symbols +
protocol); we score valid-JSON and hallucination-free (uses only p0..
params and real in-scope symbols).

    python eval_gate.py            # expects ./claw-lora and ../bench/tasks-large
"""
import json, glob, torch, re
from transformers import AutoModelForCausalLM, AutoTokenizer
from peft import PeftModel

BASE = "Qwen/Qwen2.5-Coder-0.5B-Instruct"
PROTO = open("train.py").read().split('PROTOCOL = """')[1].split('"""')[0]

tok = AutoTokenizer.from_pretrained(BASE)
m = AutoModelForCausalLM.from_pretrained(BASE, torch_dtype=torch.bfloat16, device_map="auto")
m = PeftModel.from_pretrained(m, "claw-lora")  # one model; toggle the adapter


def gen(prompt):
    msgs = [{"role": "user", "content": prompt + "\n\n" + PROTO}]
    t = tok.apply_chat_template(msgs, tokenize=False, add_generation_prompt=True)
    ids = tok(t, return_tensors="pt").to(m.device)
    out = m.generate(**ids, max_new_tokens=180, do_sample=False, pad_token_id=tok.eos_token_id)
    return tok.decode(out[0][ids.input_ids.shape[1]:], skip_special_tokens=True)


def vars_of(x):
    o = []
    if isinstance(x, dict):
        for k, v in x.items():
            o += [v] if (k == "Var" and isinstance(v, str)) else vars_of(v)
    elif isinstance(x, list):
        for v in x:
            o += vars_of(v)
    return o


def expr_vars(defs):
    """Var names from each def's EXPR only — Type::Var serializes as
    {"Var": "a"} too, so walking "ty" misreads generics as references."""
    out = []
    for d in (defs if isinstance(defs, list) else [defs]):
        if isinstance(d, dict):
            out += vars_of(d.get("expr"))
    return out


def check(raw, scope):
    try:
        j = json.loads(raw.strip().strip('`').replace('json', '', 1).strip())
    except Exception:
        return (False, False)
    names = set(n for n, _ in scope)
    hall = [v for v in expr_vars(j) if not re.match(r'^p\d+$', v) and v not in names]
    return (True, len(hall) == 0)


tasks = [json.load(open(f)) for f in sorted(glob.glob("../bench/tasks-large/*.json"))]
res = {"base": [0, 0], "tuned": [0, 0]}
for t in tasks:
    scope = [(s["name"], s["ty"]) for s in t.get("scope", [])]
    scopeln = "\n".join(f"  {n} : {s}" for n, s in scope)
    prompt = f"Task: {t['prompt']}\n\nIn-scope symbols (the ONLY callable definitions):\n{scopeln}"
    with m.disable_adapter():
        v, c = check(gen(prompt), scope)
        res["base"][0] += v; res["base"][1] += c
    v, c = check(gen(prompt), scope)
    res["tuned"][0] += v; res["tuned"][1] += c

n = len(tasks)
for k in ("base", "tuned"):
    v, c = res[k]
    print(f"{k}: valid_json={v}/{n} ({100 * v // n}%)  clean_no_halluc={c}/{n} ({100 * c // n}%)")
