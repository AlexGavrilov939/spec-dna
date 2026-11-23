"""Make the batch project of the model-team measurement.

Usage: make_project.py <dest> [<id>...]

The folder <dest> must not exist or must be empty. The command writes a git
repository that holds spec-dna.toml, one eval file for each task, the fixtures,
the hidden tests (evals/oracle/<id>/) and the check functions. The runner reads
the project from <dest>. The agent of a run never gets a path of <dest>.

The protocol is ADR 0029. The tasks are in bench/team/tasks/<id>/.
"""

import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
TASKS = HERE / "tasks"
MAX_TURNS = 40
MAX_BUDGET_USD = 4
# One list for each arm. `Bash` is not narrowed, so no arm loses turns on a denied
# command. `Agent` lets the roles of `lean` start a subagent.
ALLOWED_TOOLS = ["Read", "Write", "Edit", "Glob", "Grep", "Agent", "Bash"]


def binary_version() -> str:
    """The version of the binary that runs the batch (variable SPEC_DNA_BIN)."""
    out = subprocess.run(
        [os.environ.get("SPEC_DNA_BIN", "spec-dna"), "--version"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    return out.split()[-1]


def task_ids() -> list[str]:
    return sorted(p.name for p in TASKS.iterdir() if (p / "task.txt").is_file())


def eval_file(task_id: str) -> dict:
    task = (TASKS / task_id / "task.txt").read_text(encoding="utf-8").strip()
    return {
        "id": task_id,
        "task": task,
        "fixture": f"evals/fixtures/{task_id}",
        "allowed-tools": ALLOWED_TOOLS,
        "max-turns": MAX_TURNS,
        "max-budget-usd": MAX_BUDGET_USD,
        "checks": [
            {"name": "tests-unchanged", "run": "tests_unchanged"},
            {"name": "visible", "run": "visible"},
            {"name": "hidden", "run": f"hidden {task_id}"},
        ],
    }


def write_project(dest: Path, ids: list[str]) -> None:
    if dest.exists() and any(dest.iterdir()):
        sys.exit(f"make_project: {dest} is not empty")
    evals = dest / "evals"
    evals.mkdir(parents=True, exist_ok=True)
    (dest / "spec-dna.toml").write_text(
        f'spec-dna = "{binary_version()}"\n\n'
        '[[roots]]\npath = "."\n\n'
        "[evals]\n"
        'dir = "evals"\n'
        "min-pass-rate = 0.8\n"
        "runs = 5\n"
        f"max-total-usd = {MAX_BUDGET_USD * 5}\n",
        encoding="utf-8",
    )
    shutil.copy(HERE / "check.sh", evals / "check.sh")
    shutil.copy(HERE / "guard.py", evals / "guard.py")
    for task_id in ids:
        src = TASKS / task_id
        (evals / f"{task_id}.json").write_text(
            json.dumps(eval_file(task_id), indent=2) + "\n", encoding="utf-8"
        )
        shutil.copytree(src / "fixture", evals / "fixtures" / task_id)
        shutil.copytree(src / "oracle", evals / "oracle" / task_id)
    env = {
        **os.environ,
        "GIT_AUTHOR_NAME": "team",
        "GIT_AUTHOR_EMAIL": "team@example.invalid",
        "GIT_COMMITTER_NAME": "team",
        "GIT_COMMITTER_EMAIL": "team@example.invalid",
        "GIT_CONFIG_GLOBAL": "/dev/null",
        "GIT_CONFIG_NOSYSTEM": "1",
    }
    for args in (
        ["init", "-q", "-b", "main"],
        ["add", "-A"],
        ["-c", "commit.gpgsign=false", "commit", "-q", "-m", "batch project"],
    ):
        subprocess.run(["git", *args], cwd=dest, env=env, check=True)


def main() -> None:
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    dest = Path(sys.argv[1]).resolve()
    ids = sys.argv[2:] or task_ids()
    for task_id in ids:
        if not (TASKS / task_id / "task.txt").is_file():
            sys.exit(f"make_project: unknown task {task_id}")
    write_project(dest, ids)
    print(f"{dest}: {len(ids)} tasks")


if __name__ == "__main__":
    main()
