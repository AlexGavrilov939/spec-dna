"""Scores and verdicts of protocol 2 (ADR 0033).

Usage: analyze.py <arms result file> [--counts <transcript_counts json>[,<json>...]] [--json]
        analyze.py --selftest

The result file is `evals/results/arms-<stamp>.json` of `spec-dna eval run --arms`.
The file holds the checks of each run. A check `p1-NN` is a test of part 1, and a
check `p2-NN` is a test of part 2. The score of a run is the share of the checks
of each part that passed. A run whose `subtype` is not `success` hit a limit
(turns, budget or time), and it scores 0 in both parts.

The main measure is the score of part 2. The verdict of a treatment X against a
control Y uses the mean score of each task, the bootstrap by task (seed 24,
2,000 resamples, 90% interval), and the ratio of the mean costs. It uses the
complete waves only (a wave is complete when each task ran once in each arm).

Verdict (the rules are in ADR 0033):
- `better`: the low end of the interval of the difference is above 0.
- `worse`: the high end of the interval of the difference is below 0.
- `no-gain-at-cost`: not `better`, not `worse`, the high end of the difference is
    below 0.20, and the low end of the ratio of the costs is above 1.00.
- `inconclusive`: each other result.

With `--counts` (the output of `transcript_counts.py --json`), the command matches
the counts to the runs, takes the turns of a run from its transcript (the runner
records the turns of the last wake-up segment only), and gives 3 variants of the
arms table and of the comparisons: all runs, the runs without a hit of the pattern of
the ADR (`leaked_adr`), and the runs without a hit of the stricter column
(`leaked_strict`). Each variant takes the floor of the verdict from all runs.
"""

import collections
import json
import random
import sys
from pathlib import Path

SEED = 24
RESAMPLES = 2000
LEVEL = 0.90
MIN_GAIN = 0.20
TASKS_FLOOR = 5
WAVES_FLOOR = 3


def share(checks: list[dict], part: str) -> float | None:
    mine = [c for c in checks if c["name"].startswith(part + "-")]
    if not mine:
        return None
    return sum(1 for c in mine if c["passed"]) / len(mine)


def runs_of(result: dict) -> list[dict]:
    """One record for each side that ran, in the complete waves only."""
    waves = result.get("waves", 0)
    out = []
    for item in result["evals"]:
        for res in item["results"]:
            if res["run"] > waves:
                continue
            for arm, side in res["arms"].items():
                if side is None:
                    continue
                limit = side.get("subtype") != "success"
                out.append(
                    {
                        "task": item["id"],
                        "run": res["run"],
                        "arm": arm,
                        "limit": limit,
                        "p1": 0.0 if limit else (share(side["checks"], "p1") or 0.0),
                        "p2": 0.0 if limit else (share(side["checks"], "p2") or 0.0),
                        "cost": side.get("cost_usd") or (item.get("max_budget_usd") or 0.0),
                        "seconds": side.get("seconds") or 0.0,
                        "turns": side.get("turns") or 0,
                        "excluded": side.get("excluded", 0),
                    }
                )
    return out


def task_means(runs: list[dict], arm: str, key: str) -> dict[str, float]:
    by_task = collections.defaultdict(list)
    for r in runs:
        if r["arm"] == arm:
            by_task[r["task"]].append(r[key])
    return {t: sum(v) / len(v) for t, v in by_task.items()}


def percentile_interval(values: list[float]) -> tuple[float, float]:
    values = sorted(values)
    n = len(values)
    lo = int((1 - LEVEL) / 2 * n)
    hi = int((1 - (1 - LEVEL) / 2) * n) - 1
    return values[lo], values[hi]


def compare(runs: list[dict], treatment: str, control: str, key: str = "p2", floor_ok: bool | None = None) -> dict:
    x, y = task_means(runs, treatment, key), task_means(runs, control, key)
    cx, cy = task_means(runs, treatment, "cost"), task_means(runs, control, "cost")
    tasks = sorted(set(x) & set(y))
    waves = min(
        (len({r["run"] for r in runs if r["arm"] == a and r["task"] == t}) for a in (treatment, control) for t in tasks),
        default=0,
    )
    out = {"treatment": treatment, "control": control, "tasks": len(tasks), "waves": waves, "key": key}
    if not tasks:
        return {**out, "verdict": "inconclusive", "below_floor": True}
    diff = sum(x[t] - y[t] for t in tasks) / len(tasks)
    ratio = sum(cx[t] for t in tasks) / max(sum(cy[t] for t in tasks), 1e-9)
    rng = random.Random(SEED)
    diffs, ratios = [], []
    for _ in range(RESAMPLES):
        pick = [rng.choice(tasks) for _ in tasks]
        diffs.append(sum(x[t] - y[t] for t in pick) / len(pick))
        ratios.append(sum(cx[t] for t in pick) / max(sum(cy[t] for t in pick), 1e-9))
    d_lo, d_hi = percentile_interval(diffs)
    r_lo, r_hi = percentile_interval(ratios)
    below_floor = (len(tasks) < TASKS_FLOOR or waves < WAVES_FLOOR) if floor_ok is None else not floor_ok
    if below_floor:
        verdict = "inconclusive"
    elif d_lo > 0:
        verdict = "better"
    elif d_hi < 0:
        verdict = "worse"
    elif d_hi < MIN_GAIN and r_lo > 1.0:
        verdict = "no-gain-at-cost"
    else:
        verdict = "inconclusive"
    return {
        **out,
        "mean_treatment": sum(x[t] for t in tasks) / len(tasks),
        "mean_control": sum(y[t] for t in tasks) / len(tasks),
        "difference": diff,
        "difference_interval": [d_lo, d_hi],
        "cost_ratio": ratio,
        "cost_ratio_interval": [r_lo, r_hi],
        "verdict": verdict,
        "below_floor": below_floor,
    }


def arm_table(runs: list[dict]) -> dict:
    table = {}
    for arm in sorted({r["arm"] for r in runs}):
        mine = [r for r in runs if r["arm"] == arm]
        n = len(mine)
        table[arm] = {
            "runs": n,
            "p1": sum(r["p1"] for r in mine) / n,
            "p2": sum(r["p2"] for r in mine) / n,
            "limit_runs": sum(1 for r in mine if r["limit"]),
            "cost_per_run": sum(r["cost"] for r in mine) / n,
            "minutes_per_run": sum(r["seconds"] for r in mine) / n / 60,
            "turns_per_run": sum(r["turns"] for r in mine) / n,
            "cost_total": sum(r["cost"] for r in mine),
        }
    return table


def match_counts(runs: list[dict], counts: dict | None) -> dict[str, set]:
    """Match the transcript counts to the runs, and set the turns and the leak counts of each run.

    `runs` holds the runs of all waves, also of a wave that is not complete: the tap
    log has lines for them. The counts of a task and an arm come in the order of their
    start, and the runs come in the order of their numbers (the scheduler starts a job
    only after the job of the wave before it). A run with `n` excluded attempts has
    `n` lines before the line that counts. If the number of lines differs from what
    the runs expect (an attempt that a signal stopped), one leaked line marks each run of
    the task and the arm, and the turns of the result stay.
    """
    leaked = {"adr": set(), "strict": set()}
    if not counts:
        return leaked
    order = collections.defaultdict(list)
    for c in sorted(counts["runs"], key=lambda c: c.get("epoch", 0)):
        order[(c["task"], c["arm"])].append(c)
    for (task, arm), lines in order.items():
        mine = sorted((r for r in runs if r["task"] == task and r["arm"] == arm), key=lambda r: r["run"])
        exact = len(lines) == sum(r["excluded"] + 1 for r in mine)
        counted: list[dict] = []
        if exact:
            at = 0
            for r in mine:
                at += r["excluded"]
                counted.append(lines[at])
                at += 1
        for column, key in (("adr", "hidden_touches"), ("strict", "strict_touches")):
            if exact:
                leaked[column].update((task, r["run"], arm) for r, c in zip(mine, counted) if c.get(key))
            elif any(c.get(key) for c in lines):
                leaked[column].update((task, r["run"], arm) for r in mine)
        for r, c in zip(mine, counted):
            r["hidden_touches"] = c.get("hidden_touches", 0)
            r["strict_touches"] = c.get("strict_touches", 0)
            r["advisor_calls"] = c.get("advisor_calls", 0)
            r["turns_recorded"] = r["turns"]
            # The runner records the turns of the last wake-up segment, so the
            # transcript count is the turns of the run (see transcript_counts.py).
            if c.get("turns"):
                r["turns"] = c["turns"]
    for r in runs:
        r["leaked_adr"] = (r["task"], r["run"], r["arm"]) in leaked["adr"]
        r["leaked_strict"] = (r["task"], r["run"], r["arm"]) in leaked["strict"]
    return leaked


def variant(runs: list[dict], floor_ok: bool) -> dict:
    arms = arm_table(runs)
    for arm, table in arms.items():
        calls = [r["advisor_calls"] for r in runs if r["arm"] == arm and "advisor_calls" in r]
        if calls:
            table["advisor_calls_per_run"] = sum(calls) / len(calls)
    names = set(arms)
    comparisons = []
    for treatment, control in (("lean", "lean-no-advisor"), ("lean", "none"), ("lean-no-advisor", "none")):
        if treatment in names and control in names:
            c = compare(runs, treatment, control, floor_ok=floor_ok)
            c["role"] = "verdict" if control != "none" or treatment == "lean" else "description"
            c["part1"] = compare(runs, treatment, control, key="p1", floor_ok=floor_ok)
            comparisons.append(c)
    return {"arms": arms, "comparisons": comparisons}


def analyze(result: dict, counts: dict | None) -> dict:
    """The arms table and the comparisons, for all runs, and without the runs that
    each leak column marks. The floor of each variant comes from all runs (ADR 0033)."""
    waves_all = result.get("waves", 0)
    everything = runs_of({**result, "waves": 10**6})
    leaked = match_counts(everything, counts)
    runs = [r for r in everything if r["run"] <= waves_all]
    floor_ok = waves_all >= WAVES_FLOOR and len({r["task"] for r in runs}) >= TASKS_FLOOR
    variants = {"all": variant(runs, floor_ok)}
    if counts:
        variants["without-adr-leaked"] = variant([r for r in runs if not r["leaked_adr"]], floor_ok)
        variants["without-strict-leaked"] = variant([r for r in runs if not r["leaked_strict"]], floor_ok)
    return {
        "waves": waves_all,
        "leaked_runs_adr": sorted(f"{t}#{n}:{a}" for t, n, a in leaked["adr"]),
        "leaked_runs_strict": sorted(f"{t}#{n}:{a}" for t, n, a in leaked["strict"]),
        "variants": variants,
        "runs": runs,
    }


def selftest() -> int:
    def synth(means: dict[str, list[float]], cost: dict[str, float]) -> list[dict]:
        out = []
        for arm, per_task in means.items():
            for t, m in enumerate(per_task):
                for run in (1, 2, 3):
                    out.append(
                        {"task": f"t{t}", "run": run, "arm": arm, "limit": False, "p1": 1.0, "p2": m,
                        "cost": cost[arm], "seconds": 60.0, "turns": 10, "excluded": 0}
                    )
        return out

    ok = True

    def expect(name: str, got: str, want: str) -> None:
        nonlocal ok
        print(f"{'ok  ' if got == want else 'FAIL'} {name}: {got}")
        ok = ok and got == want

    # A clear gain: each task gains 0.4 for 3 times the cost.
    runs = synth({"lean": [0.9] * 5, "none": [0.5] * 5}, {"lean": 3.0, "none": 1.0})
    expect("gain", compare(runs, "lean", "none")["verdict"], "better")
    # No change at 3 times the cost.
    runs = synth({"lean": [0.5] * 5, "none": [0.5] * 5}, {"lean": 3.0, "none": 1.0})
    expect("no gain at cost", compare(runs, "lean", "none")["verdict"], "no-gain-at-cost")
    # A loss.
    runs = synth({"lean": [0.2] * 5, "none": [0.6] * 5}, {"lean": 1.0, "none": 1.0})
    expect("loss", compare(runs, "lean", "none")["verdict"], "worse")
    # A mixed result: 2 tasks gain and 3 lose.
    runs = synth({"lean": [0.9, 0.9, 0.3, 0.3, 0.3], "none": [0.3, 0.3, 0.9, 0.9, 0.9]}, {"lean": 3.0, "none": 1.0})
    expect("mixed", compare(runs, "lean", "none")["verdict"], "inconclusive")
    # Below the floor: 4 tasks.
    runs = synth({"lean": [0.9] * 4, "none": [0.5] * 4}, {"lean": 3.0, "none": 1.0})
    expect("floor", compare(runs, "lean", "none")["verdict"], "inconclusive")
    # A run with no cost counts as its budget.
    side = {"claude_exit": 1, "subtype": "error_timeout", "cost_usd": None, "checks": [], "turns": 1, "seconds": 1}
    res = {"waves": 1, "evals": [{"id": "t0", "max_budget_usd": 12.0, "results": [{"run": 1, "arms": {"none": side}}]}]}
    got = runs_of(res)[0]
    expect("no cost", f"{got['cost']:.0f} {got['p2']}", "12 0.0")
    # One leaked run does not put the verdict below the floor.
    runs = synth({"lean": [0.9] * 5, "none": [0.5] * 5}, {"lean": 3.0, "none": 1.0})
    runs = [r for r in runs if not (r["task"] == "t0" and r["run"] == 1 and r["arm"] == "lean")]
    expect("leaked run dropped", compare(runs, "lean", "none", floor_ok=True)["verdict"], "better")

    # The counts of a transcript: the turns, and 2 leak columns that differ.
    def side(turns: int) -> dict:
        return {"subtype": "success", "cost_usd": 1.0, "checks": [{"name": "p2-01", "passed": True}], "turns": turns, "seconds": 60.0}

    res = {
        "waves": 2,
        "evals": [
            {"id": "t0", "max_budget_usd": 25, "results": [
                {"run": 1, "arms": {"none": side(2), "lean": side(5)}},
                {"run": 2, "arms": {"none": side(7), "lean": side(5)}},
            ]}
        ],
    }
    # The log is out of order (2 processes wrote it). The epoch gives the order.
    counts = {"runs": [
        {"task": "t0", "arm": "none", "epoch": 20, "turns": 90, "hidden_touches": 0, "strict_touches": 0},
        {"task": "t0", "arm": "none", "epoch": 10, "turns": 152, "hidden_touches": 3, "strict_touches": 0},
        {"task": "t0", "arm": "lean", "epoch": 10, "turns": 40, "hidden_touches": 0, "strict_touches": 1},
        {"task": "t0", "arm": "lean", "epoch": 20, "turns": 41, "hidden_touches": 0, "strict_touches": 0},
    ]}
    out = analyze(res, counts)
    by = {(r["arm"], r["run"]): r for r in out["runs"]}
    expect("turns from the transcript", f"{by[('none', 1)]['turns']} {by[('none', 2)]['turns']}", "152 90")
    expect("the recorded turns stay", by[("none", 1)]["turns_recorded"], 2)
    expect("leaked by the ADR pattern", out["leaked_runs_adr"], ["t0#1:none"])
    expect("leaked by the strict column", out["leaked_runs_strict"], ["t0#1:lean"])
    expect("a variant for each column", sorted(out["variants"]), ["all", "without-adr-leaked", "without-strict-leaked"])
    # A count that does not fit the runs (an excluded attempt) marks each run of the task and the arm.
    counts["runs"].append({"task": "t0", "arm": "none", "epoch": 30, "turns": 5, "hidden_touches": 0, "strict_touches": 0})
    out = analyze(res, counts)
    expect("no fit marks each run", out["leaked_runs_adr"], ["t0#1:none", "t0#2:none"])
    # A wave that is not complete has lines in the log. They match their own runs and do not spill.
    res3 = {"waves": 1, "evals": [{"id": "t0", "max_budget_usd": 25, "results": [
        {"run": 1, "arms": {"none": side(2)}}, {"run": 2, "arms": {"none": side(2)}}]}]}
    counts3 = {"runs": [
        {"task": "t0", "arm": "none", "epoch": 1, "turns": 152, "strict_touches": 0},
        {"task": "t0", "arm": "none", "epoch": 2, "turns": 9, "strict_touches": 2},
    ]}
    out = analyze(res3, counts3)
    kept = [r for r in out["runs"] if r["run"] <= 1]
    expect("a partial wave does not spill", (out["leaked_runs_strict"], kept[0]["turns"]), (["t0#2:none"], 152))
    # An excluded attempt has a line before the line that counts.
    res4 = {"waves": 1, "evals": [{"id": "t0", "max_budget_usd": 25, "results": [
        {"run": 1, "arms": {"none": {**side(2), "excluded": 1}}}]}]}
    counts4 = {"runs": [
        {"task": "t0", "arm": "none", "epoch": 1, "turns": 3, "strict_touches": 1},
        {"task": "t0", "arm": "none", "epoch": 2, "turns": 77, "strict_touches": 0},
    ]}
    out = analyze(res4, counts4)
    expect("an excluded attempt is skipped", (out["leaked_runs_strict"], out["runs"][0]["turns"]), ([], 77))
    return 0 if ok else 1


def main() -> int:
    args = sys.argv[1:]
    if not args:
        sys.exit(__doc__)
    if args == ["--selftest"]:
        return selftest()
    result = json.loads(Path(args[0]).read_text())
    counts = None
    if "--counts" in args:
        # Several files, with commas: the counts of the batch and of the probes.
        runs = []
        for name in args[args.index("--counts") + 1].split(","):
            runs += json.loads(Path(name).read_text())["runs"]
        counts = {"runs": runs}
    out = analyze(result, counts)
    if "--json" in args:
        print(json.dumps(out, indent=2))
        return 0
    print(
        f"waves: {out['waves']}  leaked runs (ADR pattern): {len(out['leaked_runs_adr'])}"
        f"  leaked runs (strict): {len(out['leaked_runs_strict'])}"
    )
    for name, v in out["variants"].items():
        print(f"== {name}")
        for arm, a in v["arms"].items():
            print(arm, {k: round(x, 3) if isinstance(x, float) else x for k, x in a.items()})
        for c in v["comparisons"]:
            print(
                f"{c['treatment']} vs {c['control']} ({c['role']}): {c['verdict']}"
                + (
                    f", difference {c['difference']:+.3f} {c['difference_interval'][0]:+.3f}..{c['difference_interval'][1]:+.3f},"
                    f" cost ratio {c['cost_ratio']:.2f} {c['cost_ratio_interval'][0]:.2f}..{c['cost_ratio_interval'][1]:.2f}"
                    if "difference" in c
                    else ""
                )
            )
    return 0


if __name__ == "__main__":
    sys.exit(main())
