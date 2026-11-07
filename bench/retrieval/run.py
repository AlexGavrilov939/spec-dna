#!/usr/bin/env python3
"""Retrieval correctness of `spec-dna context`.

For each task and each budget, runs `spec-dna context <entry> --budget B`
and compares the served sections with the task's required sections:

    recall (full)       required sections served in full / required sections
    recall (listed)     required sections served in full or listed as a stub
    precision           required sections served in full / sections served in full
    critical            sections stating a critical constraint served in full
    size                characters served (the `used` count) and an estimated
                        token count: ceil(UTF-8 bytes of the text output / 4)
    two-step            the context output plus `show` of each required section
                        it listed as a stub: characters, and recall (= listed)
    baseline            characters of every document that holds a required
                        section, read whole (an agent that knows which files to
                        open and reads them fully)
    latency             median wall time of the text command, ms

Tasks come in two formats: generator tasks (`required` holds section ids)
and hand-made oracles (`required` holds doc, heading and line; the line is
mapped to a section id through `spec-dna show --json`).

    python3 run.py --repo DIR --tasks FILE [--budgets 4000,8000,12000,16000]
        [--runs 5] --out FILE.json
"""
import argparse
import json
import os
import statistics
import subprocess
import time

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
BIN = os.path.join(ROOT, "target", "release", "spec-dna")


def run(repo, *args):
    p = subprocess.run([BIN, *args], cwd=repo, capture_output=True, text=True)
    if p.returncode != 0:
        raise RuntimeError(f"{args}: {p.stderr}")
    return p.stdout


def section_ids(repo, task):
    """Required and critical section ids of a task."""
    req = task["required"]
    if req and isinstance(req[0], str):
        return set(req), set()
    cache = {}

    def sid(doc, line):
        if doc not in cache:
            cache[doc] = json.loads(run(repo, "show", doc, "--json"))["units"]
        units = [u for u in cache[doc] if u["line"] <= line]
        if not units:
            raise RuntimeError(f"{doc}: no section at line {line}")
        return units[-1]["id"]

    def by_heading(doc, heading):
        if doc not in cache:
            cache[doc] = json.loads(run(repo, "show", doc, "--json"))["units"]
        for u in cache[doc]:
            if u["text"].split("\n", 1)[0].lstrip("#").strip() == heading:
                return u["id"]
        raise RuntimeError(f"{doc}: no heading {heading!r}")

    required = {sid(r["doc"], r["line"]) for r in req}
    critical = {by_heading(c["doc"], c["heading"]) for c in task.get("critical", [])}
    return required, critical


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--repo", required=True)
    ap.add_argument("--tasks", required=True)
    ap.add_argument("--budgets", default="4000,8000,12000,16000")
    ap.add_argument("--runs", type=int, default=5)
    ap.add_argument("--out", required=True)
    a = ap.parse_args()
    tasks = json.load(open(a.tasks))["tasks"]
    budgets = [int(b) for b in a.budgets.split(",")]
    rows = []
    doc_chars = {}

    def whole(doc):
        if doc not in doc_chars:
            units = json.loads(run(a.repo, "show", doc, "--json"))["units"]
            doc_chars[doc] = sum(len(u["text"]) for u in units)
        return doc_chars[doc]

    for t in tasks:
        required, critical = section_ids(a.repo, t)
        baseline = sum(whole(d) for d in {r.split("#")[0] for r in required})
        for b in budgets:
            d = json.loads(run(a.repo, "context", *t["entry"], "--budget", str(b), "--json"))
            full = {u["id"] for u in d["units"] if u["mode"] == "full"}
            listed = {u["id"] for u in d["units"]}
            stub_chars = sum(u["chars"] for u in d["units"] if u["mode"] == "stub" and u["id"] in required)
            walls = []
            text = ""
            for _ in range(a.runs):
                t0 = time.perf_counter()
                text = run(a.repo, "context", *t["entry"], "--budget", str(b))
                walls.append((time.perf_counter() - t0) * 1000)
            rows.append(
                {
                    "task": t["id"],
                    "budget": b,
                    "required": len(required),
                    "recall_full": len(required & full) / len(required) if required else 1.0,
                    "recall_listed": len(required & listed) / len(required) if required else 1.0,
                    "precision": len(required & full) / len(full) if full else 0.0,
                    "critical_full": (len(critical & full) / len(critical)) if critical else None,
                    "missed": sorted(required - full),
                    "chars": d["used"],
                    "two_step_chars": d["used"] + stub_chars,
                    "baseline_chars": baseline,
                    "tokens_est": (len(text.encode()) + 3) // 4,
                    "latency_ms": round(statistics.median(walls), 1),
                }
            )
    summary = []
    for b in budgets:
        rs = [r for r in rows if r["budget"] == b]
        crit = [r["critical_full"] for r in rs if r["critical_full"] is not None]
        summary.append(
            {
                "budget": b,
                "tasks": len(rs),
                "recall_full_mean": round(statistics.mean(r["recall_full"] for r in rs), 3),
                "recall_listed_mean": round(statistics.mean(r["recall_listed"] for r in rs), 3),
                "precision_mean": round(statistics.mean(r["precision"] for r in rs), 3),
                "critical_full_mean": round(statistics.mean(crit), 3) if crit else None,
                "tasks_all_required_full": sum(r["recall_full"] == 1.0 for r in rs),
                "chars_median": statistics.median(r["chars"] for r in rs),
                "two_step_chars_median": statistics.median(r["two_step_chars"] for r in rs),
                "baseline_chars_median": statistics.median(r["baseline_chars"] for r in rs),
                "tokens_est_median": statistics.median(r["tokens_est"] for r in rs),
                "latency_ms_median": statistics.median(r["latency_ms"] for r in rs),
            }
        )
    out = {"tasks_file": os.path.basename(a.tasks), "load_avg_1m": round(os.getloadavg()[0], 2), "summary": summary, "rows": rows}
    with open(a.out, "w") as fh:
        json.dump(out, fh, indent=2)
        fh.write("\n")
    for s in summary:
        print(json.dumps(s))


if __name__ == "__main__":
    main()
