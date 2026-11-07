#!/usr/bin/env python3
"""Harness for the git access benchmark (ADR 0002, confirmation step 2).

    python3 run.py prepare --source DIR|GIT_DIR --rev REV|- --out REPO
    python3 run.py measure --repo REPO --label NAME --cli BIN --gix BIN [--git-direct GIT] [--runs 15] --out FILE.json

prepare: builds a fresh repository from a snapshot and never writes to the source.
    A git source is exported with `git archive REV`; with `--rev -` the directory is
    copied. Then: commit "base", change 10 .md files and commit "change", pack with
    `git gc`, change 10 other .md files in the working tree and add 3 untracked .md
    files. The base revision for measurements is the tag `bench-base`.

measure: for each blob selection (`changed`, `md`) runs 1 warm-up per mode, then
    --runs runs per mode, interleaved run by run, rotating which mode goes first.
    Modes: `cli` and `cli-par` (git from PATH), `cli-direct` and `cli-par-direct`
    (the same with --git-direct, optional), `gix`. Wall time is measured here around
    each process; step times and peak RSS (getrusage, self and largest child) come
    from the binary's stderr. Every stdout must be identical across runs and modes.
    Unix only.
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

GIT_ENV = {
    "GIT_AUTHOR_NAME": "bench", "GIT_AUTHOR_EMAIL": "bench@example.invalid",
    "GIT_COMMITTER_NAME": "bench", "GIT_COMMITTER_EMAIL": "bench@example.invalid",
}
GIT_SAFE = ["-c", "commit.gpgsign=false", "-c", "core.hooksPath=/dev/null", "-c", "tag.gpgsign=false",
            "-c", "gc.auto=0", "-c", "maintenance.auto=false"]


def git(repo, *args, date=None, capture=True):
    env = dict(os.environ, **GIT_ENV)
    if date:
        env["GIT_AUTHOR_DATE"] = env["GIT_COMMITTER_DATE"] = date
    cmd = ["git", "-C", repo, *GIT_SAFE, *args]
    p = subprocess.run(cmd, env=env, check=True, capture_output=capture, text=True)
    return p.stdout if capture else None


def append(path, text):
    with open(path, "a", encoding="utf-8") as fh:
        fh.write(text)


def tree_bytes(repo):
    """Sum of blob sizes in the base tree."""
    total = 0
    for rec in git(repo, "ls-tree", "-r", "-l", "-z", "bench-base").split("\0"):
        if rec:
            mode, kind, oid, size = rec.split("\t")[0].split()
            total += int(size) if kind == "blob" else 0
    return total


def prepare(a):
    out = a.out
    if os.path.exists(out):
        shutil.rmtree(out)
    os.makedirs(out)
    if a.rev != "-":
        src = subprocess.Popen(["git", "-C", a.source, "archive", "--format=tar", a.rev], stdout=subprocess.PIPE)
        subprocess.run(["tar", "-x", "-C", out], stdin=src.stdout, check=True)
        if src.wait() != 0:
            sys.exit("git archive failed")
    else:
        shutil.copytree(a.source, out, dirs_exist_ok=True, symlinks=True)
    subprocess.run(["git", "init", "-q", "--template=", "-b", "main", out], check=True)
    git(out, "add", "-A")
    git(out, "commit", "-q", "-m", "base", date="2026-01-01T00:00:00Z")
    git(out, "tag", "bench-base")
    md = sorted(p for p in git(out, "ls-files", "-z", "--", "*.md").split("\0") if p)
    if len(md) < 20:
        sys.exit(f"need at least 20 .md files, found {len(md)}")
    picks = [md[int((k + 0.5) * len(md) / 20)] for k in range(20)]
    committed, worktree = picks[0::2], picks[1::2]
    for i, p in enumerate(committed):
        append(os.path.join(out, p), f"\nBench change {i} committed after the base.\n")
    git(out, "commit", "-q", "-am", "change", date="2026-01-02T00:00:00Z")
    git(out, "gc", "-q")
    for i, p in enumerate(worktree):
        append(os.path.join(out, p), f"\nBench change {i} left in the working tree.\n")
    untracked = [
        os.path.join(os.path.dirname(committed[0]), "bench-untracked-1.md"),
        "bench-untracked-2.md",
        os.path.join("bench-new-dir", "sub", "bench-untracked-3.md"),
    ]
    for p in untracked:
        os.makedirs(os.path.join(out, os.path.dirname(p)) or out, exist_ok=True)
        with open(os.path.join(out, p), "w", encoding="utf-8") as fh:
            fh.write("# Untracked\n\nCreated by the benchmark.\n")
    # Refresh the stat data once (with locks) after a pause, so no index entry is racily clean.
    time.sleep(1.2)
    git(out, "status", "--porcelain")
    st = git(out, "status", "--porcelain=v1", "-z", "--untracked-files=all", "--ignored").split("\0")
    info = {
        "tracked_files": len([p for p in git(out, "ls-files", "-z").split("\0") if p]),
        "md_files": len(md),
        "worktree_modified": sum(1 for r in st if r[:2] == " M"),
        "untracked": sum(1 for r in st if r[:2] == "??"),
        "ignored": sum(1 for r in st if r[:2] == "!!"),
        "tree_bytes": tree_bytes(out),
    }
    print(json.dumps(info, indent=2))
    with open(out + ".prepare.json", "w") as fh:
        json.dump(info, fh, indent=2)


def once(cmd, env=None):
    t0 = time.perf_counter()
    p = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env)
    out, err = p.communicate()
    wall = (time.perf_counter() - t0) * 1000
    if p.returncode != 0:
        raise RuntimeError(f"{cmd} exited {p.returncode}: {err.decode()[:800]}")
    timing = {}
    for line in err.decode().splitlines():
        if line.startswith("timing "):
            for kv in line.split()[1:]:
                k, _, v = kv.partition("=")
                timing[k] = float(v)
    return wall, out.decode(), timing


def stats(values):
    return {"median": round(statistics.median(values), 2), "min": round(min(values), 2),
            "max": round(max(values), 2), "stdev": round(statistics.stdev(values), 2) if len(values) > 1 else 0.0}


def measure(a):
    # name -> (binary, mode argument, environment)
    direct = dict(os.environ, GIT_ACCESS_GIT=a.git_direct) if a.git_direct else None
    modes = {"cli": (a.cli, "cli", None)}
    if direct:
        modes["cli-direct"] = (a.cli, "cli", direct)
    modes["cli-par"] = (a.cli, "cli-par", None)
    if direct:
        modes["cli-par-direct"] = (a.cli, "cli-par", direct)
    modes["gix"] = (a.gix, "gix", None)
    load0 = os.getloadavg()[0]
    git_version = subprocess.run(["git", "--version"], capture_output=True, text=True).stdout.strip()
    result = {
        "label": a.label, "runs": a.runs, "load_avg_1m": {"start": round(load0, 2)},
        "git": git_version, "git_direct": a.git_direct, "platform": platform.platform(),
        "cpus": os.cpu_count(), "selections": {},
    }
    try:
        result["prepare"] = json.load(open(os.path.abspath(a.repo) + ".prepare.json"))
    except OSError:
        pass
    names = list(modes)
    for which in ("changed", "md"):
        outputs = {}
        rows = {m: [] for m in names}

        def run(m, tag):
            exe, arg, env = modes[m]
            wall, out, timing = once([exe, arg, a.repo, "bench-base", which], env)
            outputs.setdefault(out, []).append(f"{m} {tag}")
            timing["wall_ms"] = wall
            return timing

        for m in names:
            run(m, "warm-up")
        for i in range(a.runs):
            k = i % len(names)  # rotate the order so each mode goes first equally often
            for m in names[k:] + names[:k]:
                rows[m].append(run(m, f"run {i}"))
        equal = len(outputs) == 1
        sel = {"outputs_equal": equal, "summary": next(iter(outputs)) if equal else outputs, "modes": {}}
        for m in names:
            keys = ["open_ms", "head_ms", "base_ms", "blobs_ms", "steps_ms", "wall_ms"]
            sel["modes"][m] = {k: stats([r[k] for r in rows[m]]) for k in keys}
            sel["modes"][m]["rss_self_mb"] = round(max(r["rss_self_kib"] for r in rows[m]) / 1024, 1)
            sel["modes"][m]["rss_child_max_mb"] = round(max(r["rss_child_max_kib"] for r in rows[m]) / 1024, 1)
        result["selections"][which] = sel
        print(f"{a.label} {which}: equal={sel['outputs_equal']} " + " ".join(
            f"{m} {sel['modes'][m]['wall_ms']['median']}" for m in names) + " ms", flush=True)
    result["load_avg_1m"]["end"] = round(os.getloadavg()[0], 2)
    with open(a.out, "w") as fh:
        json.dump(result, fh, indent=2)
        fh.write("\n")


def main():
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("prepare")
    p.add_argument("--source", required=True)
    p.add_argument("--rev", required=True)
    p.add_argument("--out", required=True)
    m = sub.add_parser("measure")
    m.add_argument("--repo", required=True)
    m.add_argument("--label", required=True)
    m.add_argument("--cli", required=True)
    m.add_argument("--gix", required=True)
    m.add_argument("--git-direct", help="also run cli mode with this git executable (GIT_ACCESS_GIT)")
    m.add_argument("--runs", type=int, default=15)
    m.add_argument("--out", required=True)
    a = ap.parse_args()
    prepare(a) if a.cmd == "prepare" else measure(a)


if __name__ == "__main__":
    main()
