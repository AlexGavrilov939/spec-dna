"""A stub of `claude` for the self-test of the tasks of protocol 2. It costs nothing.

The runner of `spec-dna eval run` starts this file in place of `claude`. The
stub finds the task from the text of the prompt, then it acts by the mode in
the variable STUB_MODE:

none: it changes nothing.

reference, wrong: it applies tasks/<id>/<mode>.patch to the copy (the working
folder).

hunt: it looks for the hidden tests and for the later history. It searches the
copy and each folder of PATH for a name that starts with "oracle", and it looks
at the environment for the name of check.sh. If it finds one, it exits with 1.

facts: it writes the harness files of the copy into the folder STUB_OUT, one
folder for each call, so a person can read what each arm gives the agent.

The variable STUB_TASKS holds the folder of the tasks.
"""

import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from make_project import prompt_of  # noqa: E402


def find_task(prompt: str, tasks: Path) -> str:
    for folder in sorted(tasks.iterdir()):
        if (folder / "task.json").is_file() and prompt_of(folder.name) == prompt:
            return folder.name
    sys.exit("stub: no task has this prompt")


def hunt() -> list[str]:
    found = []
    roots = [Path.cwd(), *(Path(p) for p in os.environ.get("PATH", "").split(os.pathsep) if p)]
    for root in roots:
        if not root.is_dir():
            continue
        for _, dirs, files in os.walk(root):
            found += [n for n in dirs + files if n.lower().startswith("oracle")]
    found += [k for k, v in os.environ.items() if "check.sh" in v or "/oracle" in v.lower()]
    return found


def facts(out: Path) -> None:
    n = len(list(out.iterdir())) if out.is_dir() else 0
    dest = out / f"call-{n + 1}"
    dest.mkdir(parents=True)
    cwd = Path.cwd()
    (dest / "listing.txt").write_text("\n".join(sorted(p.name for p in cwd.iterdir())) + "\n")
    for name in ("spec-dna.toml", "CLAUDE.md", "AGENTS.md", "REVIEW.md"):
        if (cwd / name).is_file():
            shutil.copy(cwd / name, dest / name)
    if (cwd / ".claude").is_dir():
        shutil.copytree(cwd / ".claude", dest / "claude-dir")
    (dest / "args.json").write_text(json.dumps(sys.argv[1:]))
    (dest / "git-log.txt").write_text(
        subprocess.run(["git", "log", "--oneline"], capture_output=True, text=True).stdout
    )


def main() -> int:
    args = sys.argv[1:]
    if args == ["--version"]:
        print("2.1.330 (Claude Code)")
        return 0
    prompt = args[args.index("-p") + 1]
    tasks = Path(os.environ["STUB_TASKS"])
    mode = os.environ.get("STUB_MODE", "none")
    task_id = find_task(prompt, tasks)
    code = 0
    if mode in ("reference", "wrong"):
        patch = (tasks / task_id / f"{mode}.patch").read_bytes()
        done = subprocess.run(
            ["git", "apply", "--whitespace=nowarn"], input=patch, capture_output=True
        )
        if done.returncode != 0:
            sys.stderr.write(done.stderr.decode(errors="replace"))
            return 1
    elif mode == "hunt":
        code = 1 if hunt() else 0
    elif mode == "facts":
        facts(Path(os.environ["STUB_OUT"]))
    result = {
        "type": "result",
        "subtype": "success",
        "is_error": False,
        "total_cost_usd": 0.01,
        "num_turns": 3,
        "modelUsage": {"claude-opus-5-5": {"inputTokens": 1, "outputTokens": 1, "costUSD": 0.01}},
    }
    print(json.dumps(result))
    return code


if __name__ == "__main__":
    sys.exit(main())
