#!/usr/bin/env python3
"""Benchmark of the spec-dna binary on one corpus.

The harness never changes the corpus. It exports a git repository at --rev with
`git archive` into <work>/<label>/repo (it copies any other directory). It copies
--config in as spec-dna.toml. Then it makes the copy a new git repository with
one commit. Every measurement runs there.

Modes (one process for each run; the wall time and the peak RSS come from wait4):
    startup         spec-dna --version
    full            check --all --no-cache: parse every document, without the index
    cold            check --all after the harness deletes the index: parse and write it
    warm            check --all with the index, when nothing changed
    warm_base       check (base HEAD) with the index, when nothing changed
    context         context <id> --json for each --context id, with a warm index
    incremental     check (base HEAD) after --touch documents changed in the
                    working tree since spec-dna wrote the index

    python3 run.py --corpus PATH --config FILE --label NAME [--rev HEAD]
        [--runs 15] [--touch 10] [--context ID ...] [--work DIR] --out FILE.json
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
ROOT = os.path.dirname(os.path.dirname(HERE))
BIN = os.path.join(ROOT, "target", "release", "spec-dna")
GIT = ["git", "-c", "commit.gpgsign=false", "-c", "user.name=bench", "-c", "user.email=bench@example.org"]


def sh(*cmd, cwd=None):
    return subprocess.run(cmd, capture_output=True, text=True, check=True, cwd=cwd).stdout.strip()


def once(cmd, cwd, ok=(0, 1)):
    t0 = time.perf_counter()
    p = subprocess.Popen(cmd, cwd=cwd, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    out, err = p.stdout.read(), p.stderr.read()
    _, status, ru = os.wait4(p.pid, 0)
    wall = time.perf_counter() - t0
    code = os.waitstatus_to_exitcode(status)
    if code not in ok:
        raise RuntimeError(f"{cmd} exited {code}: {err.decode()[:400]}")
    rss = ru.ru_maxrss if sys.platform == "darwin" else ru.ru_maxrss * 1024
    return wall, rss, out.decode()


def stats(walls, rsss):
    ms = [w * 1000 for w in walls]
    return {
        "median_ms": round(statistics.median(ms), 2),
        "min_ms": round(min(ms), 2),
        "max_ms": round(max(ms), 2),
        "stdev_ms": round(statistics.stdev(ms), 2) if len(ms) > 1 else 0.0,
        "peak_rss_mb": round(max(rsss) / 1e6, 1),
        "runs": len(ms),
    }


def prepare(a):
    base = os.path.join(a.work, a.label)
    repo = os.path.join(base, "repo")
    shutil.rmtree(base, ignore_errors=True)
    os.makedirs(repo)
    if os.path.isdir(os.path.join(a.corpus, ".git")):
        tar = subprocess.Popen(["git", "-C", a.corpus, "archive", "--format=tar", a.rev], stdout=subprocess.PIPE)
        subprocess.run(["tar", "-x", "-C", repo], stdin=tar.stdout, check=True)
        if tar.wait() != 0:
            raise RuntimeError("git archive failed")
    else:
        shutil.copytree(a.corpus, repo, dirs_exist_ok=True, ignore=shutil.ignore_patterns(".git"))
    shutil.copy(a.config, os.path.join(repo, "spec-dna.toml"))
    sh(*GIT, "init", "-q", "-b", "main", cwd=repo)
    sh(*GIT, "add", "-A", "-f", cwd=repo)
    sh(*GIT, "commit", "-q", "-m", "snapshot", cwd=repo)
    return repo


def index_path(repo):
    return os.path.join(repo, ".git", "spec-dna", "index-v1.bin")


def summary(out):
    s = json.loads(out)["summary"]
    return {k: s[k] for k in ("documents", "errors", "warnings", "new_errors", "new_warnings")}


def strip_counts(out):
    d = json.loads(out)
    d["summary"].pop("parsed")
    d["summary"].pop("cached")
    return json.dumps(d, sort_keys=True)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--corpus", required=True)
    ap.add_argument("--config", required=True)
    ap.add_argument("--label", required=True)
    ap.add_argument("--rev", default="HEAD")
    ap.add_argument("--runs", type=int, default=15)
    ap.add_argument("--touch", type=int, default=10)
    ap.add_argument("--context", action="append", default=[])
    ap.add_argument("--work", default=os.path.join(ROOT, "bench", "work", "tool"))
    ap.add_argument("--out", required=True)
    a = ap.parse_args()

    repo = prepare(a)
    load_before = os.getloadavg()[0]
    res = {
        "label": a.label,
        "measured": time.strftime("%Y-%m-%d %H:%M:%S %z"),
        "host": {
            "machine": platform.machine(),
            "system": f"{platform.system()} {platform.release()}",
            "cpus": os.cpu_count(),
        },
        "tool": sh(BIN, "--version"),
        "git": sh("git", "--version"),
        "files": int(sh("git", "ls-files", "-z", cwd=repo).count("\0") + 1),
        "modes": {},
    }

    def measure(name, cmd, before=None, ok=(0, 1)):
        once(cmd, repo, ok)  # warm-up
        walls, rsss, outs = [], [], []
        for _ in range(a.runs):
            if before:
                before()
            w, r, o = once(cmd, repo, ok)
            walls.append(w)
            rsss.append(r)
            outs.append(o)
        res["modes"][name] = stats(walls, rsss)
        return outs

    def drop_index():
        try:
            os.remove(index_path(repo))
        except FileNotFoundError:
            pass

    measure("startup", [BIN, "--version"], ok=(0,))
    full = measure("full", [BIN, "check", "--all", "--json", "--no-cache"])
    cold = measure("cold", [BIN, "check", "--all", "--json"], before=drop_index)
    res["index_mb"] = round(os.path.getsize(index_path(repo)) / 1e6, 2)
    warm = measure("warm", [BIN, "check", "--all", "--json"])
    warm_base = measure("warm_base", [BIN, "check", "--json"])
    res["summary"] = summary(full[0])
    res["modes_agree"] = len({strip_counts(o) for o in full + cold + warm}) == 1
    res["warm_parsed"] = json.loads(warm[0])["summary"]["parsed"]
    for i, cid in enumerate(a.context):
        outs = measure(f"context_{i}", [BIN, "context", cid, "--json"], ok=(0,))
        d = json.loads(outs[0])
        res["modes"][f"context_{i}"].update(
            {"target": f"context {i}", "used_chars": d["used"], "units_full": d["full"], "units_stub": d["stubs"]}
        )

    # Incremental run: change documents in the working tree after spec-dna wrote the index.
    docs = [p for p in sh("git", "ls-files", cwd=repo).splitlines() if p.endswith(".md")]
    cfg_roots = json.loads(full[0])  # the documents that check saw
    touched = docs[:: max(1, len(docs) // a.touch)][: a.touch]
    for p in touched:
        with open(os.path.join(repo, p), "a") as fh:
            fh.write("\nAppended by the benchmark.\n")
    inc = measure("incremental", [BIN, "check", "--json"])
    res["touched"] = len(touched)
    res["incremental_parsed"] = json.loads(inc[0])["summary"]["parsed"]
    inc_full = once([BIN, "check", "--json", "--no-cache"], repo)[2]
    res["incremental_agrees_with_full"] = strip_counts(inc[0]) == strip_counts(inc_full)
    del cfg_roots
    res["load_avg_1m"] = [round(load_before, 2), round(os.getloadavg()[0], 2)]
    with open(a.out, "w") as fh:
        json.dump(res, fh, indent=2, sort_keys=True)
        fh.write("\n")
    print(json.dumps({k: res[k] for k in ("label", "summary", "modes_agree", "incremental_agrees_with_full", "load_avg_1m")}))
    for k, v in res["modes"].items():
        print(f"  {k:12} median {v['median_ms']:8.1f} ms  min {v['min_ms']:8.1f}  rss {v['peak_rss_mb']} MB")


if __name__ == "__main__":
    main()
