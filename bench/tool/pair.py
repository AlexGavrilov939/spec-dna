#!/usr/bin/env python3
"""Paired benchmark of two spec-dna binaries on one corpus.

The release rule of 0.7.0 (ADR 0023, design section 15): both binaries run in
one job, on one machine, in turns. The pairs cancel the drift of a shared
runner. The rule reads the median of the paired ratios head / base of the mode
`full`, and the limit is 1.10.

Pair i runs the base once and the head once. The order of the two runs flips
from pair to pair, so a warm cache or a slow start does not help one binary.
Each binary has its own copy of the corpus, and the copy is its own git
repository with one commit. The harness sets the version pin of each copy to the
version of its binary.

Modes (one process for each run, as in run.py):
    full   check --all --json --no-cache: parse every document, without the index
    warm   check --all --json with the index, when nothing changed

    python3 pair.py --corpus PATH --base-bin FILE --head-bin FILE
        [--pairs 40] [--modes full,warm] [--limit 1.10] [--work DIR] --out FILE.json
The exit code is 1 if the median ratio of `full` is over the limit.
It runs only on Unix (os.wait4).
"""
import argparse
import json
import os
import shutil
import statistics
import sys

from run import GIT, once, sh

COMMANDS = {
    "full": ["check", "--all", "--json", "--no-cache"],
    "warm": ["check", "--all", "--json"],
}


def version_of(binary):
    """The number of `spec-dna --version`, as `spec-dna 0.7.0`."""
    return sh(binary, "--version").split()[-1]


def prepare(corpus, binary, root):
    """Copies the corpus into `root/repo`, sets the pin and makes one commit."""
    repo = os.path.join(root, "repo")
    shutil.rmtree(root, ignore_errors=True)
    shutil.copytree(corpus, repo, ignore=shutil.ignore_patterns(".git"))
    cfg = os.path.join(repo, "spec-dna.toml")
    with open(cfg) as fh:
        lines = fh.read().split("\n")
    lines[0] = f'spec-dna = "{version_of(binary)}"'
    with open(cfg, "w") as fh:
        fh.write("\n".join(lines))
    sh(*GIT, "init", "-q", "-b", "main", cwd=repo)
    sh(*GIT, "add", "-A", "-f", cwd=repo)
    sh(*GIT, "commit", "-q", "-m", "snapshot", cwd=repo)
    return repo


def run_once(binary, repo, mode):
    wall, _, out = once([binary, *COMMANDS[mode]], repo)
    return wall * 1000, out


def summary(out):
    s = json.loads(out)["summary"]
    return {k: s[k] for k in ("documents", "errors", "warnings")}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--corpus", required=True)
    ap.add_argument("--base-bin", required=True)
    ap.add_argument("--head-bin", required=True)
    ap.add_argument("--pairs", type=int, default=40)
    ap.add_argument("--modes", default="full,warm")
    ap.add_argument("--limit", type=float, default=1.10)
    ap.add_argument("--work", required=True)
    ap.add_argument("--out", required=True)
    a = ap.parse_args()
    modes = a.modes.split(",")
    if "full" not in modes or any(m not in COMMANDS for m in modes):
        sys.exit("--modes must hold `full`, and only `full` and `warm`")

    bins = {"base": a.base_bin, "head": a.head_bin}
    repos = {k: prepare(a.corpus, b, os.path.join(a.work, k)) for k, b in bins.items()}
    res = {
        "base": version_of(a.base_bin),
        "head": version_of(a.head_bin),
        "pairs": a.pairs,
        "limit": a.limit,
        "load_avg_1m_before": round(os.getloadavg()[0], 2),
        "modes": {},
    }
    for mode in modes:
        # The warm-up of each binary also writes the index for `warm`. The
        # diagnostics of the two binaries must give the same counts.
        counts = {k: summary(run_once(bins[k], repos[k], mode)[1]) for k in bins}
        res["modes"][mode] = {"counts": counts, "pairs": []}
        base_ms, head_ms, ratios = [], [], []
        for i in range(a.pairs):
            order = ("base", "head") if i % 2 == 0 else ("head", "base")
            ms = {k: run_once(bins[k], repos[k], mode)[0] for k in order}
            base_ms.append(ms["base"])
            head_ms.append(ms["head"])
            ratios.append(ms["head"] / ms["base"])
        res["modes"][mode].update(
            {
                "base_median_ms": round(statistics.median(base_ms), 2),
                "head_median_ms": round(statistics.median(head_ms), 2),
                "ratio_median": round(statistics.median(ratios), 4),
                "ratio_mean": round(statistics.mean(ratios), 4),
                "ratio_min": round(min(ratios), 4),
                "ratio_max": round(max(ratios), 4),
                "ratios_over_limit": sum(r > a.limit for r in ratios),
            }
        )
        res["modes"][mode]["pairs"] = [round(r, 4) for r in ratios]
    res["load_avg_1m_after"] = round(os.getloadavg()[0], 2)
    res["pass"] = res["modes"]["full"]["ratio_median"] <= a.limit
    with open(a.out, "w") as fh:
        json.dump(res, fh, indent=2, sort_keys=True)
        fh.write("\n")
    print(f"paired benchmark: base {res['base']}, head {res['head']}, {a.pairs} pairs, limit {a.limit}")
    for mode, m in res["modes"].items():
        print(
            f"  {mode:5} base {m['base_median_ms']:8.1f} ms  head {m['head_median_ms']:8.1f} ms  "
            f"ratio median {m['ratio_median']:.3f}  mean {m['ratio_mean']:.3f}  "
            f"min {m['ratio_min']:.3f}  max {m['ratio_max']:.3f}"
        )
    print(f"rule: the median ratio of full is {res['modes']['full']['ratio_median']:.3f}; {'pass' if res['pass'] else 'FAIL'}")
    sys.exit(0 if res["pass"] else 1)


if __name__ == "__main__":
    main()
