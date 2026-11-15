#!/usr/bin/env python3
"""Cost of the 4 stage skills of the `lean` harness (0.6.0, step 4, slice S8).

The script measures the first request of a headless `claude -p` run in a
temporary repository that `spec-dna init` prepared. One arm has the 4 skills
(`skills = ["intent", "spec", "plan", "review"]`). The other arm has none
(`skills = []`, no `REVIEW.md`). Each arm runs N times, in the order ON, OFF,
ON, OFF, ... The metric of a run is
`input_tokens + cache_creation_input_tokens + cache_read_input_tokens` of the
`usage` of the result. The cache state does not change this sum.

The script calls the network and spends money (about 0.07 to 0.16 USD a run).
It stops at once if one run costs more than `--stop-above-usd`, or if the
total passes `--max-total-usd`. It never installs spec-dna: it puts the binary
that `--bin` names on `PATH` of the `claude` process only.

Isolation of the run, in this order:
1. Remove the variables that Claude Code sets for a nested session and for
plugins (`ENV_UNSET`). `CLAUDE_CONFIG_DIR` stays, because the login is in the
keychain under a name that depends on this directory.
2. `--setting-sources project`: no user or local settings, so no user
`CLAUDE.md`, user skills, user agents, user hooks or user plugins.
3. `--strict-mcp-config`: no MCP server of the account.
The script checks the `system/init` event of each run. It reports the lists
that differ between the arms, and these must be the 4 skills only.

Usage:
python3 bench/harness/skills-cost.py --bin target/release/spec-dna --out bench/results/skills-cost-0.6.0.json
python3 bench/harness/skills-cost.py --bin target/release/spec-dna --prepare-only
"""

import argparse
import atexit
import datetime
import json
import os
import shutil
import statistics
import subprocess
import sys
import tempfile

WORD = "ok"
PROMPT = "Reply with the word %s." % WORD
THRESHOLD = 600
SKILLS = ["spec-dna-intent", "spec-dna-spec", "spec-dna-plan", "spec-dna-review"]
ENV_UNSET = [
    "CLAUDECODE",
    "CLAUDE_CODE_PLUGIN_DIRS",
    "CLAUDE_CODE_CHILD_SESSION",
    "CLAUDE_CODE_SESSION_ID",
    "CLAUDE_CODE_SESSION_ATTENDED",
    "CLAUDE_CODE_SUBAGENT_MODEL",
    "CLAUDE_CODE_ENTRYPOINT",
    "CLAUDE_CODE_MESSAGING_SOCKET",
    "CLAUDE_CODE_MESSAGING_TOKEN",
    "CLAUDE_EFFORT",
]
ISOLATION = (
    "unset the nested-session and plugin variables; "
    "--setting-sources project; --strict-mcp-config; "
    "CLAUDE_CONFIG_DIR kept for the login"
)
GIT = ["git", "-c", "commit.gpgsign=false", "-c", "user.name=Bench", "-c", "user.email=bench@example.org"]


def run(cmd, cwd, env=None, check=True):
    return subprocess.run(cmd, cwd=cwd, env=env, check=check, capture_output=True, text=True)


def prepare(spec_dna, root, arm):
    """Makes the repository of one arm. Both arms use the same steps."""
    repo = os.path.join(root, "prep-" + arm)
    os.makedirs(repo)
    env = dict(os.environ, GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL="/dev/null")
    run(["git", "init", "-q", "-b", "main", "."], repo, env)
    with open(os.path.join(repo, "README.md"), "w") as f:
        f.write("# Scratch\n")
    answers = os.path.join(root, "answers.toml")
    with open(answers, "w") as f:
        f.write('harness = "lean"\n')
    run([spec_dna, "init", "--answers", answers], repo, env)
    if arm == "off":
        # `init` does not delete a stale file, so the script removes the
        # skill folders and `REVIEW.md`, and then `init` updates the files
        # that remain (the page `model-team.md`).
        toml_path = os.path.join(repo, "spec-dna.toml")
        text = open(toml_path).read()
        text = text.replace('skills = ["intent", "spec", "plan", "review"]', "skills = []")
        text = text.replace("review-md = true", "review-md = false")
        for name in SKILLS:
            shutil.rmtree(os.path.join(repo, ".claude", "skills", name))
        os.remove(os.path.join(repo, "REVIEW.md"))
        with open(toml_path, "w") as f:
            f.write(text)
        run([spec_dna, "init"], repo, env)
    run(["git", "add", "-A"], repo, env)
    commit_env = dict(env, GIT_AUTHOR_DATE="2026-01-01T00:00:00Z", GIT_COMMITTER_DATE="2026-01-01T00:00:00Z")
    run(GIT + ["commit", "-q", "-m", "scratch"], repo, commit_env)
    return repo


def listing_bytes(repo):
    """Bytes of the lines `- name: description` of the 4 stage skills."""
    total = 0
    per = []
    for name in SKILLS:
        path = os.path.join(repo, ".claude", "skills", name, "SKILL.md")
        desc = None
        for line in open(path).read().split("\n"):
            if line.startswith("description: "):
                desc = line[len("description: "):]
                break
        n = len("- %s: %s\n" % (name, desc))
        per.append(n)
        total += n
    return total, per


def claude_env(bin_dir):
    env = {k: v for k, v in os.environ.items() if k not in ENV_UNSET}
    env["PATH"] = bin_dir + os.pathsep + env.get("PATH", "")
    return env


def one_run(prep, work, bin_dir):
    if os.path.exists(work):
        shutil.rmtree(work)
    shutil.copytree(prep, work, symlinks=True)
    cmd = [
        "claude", "-p", PROMPT,
        "--output-format", "stream-json", "--verbose",
        "--max-turns", "1", "--max-budget-usd", "0.3",
        "--permission-mode", "dontAsk",
        "--setting-sources", "project", "--strict-mcp-config",
        "--no-session-persistence",
    ]
    p = run(cmd, work, claude_env(bin_dir), check=False)
    init, result = None, None
    for line in p.stdout.splitlines():
        try:
            e = json.loads(line)
        except ValueError:
            continue
        if e.get("type") == "system" and e.get("subtype") == "init":
            init = e
        if e.get("type") == "result":
            result = e
    if result is None or result.get("is_error") or init is None:
        sys.exit("claude run failed (exit %d); no spend is lost beyond the earlier runs" % p.returncode)
    u = result["usage"]
    total = u["input_tokens"] + u["cache_creation_input_tokens"] + u["cache_read_input_tokens"]
    names = lambda key: sorted(x if isinstance(x, str) else x.get("name", "?") for x in init.get(key, []))
    return {
        "input_tokens": u["input_tokens"],
        "cache_creation_input_tokens": u["cache_creation_input_tokens"],
        "cache_read_input_tokens": u["cache_read_input_tokens"],
        "total_tokens": total,
        "cost_usd": round(result["total_cost_usd"], 4),
    }, {k: names(k) for k in ("skills", "agents", "plugins", "mcp_servers", "tools")}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--bin", required=True, help="the spec-dna binary to test")
    ap.add_argument("--out", help="the result file")
    ap.add_argument("--runs", type=int, default=3)
    ap.add_argument("--stop-above-usd", type=float, default=0.5)
    ap.add_argument("--max-total-usd", type=float, default=3.0)
    ap.add_argument("--prepare-only", action="store_true", help="make the 2 repositories, run no claude")
    a = ap.parse_args()

    spec_dna = os.path.abspath(a.bin)
    root = tempfile.mkdtemp(prefix="skills-cost-")
    atexit.register(shutil.rmtree, root, ignore_errors=True)
    bin_dir = os.path.join(root, "bin")
    os.makedirs(bin_dir)
    os.symlink(spec_dna, os.path.join(bin_dir, "spec-dna"))
    spec_dna = os.path.join(bin_dir, "spec-dna")
    preps = {arm: prepare(spec_dna, root, arm) for arm in ("on", "off")}
    lb, per = listing_bytes(preps["on"])
    print("repositories ready under", root, "listing bytes", lb, per)
    if a.prepare_only:
        return

    work = os.path.join(root, "work")  # both arms run at the same path
    runs = {"on": [], "off": []}
    inits = {"on": [], "off": []}
    total_cost = 0.0
    for i in range(a.runs):
        for arm in ("on", "off"):
            r, init = one_run(preps[arm], work, bin_dir)
            runs[arm].append(r)
            inits[arm].append(init)
            total_cost += r["cost_usd"]
            print("run %d %s: %d tokens, %.4f USD" % (i + 1, arm, r["total_tokens"], r["cost_usd"]))
            if r["cost_usd"] > a.stop_above_usd or total_cost > a.max_total_usd:
                sys.exit("stop: a run or the total costs too much (total %.4f USD)" % total_cost)

    med = {arm: statistics.median(r["total_tokens"] for r in runs[arm]) for arm in runs}
    # What differs between the arms in the init event, by list.
    differ = {}
    for key in ("skills", "agents", "plugins", "mcp_servers", "tools"):
        on = set().union(*(set(x[key]) for x in inits["on"]))
        off = set().union(*(set(x[key]) for x in inits["off"]))
        if on != off:
            differ[key] = {"only_on": sorted(on - off), "only_off": sorted(off - on)}
    stable = all(x == inits[arm][0] for arm in inits for x in inits[arm])
    diff = med["on"] - med["off"]
    claude_version = run(["claude", "--version"], root, claude_env(bin_dir)).stdout.strip()
    commit = run(["git", "rev-parse", "--short", "HEAD"], os.path.dirname(os.path.abspath(__file__))).stdout.strip()
    out = {
        "measure": "skills-cost",
        "step": "0.6.0 step 4, slice S8",
        "date": datetime.date.today().isoformat(),
        "claude_version": claude_version,
        "spec_dna_commit": commit,
        "spec_dna_version": run([spec_dna, "--version"], root).stdout.strip(),
        "prompt_word": WORD,
        "isolation": ISOLATION,
        "runs_per_arm": a.runs,
        "order": "on, off, on, off, ...",
        "listing_bytes": lb,
        "listing_bytes_per_skill": per,
        "threshold_tokens": THRESHOLD,
        "runs": {"skills_on": runs["on"], "skills_off": runs["off"]},
        "median_tokens_skills_on": med["on"],
        "median_tokens_skills_off": med["off"],
        "median_difference_tokens": diff,
        "above_threshold": diff > THRESHOLD,
        "init_lists_stable_in_each_arm": stable,
        "init_lists_that_differ_between_arms": differ,
        "total_cost_usd": round(total_cost, 4),
    }
    text = json.dumps(out, indent=2) + "\n"
    if a.out:
        with open(a.out, "w") as f:
            f.write(text)
    print(text)


if __name__ == "__main__":
    main()
