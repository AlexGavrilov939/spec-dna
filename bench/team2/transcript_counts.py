"""Count the advisor calls of a batch from the transcripts of Claude Code.

Usage: transcript_counts.py <calls.jsonl of tap.py> [--epoch-min <n>] [--json]

For each logged run, the command finds the transcript files of the copy folder
under $CLAUDE_CONFIG_DIR/projects (also the files of subagents), and it counts:
messages of the assistant, advisor calls (a `server_tool_use` block named
`advisor`), advisor results, starts of a subagent (tool `Agent`), all tool uses,
tool uses whose input names a hidden place (a name with "oracle", the tool
folder, the lab or the repository), and tool results that name one. It prints counts for each arm. It reads no text into its
output: no prompt, no answer, no command and no path.

The arm comes from the log, not from the transcript: `none` for the level
`none`, `lean-no-advisor` for the level `lean` with no `advisorModel` key and no
fallback-advisor.md in the copy, else the level.
"""

import collections
import json
import os
import re
import sys
from pathlib import Path

# A touch of a place outside the copy that holds the oracle, the later history
# or the fixed code: the folders of this repository and of its checkouts, the
# public repository and its host, and the tools of the batch (ADR 0033, "Leaks").
HIDDEN = re.compile(
    r"oracle|/var/www|github\.com|githubusercontent|\bgh (pr|api|repo|run|issue|search)|"
    r"git (clone|fetch|remote|ls-remote)|refs/pull|evals/(fixtures|oracle)|claude/projects|lsof|"
    r"bench/team|tasks/t2-|/private/var/tmp|/private/tmp/claude",
    re.I,
)


def arm_of(entry: dict) -> str:
    level = entry.get("level") or "project"
    # The arm `lean-no-advisor` has no `advisorModel` key and no fallback-advisor.md.
    if level == "lean" and not entry.get("advisor_model_key") and not entry.get("fallback_advisor_file"):
        return "lean-no-advisor"
    return level


def transcript_files(copy: str, since: int) -> list[Path]:
    projects = Path(os.environ["CLAUDE_CONFIG_DIR"]) / "projects"
    found = []
    for folder in projects.iterdir():
        if folder.name.endswith("-" + copy):
            found += [f for f in folder.rglob("*.jsonl") if f.stat().st_mtime >= since - 5]
    return found


def count(files: list[Path]) -> dict:
    c = collections.Counter()
    models = set()
    for file in files:
        for line in file.read_text(encoding="utf-8", errors="replace").splitlines():
            try:
                entry = json.loads(line)
            except ValueError:
                continue
            if entry.get("type") != "assistant" and entry.get("type") != "user":
                continue
            message = entry.get("message")
            if not isinstance(message, dict):
                continue
            if entry["type"] == "assistant":
                c["assistant_messages"] += 1
                if message.get("model"):
                    models.add(message["model"])
            content = message.get("content")
            if not isinstance(content, list):
                continue
            for block in content:
                if not isinstance(block, dict):
                    continue
                kind = block.get("type")
                if kind == "server_tool_use" and block.get("name") == "advisor":
                    c["advisor_calls"] += 1
                elif kind == "advisor_tool_result":
                    c["advisor_results"] += 1
                elif kind == "tool_result":
                    if HIDDEN.search(json.dumps(block.get("content", ""))):
                        c["hidden_in_results"] += 1
                elif kind in ("tool_use", "server_tool_use"):
                    c["tool_uses"] += 1
                    if block.get("name") in ("Agent", "Task"):
                        c["subagent_starts"] += 1
                    if HIDDEN.search(json.dumps(block.get("input", ""))):
                        c["hidden_touches"] += 1
    return {**c, "files": len(files), "models": sorted(models)}


def main() -> None:
    args = sys.argv[1:]
    if not args:
        sys.exit(__doc__)
    log = Path(args[0])
    since_min = int(args[args.index("--epoch-min") + 1]) if "--epoch-min" in args else 0
    runs = []
    for line in log.read_text(encoding="utf-8").splitlines():
        entry = json.loads(line)
        if entry["epoch"] < since_min:
            continue
        files = transcript_files(entry["copy"], entry["epoch"])
        runs.append({"arm": arm_of(entry), "task": entry["task"], **count(files)})
    arms = collections.defaultdict(collections.Counter)
    for run in runs:
        a = arms[run["arm"]]
        a["runs"] += 1
        a["runs_with_transcript"] += 1 if run["files"] else 0
        a["runs_with_advisor_call"] += 1 if run.get("advisor_calls") else 0
        for key in ("advisor_calls", "advisor_results", "subagent_starts", "tool_uses", "hidden_touches", "hidden_in_results"):
            a[key] += run.get(key, 0)
    out = {"arms": {k: dict(v) for k, v in sorted(arms.items())}, "runs": runs}
    if "--json" in args:
        print(json.dumps(out, indent=2))
        return
    for arm, a in out["arms"].items():
        print(arm, dict(a))


if __name__ == "__main__":
    main()
