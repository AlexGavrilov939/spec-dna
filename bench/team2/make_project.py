"""Make the batch project of protocol 2 of the model-team measurement (ADR 0033).

Usage: make_project.py <dest> [--candidates] [--limits '<json>'] [<id>...]

The folder <dest> must not exist or must be empty. The command writes a git
repository that holds spec-dna.toml, one eval file for each task, the fixtures,
the hidden tests (evals/oracle/<id>/) and the check functions. The runner reads
the project from <dest>. The agent of a run never gets a path of <dest>.

A fixture is the tree of the start commit of the task, from `git archive`, with
the files of the agent team of this repository taken out. It has no history.
The lists of the tests that count are `tasks/<id>/parts.json`. With
`--candidates`, the self-test uses the list of all candidates instead.

The limits of a run are in `limits.json` (the formula of ADR 0033), or the
defaults of the dry run. `--limits '{"max-turns": 300, "max-budget-usd": 25}'`
sets them for the probes.
"""

import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from prepare import extract  # noqa: E402

TASKS = HERE / "tasks"
STRIP = [".claude", "CLAUDE.md", "AGENTS.md", "REVIEW.md", ".spec-dna", ".github"]
DEFAULT_LIMITS = {"max-turns": 150, "max-budget-usd": 20}
ALLOWED_TOOLS = ["Read", "Write", "Edit", "Glob", "Grep", "Agent", "Bash"]


def binary_version() -> str:
    out = subprocess.run(
        [os.environ.get("SPEC_DNA_BIN", "spec-dna"), "--version"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    return out.split()[-1]


def task_ids() -> list[str]:
    return sorted(p.name for p in TASKS.iterdir() if (p / "task.json").is_file())


def prompt_of(task_id: str) -> str:
    meta = json.loads((TASKS / task_id / "task.json").read_text())
    return (
        f"Implement the change `{meta['slug']}` of this repository. Its intent, spec and plan "
        f"are in `docs/intent/{meta['slug']}/`. {meta['scope']} Write the tests first, as the "
        "plan says, then the code, and run the `Verify` commands of your tasks. The docs, the "
        "changelog and the CI files are out of scope. Do not make a commit."
    )


def tests_of(task_id: str, candidates: bool) -> tuple[list, list]:
    folder = TASKS / task_id
    if candidates or not (folder / "parts.json").is_file():
        c = json.loads((folder / "candidates.json").read_text())
        flat = lambda d: [[f, t] for f, names in sorted(d.items()) for t in names]  # noqa: E731
        return flat(c["part1"]), flat(c["part2"])
    p = json.loads((folder / "parts.json").read_text())
    return p["part1"], p["part2"]


def eval_file(task_id: str, limits: dict, candidates: bool) -> dict:
    part1, part2 = tests_of(task_id, candidates)
    checks = []
    for part, tests in (("p1", part1), ("p2", part2)):
        for n, (file, name) in enumerate(tests, 1):
            stem = Path(file).stem
            checks.append(
                {"name": f"{part}-{n:02d}", "run": f"hidden_test {task_id} {stem} {name}"}
            )
    return {
        "id": task_id,
        "task": prompt_of(task_id),
        "fixture": f"evals/fixtures/{task_id}",
        "allowed-tools": ALLOWED_TOOLS,
        "max-turns": limits["max-turns"],
        "max-budget-usd": limits["max-budget-usd"],
        "checks": checks,
    }


def write_project(dest: Path, ids: list[str], candidates: bool, given: dict | None = None) -> None:
    if dest.exists() and any(dest.iterdir()):
        sys.exit(f"make_project: {dest} is not empty")
    limits_file = HERE / "limits.json"
    limits = given or (json.loads(limits_file.read_text()) if limits_file.is_file() else DEFAULT_LIMITS)
    evals = dest / "evals"
    evals.mkdir(parents=True, exist_ok=True)
    (dest / "spec-dna.toml").write_text(
        f'spec-dna = "{binary_version()}"\n\n'
        '[[roots]]\npath = "."\n\n'
        "[evals]\n"
        'dir = "evals"\n'
        "min-pass-rate = 0.8\n"
        "runs = 3\n"
        f"max-total-usd = {limits['max-budget-usd'] * 3}\n",
        encoding="utf-8",
    )
    shutil.copy(HERE / "check.sh", evals / "check.sh")
    shutil.copy(HERE.parent / "team" / "guard.py", evals / "guard.py")
    for task_id in ids:
        meta = json.loads((TASKS / task_id / "task.json").read_text())
        (evals / f"{task_id}.json").write_text(
            json.dumps(eval_file(task_id, limits, candidates), indent=2) + "\n", encoding="utf-8"
        )
        fixture = evals / "fixtures" / task_id
        extract(meta["start"], fixture)
        for name in STRIP:
            target = fixture / name
            if target.is_dir():
                shutil.rmtree(target)
            elif target.exists():
                target.unlink()
        shutil.copytree(TASKS / task_id / "oracle", evals / "oracle" / task_id)
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
    args = sys.argv[1:]
    if not args:
        sys.exit(__doc__)
    candidates = "--candidates" in args
    given = None
    if "--limits" in args:
        i = args.index("--limits")
        given = json.loads(args[i + 1])
        del args[i : i + 2]
    args = [a for a in args if a != "--candidates"]
    dest = Path(args[0]).resolve()
    ids = args[1:] or task_ids()
    for task_id in ids:
        if not (TASKS / task_id / "task.json").is_file():
            sys.exit(f"make_project: unknown task {task_id}")
    write_project(dest, ids, candidates, given)
    print(f"{dest}: {len(ids)} tasks")


if __name__ == "__main__":
    main()
