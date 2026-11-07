#!/usr/bin/env python3
"""Synthetic spec corpus generator for the spec-dna benchmark.

Deterministic for a given --size and --seed. Writes Markdown docs in a
domain/capability tree, plus code files that docs link to, and an
expected.json with the counts a correct scanner must report.

    python3 gen.py --size small|medium|large --out DIR [--seed 1]

With --model spec-dna it writes a corpus in the information model of ADR
0001 instead (specs with requirements, cross-cutting guides, decisions,
changes and notes), a spec-dna.toml, and tasks.json: retrieval tasks with
the sections a correct context assembly must serve (ground truth).

    python3 gen.py --model spec-dna --size small|medium|large --out DIR [--tasks 40]
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
    ap.add_argument("--model", choices=["scanner", "spec-dna"], default="scanner")
    ap.add_argument("--tasks", type=int, default=40)
    a = ap.parse_args()
    if a.model == "spec-dna":
        return spec_dna_corpus(a)
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


# --- spec-dna model -------------------------------------------------------

SPEC_DNA_SIZES = {
    # specs, domains, guides, decisions, changes, notes
    "small": (40, 4, 4, 12, 6, 10),
    "medium": (600, 12, 24, 180, 90, 300),
    "large": (6000, 40, 120, 1800, 900, 3000),
}
TRIGGERS = [
    "When a request arrives", "While the cache is warm", "When a token expires", "If the upstream fails",
    "While a session is active", "When a client retries", "If the queue is full", "When a job starts",
]
ACTIONS = [
    "reject the request with status 429", "write one audit record", "return the cached value",
    "retry at most 3 times", "close the connection within 5 seconds", "emit a metric",
    "keep the previous version", "sign the response",
]


def requirement(rng, key, cap):
    trig = rng.choice(TRIGGERS)
    then = " then" if trig.startswith("If") else ""
    return f"- **{key}**: {trig},{then} the {cap} SHALL {rng.choice(ACTIONS)}."


def spec_dna_corpus(a):
    """Writes a corpus in the ADR 0001 model and tasks with ground truth.

    Ground truth of a task with entry spec E (ADR 0001 context order):
    - every section of E that holds requirements (E#anchor);
    - every section of each active guide whose applies-to matches E;
    - the requirement sections of each depends-on target of E, or the one
        section that holds a referenced key;
    - the Decision Outcome section of each accepted decision linked with E.
    Not required: E's overview, rationale and examples, notes, changes,
    rejected or superseded decisions, unrelated specs.
    """
    rng = random.Random(a.seed)
    n_spec, n_dom, n_guide, n_dec, n_chg, n_note = SPEC_DNA_SIZES[a.size]
    pool = [paragraph(rng) for _ in range(2000)]
    doms = [f"d{d:02d}" for d in range(n_dom)]
    specs = []
    for i in range(n_spec):
        dom = rng.choice(doms)
        specs.append({"id": f"{dom}/cap-{i:05d}", "dom": dom, "keys": {}, "deps": [], "status": "active"})
    for sp in specs:
        r = rng.random()
        sp["status"] = "deprecated" if r < 0.05 else "active"
        sp["sections"] = []
        for sec in ("requirements", "errors") if rng.random() < 0.4 else ("requirements",):
            sp["keys"][sec] = [f"{sec[:3]}-{j}" for j in range(rng.randint(2, 7))]
            sp["sections"].append(sec)
    by_id = {s["id"]: s for s in specs}
    for sp in specs:
        for _ in range(rng.choice([0, 0, 1, 1, 2])):
            t = rng.choice(specs)
            if t is sp or t["status"] != "active":
                continue
            if rng.random() < 0.3:
                sec = rng.choice(t["sections"])
                sp["deps"].append((t["id"], sec, rng.choice(t["keys"][sec])))
            else:
                sp["deps"].append((t["id"], None, None))
    guides = []
    for i in range(n_guide):
        scope = rng.choice(doms)
        guides.append({
            "id": f"guides/rule-{i:04d}",
            "applies": scope if rng.random() < 0.7 else f"{scope}/*",
            "status": "active" if rng.random() < 0.85 else "retired",
            "dom": scope,
        })
    decisions = []
    for i in range(n_dec):
        links = rng.sample(specs, k=rng.randint(1, 2))
        st = rng.choices(["accepted", "proposed", "rejected", "superseded"], weights=[70, 10, 10, 10])[0]
        decisions.append({"id": f"decisions/dec-{i:05d}", "links": [s["id"] for s in links], "status": st})
    # A superseded decision needs an accepted successor.
    for i, d in enumerate(decisions):
        if d["status"] == "superseded":
            succ = rng.choice([x for x in decisions if x["status"] == "accepted"])
            succ.setdefault("supersedes", []).append(d["id"])

    os.makedirs(a.out, exist_ok=True)

    def write(rel, text):
        p = os.path.join(a.out, "docs", rel + ".md")
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, "w") as fh:
            fh.write(text)

    def prose(n):
        return "\n\n".join(rng.choice(pool) for _ in range(n))

    for sp in specs:
        fm = ["---", f"status: {sp['status']}"]
        if sp["deps"]:
            fm.append("depends-on:")
            fm += [f"  - {d}#{k}" if k else f"  - {d}" for d, _, k in sp["deps"]]
        fm += ["---", ""]
        cap = sp["id"].split("/")[1]
        body = [f"# Capability {cap}", "", prose(1), "", "## Overview", "", prose(rng.randint(2, 6)), ""]
        for sec in sp["sections"]:
            body += [f"## {sec.capitalize()}", ""]
            body += [requirement(rng, k, cap) for k in sp["keys"][sec]]
            body.append("")
        body += ["## Rationale", "", prose(rng.randint(3, 12)), "", "## Examples", "", "```text", rng.choice(pool), "```", ""]
        write(sp["id"], "\n".join(fm + body))
    for g in guides:
        fm = ["---", "type: guide", f"status: {g['status']}", f"applies-to: [\"{g['applies']}\"]", "---", ""]
        body = [f"# Rule {g['id'].split('-')[1]}", "", prose(1), "", "## Procedure", "", prose(rng.randint(1, 3)), ""]
        write(g["id"], "\n".join(fm + body))
    for d in decisions:
        fm = ["---", "type: decision", f"status: {d['status']}"]
        if d.get("supersedes"):
            fm.append("supersedes: [" + ", ".join(d["supersedes"]) + "]")
        fm += ["---", ""]
        refs = " and ".join(f"[[{x}]]" for x in d["links"])
        body = [f"# Decision {d['id'].split('-')[1]}", "", "## Context and Problem Statement", "", f"This concerns {refs}. " + prose(rng.randint(1, 4)), "",
                "## Considered Options", "", "- Option one", "- Option two", "", "## Decision Outcome", "", "Chosen option: option one. " + rng.choice(pool), ""]
        write(d["id"], "\n".join(fm + body))
    for i in range(n_chg):
        t = rng.choice(specs)["id"]
        write(f"changes/chg-{i:05d}", "\n".join(["---", "type: change", "status: proposed", "---", "", f"# Change {i}", "",
                                                    f"Changes [[{t}]]. " + prose(rng.randint(1, 3)), ""]))
    for i in range(n_note):
        t = rng.choice(specs)["id"]
        write(f"notes/note-{i:05d}", "\n".join(["---", "type: note", "---", "", f"# Note {i}", "", f"About [[{t}]]. " + prose(rng.randint(2, 8)), ""]))

    with open(os.path.join(a.out, "spec-dna.toml"), "w") as fh:
        fh.write('spec-dna = "0.1.0"\n\n[[roots]]\npath = "docs"\ntype = "mixed"\ndefault-type = "spec"\nstatus = "active"\n')

    # Tasks with ground truth.
    linked = {}
    for d in decisions:
        if d["status"] == "accepted":
            for x in d["links"]:
                linked.setdefault(x, []).append(d["id"])
    tasks = []
    candidates = [s for s in specs if s["status"] == "active"]
    for n, sp in enumerate(rng.sample(candidates, k=min(a.tasks, len(candidates)))):
        req = [f"{sp['id']}#{sec}" for sec in sp["sections"]]
        for g in guides:
            if g["status"] == "active" and g["dom"] == sp["dom"]:
                req.append(f"{g['id']}#procedure")
                req.append(f"{g['id']}#rule-{g['id'].split('-')[1]}")
        for d, sec, key in sp["deps"]:
            if sec:
                req.append(f"{d}#{sec}")
            else:
                req += [f"{d}#{s2}" for s2 in by_id[d]["sections"]]
        for d in linked.get(sp["id"], []):
            req.append(f"{d}#decision-outcome")
        tasks.append({"id": f"s{n:03d}", "entry": [sp["id"]], "required": sorted(set(req))})
    with open(os.path.join(a.out, "tasks.json"), "w") as fh:
        json.dump({"size": a.size, "seed": a.seed, "tasks": tasks}, fh, indent=1)
    print(json.dumps({"specs": n_spec, "guides": n_guide, "decisions": n_dec, "changes": n_chg, "notes": n_note, "tasks": len(tasks)}))


if __name__ == "__main__":
    main()
