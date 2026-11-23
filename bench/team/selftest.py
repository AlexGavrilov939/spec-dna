"""The self-test of the team tasks. It runs with no model and costs nothing.

Usage: selftest.py [<id>...]

The variable SPEC_DNA_BIN names the binary (default: `spec-dna` on PATH). For
each task, the test makes the batch project, then runs the real runner
`spec-dna eval run` with the stub `claude` of stub_claude.py in 6 modes:

- none: the start state. The hidden tests must fail.
- reference: the reference solution. Each check must pass.
- alt: a second, different correct solution. Each check must pass.
- wrong: a solution that is wrong. It must pass tests-unchanged and visible,
and it must fail hidden.
- tamper: a stub that edits a visible test. The check tests-unchanged must fail.
- hunt: a stub that looks for the hidden tests in the copy, in PATH and in the
environment. It must find none.

The exit code is 0 when each task gives the expected result, else 1.
"""

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
TASKS = HERE / "tasks"
TRUE, FALSE = True, False

# mode -> expected value of each check: tests-unchanged, visible, hidden.
# None means that the test does not look at the check.
EXPECTED = {
    "none": (TRUE, None, FALSE),
    "reference": (TRUE, TRUE, TRUE),
    "alt": (TRUE, TRUE, TRUE),
    "wrong": (TRUE, TRUE, FALSE),
    "hunt": (TRUE, None, FALSE),
    "tamper": (FALSE, None, None),
}


def binary() -> str:
    return os.environ.get("SPEC_DNA_BIN", "spec-dna")


def run_mode(project: Path, shim: Path, task_id: str, mode: str) -> dict:
    env = {
        **os.environ,
        "PATH": f"{shim}{os.pathsep}{os.environ['PATH']}",
        "STUB_MODE": mode,
        "STUB_TASKS": str(TASKS),
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
    return json.loads(done.stdout)


def check_values(result: dict) -> tuple[dict, int]:
    run = result["evals"][0]["results"][0]
    return {c["name"]: c["passed"] for c in run["checks"]}, run["claude_exit"]


def main() -> int:
    ids = sys.argv[1:] or sorted(p.name for p in TASKS.iterdir() if (p / "task.txt").is_file())
    bad = 0
    with tempfile.TemporaryDirectory(prefix="team-selftest-") as tmp:
        shim = Path(tmp) / "bin"
        shim.mkdir()
        claude = shim / "claude"
        claude.write_text(f'#!/bin/sh\nexec python3 "{HERE / "stub_claude.py"}" "$@"\n')
        claude.chmod(0o755)
        project = Path(tmp) / "project"
        subprocess.run(
            [sys.executable, str(HERE / "make_project.py"), str(project), *ids],
            check=True,
            capture_output=True,
        )
        for task_id in ids:
            for mode, expected in EXPECTED.items():
                values, claude_exit = check_values(run_mode(project, shim, task_id, mode))
                got = (
                    values["tests-unchanged"],
                    values["visible"],
                    values["hidden"],
                )
                ok = all(e is None or e == g for e, g in zip(expected, got))
                if mode == "hunt":
                    ok = ok and claude_exit == 0
                print(f"{'ok  ' if ok else 'FAIL'} {task_id} {mode:9} {got}")
                bad += 0 if ok else 1
    print("selftest:", "passed" if bad == 0 else f"{bad} failed")
    return 0 if bad == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
