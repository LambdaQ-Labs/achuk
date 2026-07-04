#!/usr/bin/env python3
"""Dump the tuned model's gate failures: task id, hallucinated names, raw output.

Batched tuned-arm-only rerun of the gate; prints a JSON line per failing task
so the corpus can be grown to cover exactly the shapes that still miss.

    python eval_failures.py            # expects ./claw-lora and ../bench/tasks-large
"""
import json, glob, torch, re, os
from transformers import AutoModelForCausalLM, AutoTokenizer
from peft import PeftModel

BASE = "Qwen/Qwen2.5-Coder-0.5B-Instruct"
BS = 32
PROTO = open("train.py").read().split('PROTOCOL = """')[1].split('"""')[0]

tok = AutoTokenizer.from_pretrained(BASE, padding_side="left")
if tok.pad_token is None:
    tok.pad_token = tok.eos_token
m = AutoModelForCausalLM.from_pretrained(BASE, torch_dtype=torch.bfloat16, device_map="auto")
m = PeftModel.from_pretrained(m, "claw-lora")


def gen_batch(prompts):
    outs = []
    for i in range(0, len(prompts), BS):
        chunk = prompts[i:i + BS]
        texts = [
            tok.apply_chat_template(
                [{"role": "user", "content": p + "\n\n" + PROTO}],
                tokenize=False, add_generation_prompt=True)
            for p in chunk
        ]
        enc = tok(texts, return_tensors="pt", padding=True).to(m.device)
        out = m.generate(**enc, max_new_tokens=180, do_sample=False,
                         pad_token_id=tok.pad_token_id)
        outs += [tok.decode(out[j][enc.input_ids.shape[1]:], skip_special_tokens=True)
                 for j in range(len(chunk))]
        print(f"gen: {len(outs)}/{len(prompts)}", flush=True)
    return outs


def vars_of(x):
    o = []
    if isinstance(x, dict):
        for k, v in x.items():
            o += [v] if (k == "Var" and isinstance(v, str)) else vars_of(v)
    elif isinstance(x, list):
        for v in x:
            o += vars_of(v)
    return o


files = sorted(glob.glob("../bench/tasks-large/*.json"))
tasks = [json.load(open(f)) for f in files]
scopes, prompts = [], []
for t in tasks:
    scope = [(s["name"], s["ty"]) for s in t.get("scope", [])]
    scopeln = "\n".join(f"  {n} : {s}" for n, s in scope)
    scopes.append(scope)
    prompts.append(f"Task: {t['prompt']}\n\nIn-scope symbols (the ONLY callable definitions):\n{scopeln}")

outs = gen_batch(prompts)
fails = 0
for f, t, scope, raw in zip(files, tasks, scopes, outs):
    names = set(n for n, _ in scope)
    try:
        j = json.loads(raw.strip().strip('`').replace('json', '', 1).strip())
        hall = sorted(set(v for v in vars_of(j) if not re.match(r'^p\d+$', v) and v not in names))
        if not hall:
            continue
        reason = {"hallucinated": hall}
    except Exception as e:
        reason = {"invalid_json": str(e)}
    fails += 1
    print("FAIL " + json.dumps({
        "task": os.path.basename(f), "prompt": t["prompt"],
        "scope": sorted(names), **reason, "raw": raw[:400],
    }), flush=True)
print(f"total fails: {fails}/{len(tasks)}", flush=True)
