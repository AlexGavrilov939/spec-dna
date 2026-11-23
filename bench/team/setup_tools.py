"""Install the tool folder of a paid batch.

Usage: setup_tools.py <tools folder> <spec-dna binary> <real claude binary>

The folder must not exist or must be empty, and it must be outside the batch
project, so no path of the project is on PATH. The command writes:

- bin/spec-dna: a copy of the binary that runs the batch.
- bin/claude: a shell file that starts tap.py (see tap.py).
- zdot/: the startup files of zsh for ZDOTDIR (see run_scrubbed.sh). They only set PATH.
- tap.py, tap.json, calls.jsonl (the log, empty).

Put `<tools folder>/bin` first on PATH when you start `spec-dna eval run`.
"""

import hashlib
import json
import shutil
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
TASKS = HERE / "tasks"


def main() -> None:
    if len(sys.argv) != 4:
        sys.exit(__doc__)
    tools = Path(sys.argv[1]).resolve()
    binary = Path(sys.argv[2]).resolve()
    real = Path(sys.argv[3]).resolve()
    if tools.exists() and any(tools.iterdir()):
        sys.exit(f"setup_tools: {tools} is not empty")
    (tools / "bin").mkdir(parents=True, exist_ok=True)
    shutil.copy(binary, tools / "bin" / "spec-dna")
    (tools / "bin" / "spec-dna").chmod(0o755)
    shutil.copy(HERE / "tap.py", tools / "tap.py")
    prompts = {}
    for folder in sorted(TASKS.iterdir()):
        text = folder / "task.txt"
        if text.is_file():
            task = text.read_text(encoding="utf-8").strip()
            prompts[hashlib.sha256(task.encode()).hexdigest()] = folder.name
    log = tools / "calls.jsonl"
    log.write_text("", encoding="utf-8")
    (tools / "tap.json").write_text(
        json.dumps({"real": str(real), "log": str(log), "prompts": prompts}, indent=2) + "\n",
        encoding="utf-8",
    )
    (tools / "zdot").mkdir()
    # The shell of the agent has the same PATH as the runner: its tools first, then
    # Homebrew (the same python3 as the checks), then the system folders.
    path_line = f'export PATH="{tools / "bin"}:/opt/homebrew/bin:$PATH"\n'
    for name in (".zshenv", ".zprofile", ".zshrc"):
        (tools / "zdot" / name).write_text(path_line, encoding="utf-8")
    shim = tools / "bin" / "claude"
    shim.write_text(f'#!/bin/sh\nexec "{sys.executable}" -I "{tools / "tap.py"}" "$@"\n')
    shim.chmod(0o755)
    print(f"{tools}: {len(prompts)} tasks")


if __name__ == "__main__":
    main()
