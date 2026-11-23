"""The self-test of the tasks of protocol 2. It runs with no model and costs nothing.

Usage: selftest.py [--write] [<id>...]

The variable SPEC_DNA_BIN names the binary (default: `spec-dna` on PATH). For
each task, the test makes the batch project with all candidate tests as checks,
then runs the real runner `spec-dna eval run` with the stub `claude` of
stub_claude.py in 3 modes:

- none: the start state. A test of part 1 must fail.
- wrong: the solution with the known defects. A test of part 1 must pass, and a
    test of part 2 must fail.

A test of part 2 must also fail on the start tree, so a run that changes nothing
scores 0 in both parts.
- reference: the reference solution. Each test that counts must pass.

The tests that fit these rules are the tests that count. With `--write`, the
command writes them to `tasks/<id>/parts.json` (and the others, with the reason,
to the key `dropped`). Without it, the command only reports. The exit code is 1
when a task has no test in part 1 or none in part 2.

The runs share one build folder for each task (TEAM2_TARGET_DIR), so only the
first build is slow.
"""

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
TASKS = HERE / "tasks"
MODES = ("none", "wrong", "reference")


def binary() -> str:
    return os.environ.get("SPEC_DNA_BIN", "spec-dna")


def candidate_list(task_id: str) -> tuple[list, list]:
    c = json.loads((TASKS / task_id / "candidates.json").read_text())
    flat = lambda d: [[f, t] for f, names in sorted(d.items()) for t in names]  # noqa: E731
    return flat(c["part1"]), flat(c["part2"])


def run_mode(project: Path, shim: Path, task_id: str, mode: str, target: Path) -> dict:
    env = {
        **os.environ,
        "PATH": f"{shim}{os.pathsep}{os.environ['PATH']}",
        "STUB_MODE": mode,
        "STUB_TASKS": str(TASKS),
        "TEAM2_TARGET_DIR": str(target),
        "SPEC_DNA_EVAL_CHECK_TIMEOUT": "2400",
    }
    for name in ("CLAUDECODE", "CLAUDE_CODE_PLUGIN_DIRS"):
        env.pop(name, None)
    done = subprocess.run(
        [binary(), "eval", "run", task_id, "--runs", "1", "--harness", "none", "--json"],
        cwd=project,
        env=env,
        capture_output=True,
        text=True,
    )
    if not done.stdout.strip():
        sys.exit(f"selftest: no result for {task_id} {mode}: {done.stderr.strip()}")
    run = json.loads(done.stdout)["evals"][0]["results"][0]
    return {c["name"]: c["passed"] for c in run["checks"]}


def main() -> int:
    args = sys.argv[1:]
    write = "--write" in args
    ids = [a for a in args if not a.startswith("--")] or sorted(
        p.name for p in TASKS.iterdir() if (p / "task.json").is_file()
    )
    bad = 0
    with tempfile.TemporaryDirectory(prefix="team2-selftest-") as tmp:
        shim = Path(tmp) / "bin"
        shim.mkdir()
        claude = shim / "claude"
        claude.write_text(f'#!/bin/sh\nexec python3 "{HERE / "stub_claude.py"}" "$@"\n')
        claude.chmod(0o755)
        for task_id in ids:
            project = Path(tmp) / f"project-{task_id}"
            subprocess.run(
                [sys.executable, str(HERE / "make_project.py"), str(project), "--candidates", task_id],
                check=True,
                capture_output=True,
            )
            target = Path(tempfile.gettempdir()) / f"team2-target-{task_id}"
            part1, part2 = candidate_list(task_id)
            names1 = [f"p1-{n:02d}" for n in range(1, len(part1) + 1)]
            names2 = [f"p2-{n:02d}" for n in range(1, len(part2) + 1)]
            seen = {mode: run_mode(project, shim, task_id, mode, target) for mode in MODES}
            keep1, keep2, dropped = [], [], {}
            for name, test in zip(names1, part1):
                none, wrong, ref = (seen[m][name] for m in MODES)
                if (not none) and wrong and ref:
                    keep1.append(test)
                else:
                    dropped[f"{test[0]}::{test[1]}"] = f"part 1: none={none} wrong={wrong} reference={ref}"
            for name, test in zip(names2, part2):
                none, wrong, ref = (seen[m][name] for m in MODES)
                if (not none) is True and (not wrong) and ref:
                    keep2.append(test)
                else:
                    dropped[f"{test[0]}::{test[1]}"] = f"part 2: none={none} wrong={wrong} reference={ref}"
            ok = bool(keep1) and bool(keep2)
            print(
                f"{'ok  ' if ok else 'FAIL'} {task_id}: part 1 {len(keep1)} of {len(part1)}, "
                f"part 2 {len(keep2)} of {len(part2)}"
            )
            for key, reason in dropped.items():
                print(f"     dropped {key}: {reason}")
            bad += 0 if ok else 1
            if write:
                (TASKS / task_id / "parts.json").write_text(
                    json.dumps({"part1": keep1, "part2": keep2, "dropped": dropped}, indent=2) + "\n"
                )
    print("selftest:", "passed" if bad == 0 else f"{bad} failed")
    return 0 if bad == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
