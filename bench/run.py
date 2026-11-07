#!/usr/bin/env python3
"""Benchmark harness: it runs the Go and Rust scanners on any corpus path.

The harness never changes the corpus. It exports a git repository at --rev with
`git archive` into <work>/<label>/tree (default work: bench/work). It copies any other directory there.
Every measurement runs on that copy.

Modes (each run is one process; the wall time and the peak RSS come from wait4):
    startup      scan --version
    full         full scan without the index (the OS file cache is warm, see README)
    index_build  full scan that also writes the index
    warm         scan with an up-to-date index, when nothing changed
    incremental  scan with the index, after --touch files changed

    python3 run.py --corpus PATH --label NAME [--rev HEAD] [--runs 10] [--touch 10] [--work DIR] --out FILE.json
It runs only on Unix (os.wait4).
"""
import argparse
import json
import os
import platform
import shutil
import statistics
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
BINS = {"go": os.path.join(HERE, "go", "scan-go"), "rust": os.path.join(HERE, "rust", "target", "release", "scan")}


def sh(*cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, check=True, **kw).stdout.strip()


def build():
    sh("go", "build", "-trimpath", "-ldflags=-s -w", "-o", BINS["go"], ".", cwd=os.path.join(HERE, "go"))
    sh("cargo", "build", "--release", "--quiet", cwd=os.path.join(HERE, "rust"))


def once(cmd):
    """Runs cmd once. Returns (wall seconds, peak RSS bytes, stdout)."""
    t0 = time.perf_counter()
    p = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    out = p.stdout.read()
    err = p.stderr.read()
    _, status, ru = os.wait4(p.pid, 0)
    wall = time.perf_counter() - t0
    p.returncode = os.waitstatus_to_exitcode(status)
    if p.returncode != 0:
        raise RuntimeError(f"{cmd} exited {p.returncode}: {err.decode()[:500]}")
    rss = ru.ru_maxrss if sys.platform == "darwin" else ru.ru_maxrss * 1024
    return wall, rss, out.decode()


def parse_out(out):
    res = {}
    for line in out.splitlines():
        k, _, v = line.partition(" ")
        if v.isdigit():
            res[k] = int(v)
    return res


def snapshot(corpus, rev, dest):
    if os.path.exists(dest):
        shutil.rmtree(dest)
    os.makedirs(dest)
    if os.path.isdir(os.path.join(corpus, ".git")) or rev:
        git = subprocess.Popen(["git", "-C", corpus, "archive", "--format=tar", rev or "HEAD"], stdout=subprocess.PIPE)
        subprocess.run(["tar", "-x", "-C", dest], stdin=git.stdout, check=True)
        if git.wait() != 0:
            raise RuntimeError("git archive failed")
        return sh("git", "-C", corpus, "rev-parse", rev or "HEAD")
    shutil.copytree(corpus, dest, dirs_exist_ok=True, symlinks=True)
    return None


def hardware():
    hw = {"platform": platform.platform(), "machine": platform.machine(), "cpus": os.cpu_count()}
    if sys.platform == "darwin":
        hw["cpu"] = sh("sysctl", "-n", "machdep.cpu.brand_string")
        hw["mem_gb"] = int(sh("sysctl", "-n", "hw.memsize")) // 2**30
    else:
        try:
            with open("/proc/cpuinfo") as fh:
                hw["cpu"] = next(l.split(":", 1)[1].strip() for l in fh if l.startswith("model name"))
        except (OSError, StopIteration):
            pass
    hw["go"] = sh("go", "version")
    hw["rustc"] = sh("rustc", "--version")
    return hw


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--corpus", required=True)
    ap.add_argument("--label", required=True)
    ap.add_argument("--rev")
    ap.add_argument("--runs", type=int, default=10)
    ap.add_argument("--touch", type=int, default=10)
    ap.add_argument("--exclude", action="append", default=[])
    ap.add_argument("--out", required=True)
    ap.add_argument("--work", default=os.path.join(HERE, "work"), help="scratch root for the snapshot and index")
    ap.add_argument("--no-build", action="store_true")
    a = ap.parse_args()
    if not a.no_build:
        build()
    work = os.path.join(os.path.abspath(a.work), a.label)
    tree = os.path.join(work, "tree")
    t0 = time.perf_counter()
    rev = snapshot(os.path.abspath(a.corpus), a.rev, tree)
    snap_s = time.perf_counter() - t0
    excl = sum((["--exclude", e] for e in a.exclude), [])
    docs = sorted(os.path.join(dp, f) for dp, dn, fn in os.walk(tree) for f in fn if f.endswith(".md"))
    step = max(1, len(docs) // max(1, a.touch))
    touched = docs[::step][: a.touch]
    originals = {}
    for p in touched:
        with open(p, "rb") as fh:
            originals[p] = fh.read()

    res = {
        "label": a.label, "rev": rev, "snapshot_s": round(snap_s, 2), "hardware": hardware(),
        "touch": len(touched), "binaries": {k: os.path.getsize(v) for k, v in BINS.items()}, "scanners": {},
    }
    expected = None
    exp_path = os.path.join(tree, "expected.json")
    if os.path.exists(exp_path):
        with open(exp_path) as fh:
            expected = json.load(fh)
    def load():
        return os.getloadavg()[0]

    res["load_avg_1m"] = {"start": round(load(), 2)}
    state = {}
    for name, binp in BINS.items():
        idx = os.path.join(work, f"{name}.idx")
        state[name] = {
            "idx": idx, "base": idx + ".base", "full": [binp, *excl, tree],
            "index": [binp, "--index", idx, *excl, tree], "version": [binp, "--version"], "n": 0,
        }
        res["scanners"][name] = {"first_full_run_ms": round(once(state[name]["full"])[0] * 1000, 2)}

    def rm_idx(st):
        if os.path.exists(st["idx"]):
            os.remove(st["idx"])

    def touch(st):
        shutil.copyfile(st["base"], st["idx"])
        st["n"] += 1
        for p in touched:
            with open(p, "ab") as fh:
                fh.write(b"\nTouched %d.\n" % st["n"])

    def restore():
        for p, b in originals.items():
            with open(p, "wb") as fh:
                fh.write(b)

    # In each mode, the scanners alternate run by run, so that load drift on a
    # shared host has the same effect on both.
    modes = [
        ("startup", "version", None, max(a.runs, 20)), ("full", "full", None, a.runs),
        ("index_build", "index", rm_idx, a.runs), ("warm", "index", None, a.runs),
        ("incremental", "index", touch, a.runs),
    ]
    try:
        for mode, cmd, prep, runs in modes:
            if mode == "warm":
                for name, st in state.items():
                    res["scanners"][name]["index_bytes"] = os.path.getsize(st["idx"])
                    shutil.copyfile(st["idx"], st["base"])
            walls = {n: [] for n in state}
            rss = {n: [] for n in state}
            outs = {}
            for i in range(runs + 1):              # run 0 is a warm-up
                order = list(state.items())
                if i % 2:                          # alternate the scanner that goes first
                    order.reverse()
                for name, st in order:
                    if prep:
                        prep(st)
                    w, r, out = once(st[cmd])
                    if i:
                        walls[name].append(w * 1000)
                        rss[name].append(r)
                        outs[name] = out
            if mode == "incremental":
                restore()
            for name in state:
                v = walls[name]
                res["scanners"][name][mode] = {
                    "runs": runs, "median_ms": round(statistics.median(v), 2), "min_ms": round(min(v), 2),
                    "max_ms": round(max(v), 2), "stdev_ms": round(statistics.stdev(v), 2) if runs > 1 else 0,
                    "peak_rss_mb": round(max(rss[name]) / 1048576, 1), "last_output": parse_out(outs[name])}
    finally:
        restore()
    res["load_avg_1m"]["end"] = round(load(), 2)
    outs = {n: {k: v for k, v in r["full"]["last_output"].items() if k != "parsed"} for n, r in res["scanners"].items()}
    res["outputs_equal"] = outs["go"] == outs["rust"]
    # The index must not change the answer. The index_build and warm runs must give the
    # output of the full scan. The incremental run can differ only in the bytes that the
    # touched files gained.
    def same(n, m):
        skip = {"parsed", "bytes"} if m == "incremental" else {"parsed"}
        got = {k: v for k, v in res["scanners"][n][m]["last_output"].items() if k not in skip}
        return got == {k: v for k, v in outs[n].items() if k not in skip}
    res["modes_agree"] = all(same(n, m) for n in res["scanners"] for m in ("index_build", "warm", "incremental"))
    res["corpus"] = outs["go"]
    if expected:
        res["matches_expected"] = all(outs["go"].get(k) == v for k, v in expected.items())
    with open(a.out, "w") as fh:
        json.dump(res, fh, indent=2, sort_keys=True)
        fh.write("\n")
    modes_out = ("startup", "full", "index_build", "warm", "incremental")
    brief = {n: {m: (r[m]["median_ms"], r[m]["peak_rss_mb"]) for m in modes_out} for n, r in res["scanners"].items()}
    print(json.dumps({
        "label": a.label, "load": res["load_avg_1m"], "corpus": res["corpus"], "equal": res["outputs_equal"],
        "modes_agree": res["modes_agree"],
        "expected": res.get("matches_expected"), "median_ms_rss_mb": brief,
    }))


if __name__ == "__main__":
    main()
