"""Install the tool folder of a paid batch of protocol 2.

Usage: setup_tools.py <tools folder> <spec-dna binary> <real claude binary>

The folder must not exist or must be empty, and it must be outside the batch
project, so no path of the project is on PATH. The command writes:

- bin/spec-dna: a copy of the binary that runs the batch.
- bin/claude: a shell file that starts tap.py (the tap of protocol 1, see
    ../team/tap.py). It logs one line for each run, with counts and no text.
- zdot/: the startup files of zsh for ZDOTDIR (see ../team/run_scrubbed.sh).
- tap.py, tap.json, calls.jsonl (the log, empty).

Put `<tools folder>/bin` first on PATH when you start `spec-dna eval run`.
"""

import hashlib
import json
import shutil
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from make_project import prompt_of, task_ids  # noqa: E402


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
    shutil.copy(HERE.parent / "team" / "tap.py", tools / "tap.py")
    prompts = {
        hashlib.sha256(prompt_of(task_id).encode()).hexdigest(): task_id for task_id in task_ids()
    }
    log = tools / "calls.jsonl"
    log.write_text("", encoding="utf-8")
    (tools / "tap.json").write_text(
        json.dumps({"real": str(real), "log": str(log), "prompts": prompts}, indent=2) + "\n",
        encoding="utf-8",
    )
    (tools / "zdot").mkdir()
    path_line = f'export PATH="{tools / "bin"}:/opt/homebrew/bin:$PATH"\n'
    for name in (".zshenv", ".zprofile", ".zshrc"):
        (tools / "zdot" / name).write_text(path_line, encoding="utf-8")
    shim = tools / "bin" / "claude"
    shim.write_text(f'#!/bin/sh\nexec "{sys.executable}" -I "{tools / "tap.py"}" "$@"\n')
    shim.chmod(0o755)
    print(f"{tools}: {len(prompts)} tasks")


if __name__ == "__main__":
    main()
