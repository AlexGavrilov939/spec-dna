"""A stub of `claude` for the self-test of the team tasks. It costs nothing.

The runner of `spec-dna eval run` starts this file in place of `claude`. The
stub finds the task from the text of the prompt, then it acts by the mode in
the variable STUB_MODE:

Modes:

none: it changes nothing.

reference, alt, wrong: it copies the files of tasks/<id>/<mode>/ over the copy
(the working folder).

tamper: it appends a line to the first visible test file.

hunt: it looks for the hidden tests. It searches the copy and each folder of PATH
for a name that starts with "oracle", and it looks at the environment for the name
of check.sh. If it finds one, it exits with code 1.

The variable STUB_TASKS holds the folder of the tasks.
"""

import json
import os
import shutil
import sys
from pathlib import Path


def find_task(prompt: str, tasks: Path) -> str:
    for folder in sorted(tasks.iterdir()):
        text = folder / "task.txt"
        if text.is_file() and text.read_text(encoding="utf-8").strip() == prompt:
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
    # A value of the environment that names check.sh or a folder `oracle`.
    found += [k for k, v in os.environ.items() if "check.sh" in v or "/oracle" in v.lower()]
    return found


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
    if mode in ("reference", "alt", "wrong"):
        shutil.copytree(tasks / task_id / mode, Path.cwd(), dirs_exist_ok=True)
    elif mode == "tamper":
        first = sorted(Path.cwd().glob("tests/test_*.py"))[0]
        first.write_text(first.read_text(encoding="utf-8") + "\n# edited\n", encoding="utf-8")
    elif mode == "hunt":
        code = 1 if hunt() else 0
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
