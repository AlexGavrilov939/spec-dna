"""A thin wrapper in place of `claude` for a paid batch. It records counts only.

The runner of `spec-dna eval run` starts `claude` from PATH. The folder of this
file is on PATH instead, with a shell `claude` that starts this file. For a run
(`-p <prompt>`), the wrapper appends one line to a log, then replaces itself
with the real `claude`, so the run is the same as with no wrapper.

The line has: the name of the copy folder, the task id (found from the hash of
the prompt), the harness arm that the runner wrote into the copy, 3 facts of
the harness files of the copy, the values of `--model` and `--effort`, and the
count of the other arguments. It has no prompt text and no path of the project.

Settings are in `tap.json` next to this file: `real` (the real `claude`), `log`
(the log file) and `prompts` (the map from the SHA-256 of a prompt to a task id).
Transcript counts of a batch come from `transcript_counts.py`.
"""

import hashlib
import json
import os
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent


def harness_facts(cwd: Path) -> dict:
    toml = cwd / "spec-dna.toml"
    text = toml.read_text(encoding="utf-8") if toml.is_file() else ""
    level = None
    advisor_none = False
    in_harness = False
    for raw in text.splitlines():
        line = raw.strip()
        if line.startswith("["):
            in_harness = line == "[harness]"
        elif in_harness and line.startswith("level"):
            level = line.split("=", 1)[1].strip().strip('"')
        elif in_harness and line.startswith("advisor") and '"none"' in line:
            advisor_none = True
    settings = cwd / ".claude" / "settings.json"
    settings_text = settings.read_text(encoding="utf-8") if settings.is_file() else ""
    return {
        "level": level,
        "advisor_none": advisor_none,
        "claude_dir": (cwd / ".claude").is_dir(),
        "advisor_model_key": "advisorModel" in settings_text,
        "fallback_advisor_file": (cwd / ".claude" / "agents" / "fallback-advisor.md").is_file(),
    }


def flag(args: list[str], name: str) -> str | None:
    return args[args.index(name) + 1] if name in args[:-1] else None


def main() -> None:
    config = json.loads((HERE / "tap.json").read_text(encoding="utf-8"))
    real = config["real"]
    args = sys.argv[1:]
    if "-p" in args[:-1]:
        prompt = args[args.index("-p") + 1]
        cwd = Path.cwd()
        line = {
            "copy": cwd.name,
            "task": config["prompts"].get(hashlib.sha256(prompt.encode()).hexdigest()),
            "epoch": int(time.time()),
            **harness_facts(cwd),
            "model": flag(args, "--model"),
            "effort": flag(args, "--effort"),
            "argc": len(args),
        }
        with open(config["log"], "a", encoding="utf-8") as log:
            log.write(json.dumps(line) + "\n")
    os.execv(real, [real, *args])


if __name__ == "__main__":
    main()
