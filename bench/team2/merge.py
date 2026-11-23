"""Merge the result files of the jobs of the batch into one result of `eval run --arms`.

Usage: merge.py <out file> --tasks <id>,<id>,... <wave>=<result file> [<wave>=<result file> ...]

The batch of protocol 2 runs one job for each task and wave: `spec-dna eval run
--arms ... <task> --runs 1`. Each job writes its own result file, so no 2 processes
write one file. This command puts the evals of these files into one file with the
form of `eval run --arms`, so `analyze.py` reads it. `<wave>` is the number of the
wave of the file. The run of each eval of the file gets that number.

The key `waves` is the number of complete waves, counted from wave 1 on. A wave is
complete when each task of `--tasks` has all arms with a side that ran. `spend_usd`
is the sum of the spend of the files, and `excluded` is the sum for each arm.
The command stops with an error when a task and a wave come twice.
"""

import json
import sys
from pathlib import Path


def merge(parts: list[tuple[int, dict]], tasks: list[str]) -> dict:
    if not parts:
        raise ValueError("no result file")
    first = parts[0][1]
    evals: dict[str, dict] = {}
    seen: set[tuple[str, int]] = set()
    spend = 0.0
    excluded: dict[str, int] = {}
    stopped = []
    models: set[str] = set()
    starts = []
    for wave, result in parts:
        spend += result.get("spend_usd") or 0.0
        for arm, n in (result.get("excluded") or {}).items():
            excluded[arm] = excluded.get(arm, 0) + n
        if result.get("stopped"):
            stopped.append(result["stopped"])
        models.update(result.get("models") or [])
        starts.append(result.get("started") or "")
        for item in result["evals"]:
            target = evals.setdefault(
                item["id"], {"id": item["id"], "runs": 0, "max_budget_usd": item.get("max_budget_usd"), "results": []}
            )
            for res in item["results"]:
                if (item["id"], wave) in seen:
                    raise ValueError(f"the task {item['id']} comes twice in wave {wave}")
                seen.add((item["id"], wave))
                target["results"].append({"run": wave, "arms": res["arms"]})
    for item in evals.values():
        item["results"].sort(key=lambda r: r["run"])
        item["runs"] = len(item["results"])
    arms = first["arms"]
    top = max((w for w, _ in parts), default=0)
    waves = 0
    for wave in range(1, top + 1):
        complete = True
        for task in tasks:
            res = next((r for r in evals.get(task, {"results": []})["results"] if r["run"] == wave), None)
            if res is None or any(res["arms"].get(a) is None for a in arms):
                complete = False
        if not complete:
            break
        waves = wave
    return {
        "spec_dna": first.get("spec_dna"),
        "claude_code": first.get("claude_code"),
        "models": sorted(models),
        "started": min(s for s in starts if s) if any(starts) else None,
        "arms": arms,
        "runs": top,
        "seed": first.get("seed"),
        "resamples": first.get("resamples"),
        "max_total_usd": None,
        "estimate_usd": None,
        "spend_usd": round(spend, 6),
        "excluded": excluded,
        "stopped": stopped[0] if stopped else None,
        "waves": waves,
        "evals": [evals[k] for k in sorted(evals)],
    }


def main() -> int:
    args = sys.argv[1:]
    if len(args) < 4 or "--tasks" not in args:
        sys.exit(__doc__)
    out = Path(args[0])
    tasks = args[args.index("--tasks") + 1].split(",")
    pairs = [a for a in args[1:] if "=" in a]
    parts = []
    for pair in pairs:
        wave, name = pair.split("=", 1)
        parts.append((int(wave), json.loads(Path(name).read_text())))
    merged = merge(parts, tasks)
    out.write_text(json.dumps(merged, indent=2) + "\n")
    print(f"{out}: {len(parts)} files, {merged['waves']} complete waves, spend {merged['spend_usd']:.2f} USD")
    return 0


def selftest() -> int:
    ok = True

    def expect(name: str, got: object, want: object) -> None:
        nonlocal ok
        good = got == want
        print(f"{'ok  ' if good else 'FAIL'} {name}: {got}")
        ok = ok and good

    def part(task: str, arms: list, spend: float, none_arm: str | None = None) -> dict:
        sides = {a: (None if a == none_arm else {"subtype": "success", "checks": []}) for a in arms}
        return {
            "arms": arms,
            "spend_usd": spend,
            "excluded": {a: 0 for a in arms},
            "stopped": None,
            "models": ["m"],
            "started": "2026-11-24T00:00:00Z",
            "evals": [{"id": task, "runs": 1, "max_budget_usd": 25, "results": [{"run": 1, "arms": sides}]}],
        }

    arms = ["a", "b", "c"]
    parts = [(1, part("t1", arms, 10)), (1, part("t2", arms, 20)), (2, part("t1", arms, 5)), (2, part("t2", arms, 1, "c"))]
    merged = merge(parts, ["t1", "t2"])
    expect("complete waves", merged["waves"], 1)
    expect("spend", merged["spend_usd"], 36)
    expect("runs of an eval", [(e["id"], [r["run"] for r in e["results"]]) for e in merged["evals"]], [("t1", [1, 2]), ("t2", [1, 2])])
    expect("a task is missing", merge(parts[:1], ["t1", "t2"])["waves"], 0)
    expect("all waves", merge(parts[:3] + [(2, part("t2", arms, 1))], ["t1", "t2"])["waves"], 2)
    try:
        merge([(1, part("t1", arms, 1)), (1, part("t1", arms, 1))], ["t1"])
        expect("a repeat is an error", "no error", "error")
    except ValueError:
        expect("a repeat is an error", "error", "error")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(selftest() if sys.argv[1:] == ["--selftest"] else main())
