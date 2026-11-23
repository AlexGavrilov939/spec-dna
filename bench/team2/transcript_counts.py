"""Count the turns, the advisor calls and the leaks of a batch from the transcripts of Claude Code.

Usage: transcript_counts.py <calls.jsonl of tap.py> [--epoch-min <n>] [--json]
        transcript_counts.py --copy <copy folder name> [--json]
        transcript_counts.py --selftest

For each logged run, the command finds the transcript files of the copy folder
under $CLAUDE_CONFIG_DIR/projects (also the files of subagents), and it counts:
messages of the assistant, turns, advisor calls (a `server_tool_use` block named
`advisor`), advisor results, starts of a subagent (tool `Agent`), all tool uses,
and 2 counts of tool uses that touch a hidden place (see "Leaks"). It prints
counts for each arm. It reads no text into its output: no prompt, no answer, no
command and no path.

With `--copy`, the command counts one copy folder, with no tap log and with no
time filter. The correction of the probes uses it.

The arm comes from the log, not from the transcript: `none` for the level
`none`, `lean-no-advisor` for the level `lean` with no `advisorModel` key and no
fallback-advisor.md in the copy, else the level.

Turns. The runner reads `num_turns` of the last `result` event of `claude -p`.
A background task that ends after the agent has stopped wakes the agent, and each
wake-up ends with its own `result` event whose `num_turns` counts that segment
only. So the runner records the turns of the last segment (2 for a run of 152
turns). The transcript of the main session holds one `user` entry for each turn:
the prompt, a wake-up message or a tool result. `turns` is the count of these
entries. It agrees with the `num_turns` of the 2 probes that had no wake-up (93 and
98). The entries of subagents do not count.

Leaks (ADR 0033, "Leaks" and its amendment of 2026-11-23). There are 2 counts:

- `hidden_touches` is the pattern of the ADR. It reads the whole input of a tool
    use, so it also matches the output folder of Claude Code, `git remote -v` in the
    copy and a test text with `gh pr`.
- `strict_touches` reads the target of a call only: the command of a shell call,
    and the path, the pattern of a glob or the URL of another call. It does not read
    the text that a call writes. It matches the oracle, a checkout of the repository
    (a path under /var/www), the code host, `refs/pull` and a call of `gh` or of
    `git clone` or `git ls-remote`. A `gh` inside a heredoc is no call. A `gh` in a
    quoted text counts as a call. It is reported beside the pattern of the ADR and does not replace it.
"""

import collections
import json
import os
import re
import sys
import tempfile
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

# The stricter column. A path under /var/www has slashes on both sides, so the
# output folder of Claude Code (it names the folder of a project with hyphens) is
# no match.
STRICT_TARGET = re.compile(
    r"oracle|/var/www/|github\.com|githubusercontent|refs/pull|bench/team|tasks/t2-|"
    r"evals/(fixtures|oracle)",
    re.I,
)
# `gh` with a subcommand (also with a path before it, flags before the subcommand, after
# `if`, in `bash -c "..."` or in a quoted text: a miss costs more than a false hit in
# this column), and `git clone` and `git ls-remote` (also after `-C dir`).
STRICT_COMMAND = re.compile(
    r"(^|[\s;&|(`'\"{])(\S*/)?gh\s+(-{1,2}\S+\s+([^-\s]\S*\s+)?)*"
    r"(pr|api|repo|run|issue|search|auth|browse|release|gist|workflow|status)\b"
    r"|\bgit\b[^;&|\n]*\b(clone|ls-remote)\b"
)
# The body of a heredoc that only writes text (`cat` or `tee`) is no call. The body of another
# heredoc (`bash <<EOF`, `python3 - <<EOF`) can run, so it counts. `<<<` is no heredoc.
HEREDOC = re.compile(
    r"(?<=cat)([^\n<]*)(?<!<)<<(?!<)-?\s*(['\"]?)(\w+)\2[^\n]*\n.*?\n\s*\3\b"
    r"|(?<=tee)([^\n<]*)(?<!<)<<(?!<)-?\s*(['\"]?)(\w+)\5[^\n]*\n.*?\n\s*\6\b",
    re.S,
)
# The keys of a tool input that name where a call goes. `pattern` counts for a glob only.
TARGET_KEYS = ("command", "file_path", "path", "notebook_path", "url", "glob", "query")


def strict_hit(name: str, tool_input: object) -> bool:
    if not isinstance(tool_input, dict):
        return False
    targets = [str(tool_input[k]) for k in TARGET_KEYS if k in tool_input]
    if name == "Glob" and "pattern" in tool_input:
        targets.append(str(tool_input["pattern"]))
    for target in targets:
        if STRICT_TARGET.search(target):
            return True
    command = tool_input.get("command")
    if isinstance(command, str):
        if STRICT_COMMAND.search(HEREDOC.sub("", command)):
            return True
    return False


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
        # The files of subagents are in a folder `subagents`. Their entries are
        # no turns of the main session.
        main_file = "subagents" not in file.parts
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
            if entry["type"] == "user" and main_file and not entry.get("isSidechain"):
                c["turns"] += 1
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
                    if strict_hit(str(block.get("name")), block.get("input")):
                        c["strict_touches"] += 1
    for key in SUMMED + ("assistant_messages",):
        c[key] += 0
    return {**c, "files": len(files), "models": sorted(models)}


SUMMED = (
    "advisor_calls",
    "advisor_results",
    "subagent_starts",
    "tool_uses",
    "turns",
    "hidden_touches",
    "strict_touches",
    "hidden_in_results",
)


def main() -> int:
    args = sys.argv[1:]
    if not args:
        sys.exit(__doc__)
    if args == ["--selftest"]:
        return selftest()
    if "--copy" in args:
        name = args[args.index("--copy") + 1]
        out = count(transcript_files(name, 0))
        print(json.dumps(out, indent=2) if "--json" in args else out)
        return 0
    log = Path(args[0])
    since_min = int(args[args.index("--epoch-min") + 1]) if "--epoch-min" in args else 0
    runs = []
    for line in log.read_text(encoding="utf-8").splitlines():
        entry = json.loads(line)
        if entry["epoch"] < since_min:
            continue
        files = transcript_files(entry["copy"], entry["epoch"])
        runs.append({"arm": arm_of(entry), "task": entry["task"], "epoch": entry["epoch"], **count(files)})
    # Runs of several processes can write the log at the same time. The time orders them.
    runs.sort(key=lambda r: r["epoch"])
    arms = collections.defaultdict(collections.Counter)
    for run in runs:
        a = arms[run["arm"]]
        a["runs"] += 1
        a["runs_with_transcript"] += 1 if run["files"] else 0
        a["runs_with_advisor_call"] += 1 if run.get("advisor_calls") else 0
        a["runs_hidden"] += 1 if run.get("hidden_touches") else 0
        a["runs_strict"] += 1 if run.get("strict_touches") else 0
        for key in SUMMED:
            a[key] += run.get(key, 0)
    out = {"arms": {k: dict(v) for k, v in sorted(arms.items())}, "runs": runs}
    if "--json" in args:
        print(json.dumps(out, indent=2))
        return 0
    for arm, a in out["arms"].items():
        print(arm, dict(a))
    return 0


def selftest() -> int:
    ok = True

    def expect(name: str, got: object, want: object) -> None:
        nonlocal ok
        good = got == want
        print(f"{'ok  ' if good else 'FAIL'} {name}: {got}")
        ok = ok and good

    def user(content: object) -> dict:
        return {"type": "user", "message": {"role": "user", "content": content}}

    def tool_use(name: str, tool_input: dict) -> dict:
        block = {"type": "tool_use", "name": name, "input": tool_input}
        return {"type": "assistant", "message": {"role": "assistant", "model": "m", "content": [block]}}

    result = user([{"type": "tool_result", "content": "ok"}])
    with tempfile.TemporaryDirectory() as tmp:
        folder = Path(tmp) / "projects" / "-tmp-spec-dna-eval-1-1"
        (folder / "s1" / "subagents").mkdir(parents=True)
        # A main session of 2 segments: the prompt and 1 tool result, then a
        # wake-up message and 1 tool result. It also has 4 turns that touch no
        # hidden place, and a subagent with 3 user entries that count for no turn.
        main = [
            user("do the task"),
            tool_use("Bash", {"command": "cargo test"}),
            result,
            {"type": "assistant", "message": {"role": "assistant", "model": "m", "content": [{"type": "text", "text": "done"}]}},
            user("<task-notification>"),
            tool_use("Bash", {"command": "git status"}),
            result,
        ]
        sub = [user("verify"), result, result]
        (folder / "s1.jsonl").write_text("\n".join(json.dumps(e) for e in main) + "\n")
        (folder / "s1" / "subagents" / "agent-a.jsonl").write_text(
            "\n".join(json.dumps({**e, "isSidechain": True}) for e in sub) + "\n"
        )
        os.environ["CLAUDE_CONFIG_DIR"] = tmp
        got = count(transcript_files("spec-dna-eval-1-1", 0))
        expect("turns of the main session, 2 segments", got["turns"], 4)
        expect("a subagent counts for no turn", got["files"], 2)
        expect("no leak in plain calls", (got.get("hidden_touches", 0), got.get("strict_touches", 0)), (0, 0))

    # The false positives of the probes: the ADR pattern flags them, the strict column does not.
    cases = [
        ("output folder of Claude Code", "Read", {"file_path": "/private/tmp/claude-501/-private-var-folders-x-T-spec-dna-eval-1-1/t/tasks/a.output"}, 1, 0),
        ("git remote -v in the copy", "Bash", {"command": "git remote -v"}, 1, 0),
        ("test text with gh pr", "Write", {"file_path": "tests/hook.rs", "content": "let c = \"gh pr create\";"}, 1, 0),
        ("gh pr in a heredoc", "Bash", {"command": "cat > t.sh <<'EOF'\necho run gh pr view 5\nEOF\nbash t.sh"}, 1, 0),
        # The real leaks: the strict column flags them.
        ("read of the oracle", "Read", {"file_path": "/work/evals/oracle/t2-05/tests/hook.rs"}, 1, 1),
        ("a checkout of the repository", "Bash", {"command": "cat /private/var/www/spec-dna/src/hook.rs"}, 1, 1),
        ("the code host", "Bash", {"command": "curl -s https://github.com/o/r/pull/9.diff"}, 1, 1),
        ("a url fetch", "WebFetch", {"url": "https://raw.githubusercontent.com/o/r/m/a.rs"}, 1, 1),
        ("refs/pull", "Bash", {"command": "git fetch origin refs/pull/60/head"}, 1, 1),
        ("gh as a command", "Bash", {"command": "cd x && gh pr view 5"}, 1, 1),
        ("gh after if", "Bash", {"command": "if gh pr view 5; then echo y; fi"}, 1, 1),
        ("gh with a path", "Bash", {"command": "/opt/homebrew/bin/gh api repos/o/r"}, 1, 1),
        ("gh in bash -c", "Bash", {"command": "bash -c \"gh pr list\""}, 1, 1),
        ("gh after a pipe", "Bash", {"command": "echo x | gh issue list"}, 1, 1),
        ("gh in a quoted text counts", "Bash", {"command": "echo \"run gh pr view\" > note.txt"}, 1, 1),
        ("gh with a flag before the subcommand", "Bash", {"command": "gh -R owner/repo pr view 60"}, 0, 1),  # the ADR pattern misses it
        ("gh with a long flag", "Bash", {"command": "gh --repo owner/repo pr diff 60"}, 0, 1),  # the ADR pattern misses it
        ("gh in a heredoc that a shell runs", "Bash", {"command": "bash <<EOF\necho run gh pr view 60\nEOF"}, 1, 1),
        ("gh after a here string", "Bash", {"command": "cat <<< \"x\"\ngh pr view 5"}, 0, 1),  # the ADR pattern misses it
        ("git clone after -C", "Bash", {"command": "git -C dir clone ssh://example.invalid/r.git"}, 0, 1),  # the ADR pattern misses it
        ("a web search of the host", "WebSearch", {"query": "site:github.com spec-dna"}, 1, 1),
        ("a word that ends in gh", "Bash", {"command": "cargo test --test high pr_list"}, 0, 0),
        ("glob into a checkout", "Glob", {"pattern": "/private/var/www/*/src/hook.rs"}, 1, 1),
    ]
    for label, name, tool_input, want_adr, want_strict in cases:
        adr = 1 if HIDDEN.search(json.dumps(tool_input)) else 0
        strict = 1 if strict_hit(name, tool_input) else 0
        expect(label, (adr, strict), (want_adr, want_strict))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
