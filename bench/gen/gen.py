#!/usr/bin/env python3
"""Synthetic spec corpus generator for the spec-dna benchmark.

Deterministic for a given --size and --seed. Writes Markdown docs in a
domain/capability tree, plus code files that docs link to, and an
expected.json with the counts a correct scanner must report.

    python3 gen.py --size small|medium|large --out DIR [--seed 1]
"""
import argparse
import json
import math
import os
import random

SIZES = {
    # docs, domains, capabilities per domain, code files
    "small": (60, 2, 3, 20),
    "medium": (1500, 8, 8, 400),
    "large": (20000, 24, 16, 4000),
}
KINDS = [
    ("spec", "SPEC", 0.35), ("adr", "ADR", 0.15), ("contract", "CTR", 0.10),
    ("guide", None, 0.25), ("task", "TASK", 0.15),
]
WORDS = (
    "the a system request response client server cache index token budget agent context file path link "
    "record decision status owner change review test check value field limit error retry timeout queue "
    "event handler session user account policy rule must should may not when if then while where each "
    "every one two three new old current previous next default local remote input output read write "
    "update delete create build deploy release version schema table column key map list set graph node "
    "edge parent child root branch merge commit diff patch scope domain module service endpoint route "
    "header body payload stream batch job worker thread lock state store load save parse render "
    "validate resolve report metric trace log alert gate step plan goal reason evidence proof"
).split()


def sentence(rng, n=None):
    n = n or rng.randint(6, 18)
    w = rng.choices(WORDS, k=n)
    return w[0].capitalize() + " " + " ".join(w[1:]) + "."


def paragraph(rng):
    return " ".join(sentence(rng) for _ in range(rng.randint(2, 6)))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--size", choices=SIZES, required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, default=1)
    a = ap.parse_args()
    rng = random.Random(a.seed)
    n_docs, n_dom, n_cap, n_code = SIZES[a.size]
    pool = [paragraph(rng) for _ in range(3000)]

    caps = [f"domain-{d:02d}/cap-{c:02d}" for d in range(n_dom) for c in range(n_cap)]
    code = [f"src/{rng.choice(caps)}/mod_{i:05d}.go" for i in range(n_code)]
    docs = []
    counters = {}
    for i in range(n_docs):
        kind, prefix, _ = rng.choices(KINDS, weights=[k[2] for k in KINDS])[0]
        cap = rng.choice(caps)
        counters[kind] = counters.get(kind, 0) + 1
        num = counters[kind]
        ident = f"{prefix}-{num}" if prefix else ""
        if kind == "adr":
            path = f"docs/decisions/ADR-{num}-{rng.choice(WORDS)}-{rng.choice(WORDS)}.md"
        else:
            path = f"docs/{cap}/{kind}-{num:05d}.md"
        docs.append({"path": path, "kind": kind, "id": ident, "fm": kind != "adr" and prefix is not None})
    ids = [d["id"] for d in docs if d["id"]]
    hubs = rng.sample(range(n_docs), max(1, n_docs // 100))

    exp = {
        "docs": n_docs, "links_internal": 0, "links_broken": 0, "ids": len(set(ids)),
        "idrefs": 0, "idrefs_unresolved": 0,
    }
    total_bytes = 0
    for f in code:
        p = os.path.join(a.out, f)
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, "w") as fh:
            fh.write("package main\n")
    for i, d in enumerate(docs):
        target_kb = min(300.0, math.exp(rng.gauss(math.log(5.0), 0.9)))
        lines = []
        if d["fm"]:
            lines += ["---", f"id: {d['id']}", f"kind: {d['kind']}", f"status: {rng.choice(['draft', 'active', 'superseded'])}", "---", ""]
        lines += [f"# {d['kind']} {i}", ""]
        size = 0
        here = os.path.dirname(d["path"])
        while size < target_kb * 1024:
            para = rng.choice(pool)
            r = rng.random()
            if r < 0.35:
                # internal link: mostly to a hub or a nearby doc, sometimes code, rarely broken
                j = rng.choice(hubs) if rng.random() < 0.3 else rng.randrange(n_docs)
                tgt = docs[j]["path"]
                if rng.random() < 0.15:
                    tgt = rng.choice(code)
                broken = rng.random() < 0.005
                if broken:
                    tgt = tgt.replace(".md", "-gone.md").replace(".go", "-gone.go")
                rel = os.path.relpath(tgt, here)
                anchor = "#overview" if rng.random() < 0.1 else ""
                para += f" See [{rng.choice(WORDS)}]({rel}{anchor})."
                exp["links_internal"] += 1
                exp["links_broken"] += broken
            elif r < 0.45:
                para += f" Read [{rng.choice(WORDS)}](https://example.org/{rng.choice(WORDS)})."
            elif r < 0.55 and ids:
                ref = rng.choice(ids)
                unresolved = rng.random() < 0.01
                if unresolved:
                    ref = ref.split("-")[0] + "-" + str(900000 + rng.randrange(1000))
                para += f" It follows {ref}."
                exp["idrefs"] += 1
                exp["idrefs_unresolved"] += unresolved
            elif r < 0.60:
                # decoys a scanner must ignore: links and ids in code
                para += f" Use `[x](missing-{i}.md)` and `ADR-999999` as written."
                lines += [para, "", "```md", f"[decoy](decoy-{i}.md) SPEC-999999", "```", ""]
                size += len(para) + 40
                continue
            lines += [para, ""]
            size += len(para) + 1
        text = "\n".join(lines) + "\n"
        p = os.path.join(a.out, d["path"])
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, "w") as fh:
            fh.write(text)
        total_bytes += len(text.encode())
    exp["bytes"] = total_bytes
    exp["files_walked"] = n_docs + n_code + 1  # + expected.json
    with open(os.path.join(a.out, "expected.json"), "w") as fh:
        json.dump(exp, fh, indent=1, sort_keys=True)
    print(json.dumps(exp, sort_keys=True))


if __name__ == "__main__":
    main()
