"""Scores and verdicts of protocol 2 (ADR 0033).

Usage: analyze.py <arms result file> [--counts <transcript_counts json>] [--drop-leaked] [--json]
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


def analyze(result: dict, counts: dict | None, drop_leaked: bool) -> dict:
    runs = runs_of(result)
    waves_all = result.get("waves", 0)
    floor_ok = waves_all >= WAVES_FLOOR and len({r["task"] for r in runs}) >= TASKS_FLOOR
    leaked: set[tuple[str, int, str]] = set()
    if counts:
        order = collections.defaultdict(list)
        for c in counts["runs"]:
            order[(c["task"], c["arm"])].append(c)
        for (task, arm), lines in order.items():
            n_runs = len([r for r in runs if r["task"] == task and r["arm"] == arm])
            hit = [n for n, c in enumerate(lines, 1) if c.get("hidden_touches")]
            if not hit:
                continue
            if len(lines) == n_runs:
                leaked.update((task, n, arm) for n in hit)
            else:
                # The log and the result differ (an excluded attempt): one leaked
                # line marks each run of the task and the arm.
                leaked.update((task, r["run"], arm) for r in runs if r["task"] == task and r["arm"] == arm)
        for r in runs:
            r["leaked"] = (r["task"], r["run"], r["arm"]) in leaked
        advisor = collections.defaultdict(lambda: [0, 0])
        for c in counts["runs"]:
            advisor[c["arm"]][0] += c.get("advisor_calls", 0)
            advisor[c["arm"]][1] += 1
    if drop_leaked:
        runs = [r for r in runs if not r.get("leaked")]
    arms = arm_table(runs)
    if counts:
        for arm, (calls, n) in advisor.items():
            if arm in arms:
                arms[arm]["advisor_calls_per_run"] = calls / n
    names = set(arms)
    comparisons = []
    for treatment, control in (("lean", "lean-no-advisor"), ("lean", "none"), ("lean-no-advisor", "none")):
        if treatment in names and control in names:
            c = compare(runs, treatment, control, floor_ok=floor_ok)
            c["role"] = "verdict" if control != "none" or treatment == "lean" else "description"
            comparisons.append(c)
    return {
        "waves": result.get("waves", 0),
        "leaked_runs": sorted(f"{t}#{n}:{a}" for t, n, a in leaked),
        "arms": arms,
        "comparisons": comparisons,
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
    return 0 if ok else 1


def main() -> int:
    args = sys.argv[1:]
    if not args:
        sys.exit(__doc__)
    if args == ["--selftest"]:
        return selftest()
    result = json.loads(Path(args[0]).read_text())
    counts = json.loads(Path(args[args.index("--counts") + 1]).read_text()) if "--counts" in args else None
    out = analyze(result, counts, "--drop-leaked" in args)
    if "--json" in args:
        print(json.dumps(out, indent=2))
        return 0
    print(f"waves: {out['waves']}  leaked runs: {len(out['leaked_runs'])}")
    for arm, a in out["arms"].items():
        print(arm, {k: round(v, 3) if isinstance(v, float) else v for k, v in a.items()})
    for c in out["comparisons"]:
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
