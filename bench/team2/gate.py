"""The gate and the limits after the probes of protocol 2 (ADR 0033).

Usage: gate.py <probe result file> [--spent <usd>] [--write]

The probe result file is the `arms-<stamp>.json` of the 3 probes (1 run of each
arm on the probe task). The command applies the rules of the ADR:

- The gate stops the work when, on the probe task, the score of part 2 is 1 in
    each arm (the ceiling), or 0 in each arm (the floor), or the score of part 1 is
    0 in each arm, or a probe hit a limit, or the projected cost is above the room.
- The projection is 15 x (a + b + c) x s. `a`, `b` and `c` are the costs of the 3
    probes. `s` is the mean of the sizes of the reference patches of the 5 tasks (in
    changed lines) divided by the size of the patch of the probe task.
- The room is the cap, minus the spend of the probes, minus 10 USD of reserve. The
    cap is 700 USD (the amendment of 2026-11-23 to ADR 0033; it was 380 USD).
- The limits of the batch: `max-turns` is 2 times the largest number of turns of
    a probe, rounded up to a multiple of 10, from 80 to 300. `max-budget-usd` is 2
    times the largest cost of a probe, rounded up to a whole number, from 10 to 25.

With `--write` and an open gate, the command writes `limits.json`. The exit code
is 0 when the gate is open, and 1 when it stops the work.
"""

import json
import math
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from analyze import runs_of  # noqa: E402

# The cap of the ADR was 380 USD. The amendment of 2026-11-23 sets 700 USD in total,
# with the spend of the probes (40.71 USD) inside it. No other rule changed.
CAP = 700.0
RESERVE = 10.0
PROBE_TASK = "t2-05-approver-hook"
RUNS = 15


def changed_lines(patch: Path) -> int:
    n = 0
    for line in patch.read_text(errors="replace").splitlines():
        if line.startswith(("+", "-")) and not line.startswith(("+++", "---")):
            n += 1
    return n


def main() -> int:
    args = sys.argv[1:]
    if not args:
        sys.exit(__doc__)
    result = json.loads(Path(args[0]).read_text())
    spent = float(args[args.index("--spent") + 1]) if "--spent" in args else result.get("spend_usd", 0.0)
    runs = [r for r in runs_of({**result, "waves": 1}) if r["task"] == PROBE_TASK]
    arms = {r["arm"]: r for r in runs}
    out: dict = {"probe_task": PROBE_TASK, "spent": spent, "reasons": []}
    if set(arms) != {"none", "lean-no-advisor", "lean"}:
        out["reasons"].append("the probes do not hold the 3 arms")
        print(json.dumps(out, indent=2))
        return 1
    p2 = {a: r["p2"] for a, r in arms.items()}
    p1 = {a: r["p1"] for a, r in arms.items()}
    if all(v == 1 for v in p2.values()):
        out["reasons"].append("ceiling: part 2 is 1 in each arm")
    if all(v == 0 for v in p2.values()):
        out["reasons"].append("floor: part 2 is 0 in each arm")
    if all(v == 0 for v in p1.values()):
        out["reasons"].append("floor: part 1 is 0 in each arm")
    for a, r in arms.items():
        if r["limit"]:
            out["reasons"].append(f"the probe of {a} hit a limit")
    sizes = {
        p.parent.name: changed_lines(p) for p in sorted((HERE / "tasks").glob("*/reference.patch"))
    }
    s = (sum(sizes.values()) / len(sizes)) / sizes[PROBE_TASK]
    cost = sum(r["cost"] for r in arms.values())
    projected = RUNS * cost * s
    room = CAP - spent - RESERVE
    turns = max(r["turns"] for r in arms.values())
    top = max(r["cost"] for r in arms.values())
    limits = {
        "max-turns": min(300, max(80, math.ceil(2 * turns / 10) * 10)),
        "max-budget-usd": min(25, max(10, math.ceil(2 * top))),
    }
    if projected + limits["max-budget-usd"] > room:
        out["reasons"].append(
            f"the projected cost {projected:.0f} USD plus the budget of a run is above the room {room:.0f} USD"
        )
    # The model check of ADR 0029: the arms without the advisor list no advisor model,
    # and each probe lists the main model.
    sides = {}
    for item in result["evals"]:
        if item["id"] == PROBE_TASK:
            for res in item["results"]:
                sides.update({a: s for a, s in res["arms"].items() if s})
    for a, side in sides.items():
        models = side.get("models", [])
        if a != "lean" and any("fable" in m for m in models):
            out["reasons"].append(f"the probe of {a} lists the advisor model")
        if not any("opus" in m for m in models):
            out["reasons"].append(f"the probe of {a} does not list the main model")
    out.update(
        {
            "part1": p1,
            "part2": p2,
            "probe_cost": {a: r["cost"] for a, r in arms.items()},
            "probe_turns": {a: r["turns"] for a, r in arms.items()},
            "sizes": sizes,
            "scale": s,
            "projected": projected,
            "room": room,
            "limits": limits,
            "open": not out["reasons"],
        }
    )
    print(json.dumps(out, indent=2))
    if out["open"] and "--write" in args:
        (HERE / "limits.json").write_text(json.dumps(limits, indent=2) + "\n")
    return 0 if out["open"] else 1


if __name__ == "__main__":
    sys.exit(main())
