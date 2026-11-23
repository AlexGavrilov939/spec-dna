"""Prepare the tasks of protocol 2 of the model-team measurement (ADR 0033).

Usage: prepare.py [<id>...]

The command reads `recipes.json` and the history of this repository (the
commits of master and the heads of the pull requests, `refs/pull/<n>/head`).
For each task it writes the folder `tasks/<id>/`:

- `reference.patch` and `wrong.patch`: the change from the start tree to the
    reference solution and to the solution with the known defects. The patches
    hold the paths under `src/`, `tests/`, `schema/` and the manifests only.
- `oracle/`: each file of `tests/` that the reference solution adds or changes.
    The runner copies these files into a copy after the agent has stopped.
- `candidates.json`: the names of the tests of the two parts, before the
    proof. `selftest.py` then writes `parts.json`, the lists that count.

A recipe has the keys:

- `slug`: the folder of the change in `docs/intent/` (the brief of the task).
- `scope`: the sentence of the prompt that names the tasks of the plan to do.
- `start`: the commit that the agent starts from (the parent of the slice).
- `slice`: the squash commit of the slice, the first implementation as merged.
- `mode`: `forward`, `reverse` or `pick`. In `forward`, the solution with the defects
    is the tree of `slice`, and the reference is that tree with `fixes` applied.
    In `reverse`, the reference is the tree of `slice`, and the solution with the
    defects is that tree with `fixes` taken out (the fixes are commits of the
    branch of the pull request, which the squash merge holds).
    In `pick`, the slice is a part of a pull request: the solution with the
    defects is the start tree with `commits` applied, and the reference adds
    `fixes`.
- `test_files`: optional. Only these files of `tests/` hold the oracle.
- `fixes`: the commits of the later fixes, oldest first.
- `refs`: the pull request refs to fetch.
"""

import json
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[1]
TASKS = HERE / "tasks"
PATCH_PATHS = ["src", "tests", "schema", "Cargo.toml", "Cargo.lock", "build.rs"]


def git(*args: str, cwd: Path = REPO, check: bool = True, data: bytes | None = None) -> bytes:
    done = subprocess.run(["git", *args], cwd=cwd, input=data, capture_output=True)
    if check and done.returncode != 0:
        sys.exit(f"git {' '.join(args)}: {done.stderr.decode(errors='replace')}")
    return done.stdout


def extract(sha: str, dest: Path) -> None:
    dest.mkdir(parents=True, exist_ok=True)
    archive = git("archive", sha)
    subprocess.run(["tar", "-x", "-C", str(dest)], input=archive, check=True)


def commit_patch(sha: str) -> bytes:
    """The change of one commit, for the paths that a solution holds."""
    return git("diff", f"{sha}^", sha, "--", *PATCH_PATHS)


def apply(tree: Path, patch: bytes, reverse: bool = False) -> None:
    """Apply a patch. A hunk that does not apply is rejected and reported: it
    is a unit test inside `src/`, which the oracle does not use."""
    if not patch.strip():
        return
    args = ["apply", "--reject", "-C1", "--whitespace=nowarn"]
    if reverse:
        args.append("-R")
    done = subprocess.run(["git", *args], cwd=tree, input=patch, capture_output=True)
    for rej in sorted(tree.rglob("*.rej")):
        print(f"  rejected hunks in {rej.relative_to(tree)}:")
        for line in rej.read_text().splitlines():
            if line.startswith("@@"):
                print(f"    {line}")
        rej.unlink()
    if done.returncode not in (0, 1):
        sys.exit(f"apply failed in {tree}: {done.stderr.decode(errors='replace')}")


def fix_config_tail(tree: Path) -> None:
    """Finish the one hunk of `src/config.rs` that a picked commit cannot apply
    to the start tree, because a commit that is not picked changed its context."""
    path = tree / "src" / "config.rs"
    text = path.read_text()
    marker = "let mut cfg = Config {"
    if marker not in text or "intent_dirs(&cfg)" in text:
        return
    i = text.index(marker)
    j = text.index("\n        })\n", i)
    tail = (
        "\n        };\n"
        "        // The hook `protect-approvals` reads the folders of the `intent`\n"
        "        // layer, so the harness keeps them.\n"
        "        let dirs = crate::lifecycle::intent_dirs(&cfg);\n"
        "        if let Some(h) = cfg.harness.as_mut() {\n"
        "            h.intent_dirs = dirs;\n"
        "        }\n"
        "        Ok(cfg)\n"
    )
    path.write_text(text[:j] + tail + text[j + len("\n        })\n") :])


def diff_trees(a: Path, b: Path) -> bytes:
    """The patch from tree a to tree b for the paths of a solution."""
    done = subprocess.run(
        ["git", "diff", "--no-index", "--binary", "--no-color", "a", "b"],
        cwd=a.parent,
        capture_output=True,
    )
    # `git diff --no-index` exits 1 when the trees differ.
    if done.returncode not in (0, 1):
        sys.exit(done.stderr.decode(errors="replace"))
    text = done.stdout.decode()
    # Keep the sections of the paths of a solution only. `git diff --no-index`
    # writes the folder names `a` and `b` into the paths: take them out.
    out = []
    for section in re.split(r"(?m)^(?=diff --git )", text):
        # A changed file is `a/a/<path> b/b/<path>`. An added file is
        # `a/b/<path> b/b/<path>`, and a deleted file is `a/a/<path> b/a/<path>`.
        m = re.match(r"diff --git a/[ab]/(\S+) b/[ab]/(\S+)", section)
        if not m:
            continue
        if m.group(2).split("/")[0] in PATCH_PATHS:
            head, sep, rest = section.partition("\n@@")
            head = re.sub(r"(a|b)/(a|b)/", r"\1/", head)
            out.append(head + sep + rest)
    return "".join(out).encode()


def test_functions(text: str) -> dict[str, str]:
    """The text of each `#[test]` function of a Rust file, by its name."""
    found: dict[str, str] = {}
    lines = text.splitlines()
    i = 0
    while i < len(lines):
        if lines[i].strip() == "#[test]" or lines[i].strip().startswith("#[test]"):
            j = i + 1
            while j < len(lines) and lines[j].strip().startswith("#["):
                j += 1
            m = re.match(r"\s*(?:pub\s+)?fn\s+(\w+)", lines[j]) if j < len(lines) else None
            if m:
                start = i
                depth = 0
                k = j
                opened = False
                while k < len(lines):
                    depth += lines[k].count("{") - lines[k].count("}")
                    opened = opened or "{" in lines[k]
                    if opened and depth <= 0:
                        break
                    k += 1
                found[m.group(1)] = "\n".join(lines[start : k + 1])
                i = k
        i += 1
    return found


def changed_files(a: Path, b: Path, prefix: str) -> list[str]:
    """The files under prefix that differ in b from a (added or changed)."""
    out = []
    root = b / prefix
    if not root.is_dir():
        return out
    for path in sorted(root.rglob("*")):
        if not path.is_file():
            continue
        rel = path.relative_to(b)
        old = a / rel
        if not old.is_file() or old.read_bytes() != path.read_bytes():
            out.append(str(rel))
    return out


def prepare(task_id: str, recipe: dict, work: Path) -> None:
    for ref in recipe.get("refs", []):
        n = ref.rsplit("/", 2)[-2]
        git("fetch", "-q", "origin", f"{ref}:refs/remotes/origin/pr-{n}", check=False)
    start = recipe["start"]
    a, sol = work / "a", work / "sol"
    extract(start, a)
    if recipe["mode"] == "pick":
        # The slice is a part of a pull request: the commits that the part holds.
        shutil.copytree(a, sol, symlinks=True)
        for sha in recipe["commits"]:
            apply(sol, commit_patch(sha))
        fix_config_tail(sol)
        wrong_tree = sol
        reference_tree = work / "ref"
        shutil.copytree(sol, reference_tree, symlinks=True)
        for sha in recipe["fixes"]:
            apply(reference_tree, commit_patch(sha))
    else:
        slice_tree = work / "slice"
        extract(recipe["slice"], slice_tree)
        shutil.copytree(slice_tree, sol, symlinks=True)
        if recipe["mode"] == "forward":
            wrong_tree, reference_tree = slice_tree, sol
            for sha in recipe["fixes"]:
                apply(reference_tree, commit_patch(sha))
        else:
            reference_tree, wrong_tree = slice_tree, sol
            for sha in reversed(recipe["fixes"]):
                apply(wrong_tree, commit_patch(sha), reverse=True)
    out = TASKS / task_id
    shutil.rmtree(out, ignore_errors=True)
    (out / "oracle").mkdir(parents=True)
    for name, tree in (("reference", reference_tree), ("wrong", wrong_tree)):
        # `git diff --no-index` needs the two trees as siblings named a and b.
        pair = work / f"pair-{name}"
        pair.mkdir()
        shutil.copytree(a, pair / "a", symlinks=True)
        shutil.copytree(tree, pair / "b", symlinks=True)
        (out / f"{name}.patch").write_bytes(diff_trees(pair / "a", pair / "b"))
    # The oracle holds each file of tests/ that the reference adds or changes.
    files = changed_files(a, reference_tree, "tests")
    if "test_files" in recipe:
        files = [f for f in files if Path(f).name in recipe["test_files"]]
    for rel in files:
        dest = out / "oracle" / rel
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(reference_tree / rel, dest)
    # The candidates of the 2 parts.
    part1: dict[str, list[str]] = {}
    part2: dict[str, list[str]] = {}
    for rel in files:
        if not rel.endswith(".rs") or rel.count("/") != 1:
            continue
        old = test_functions((a / rel).read_text()) if (a / rel).is_file() else {}
        mid = test_functions((wrong_tree / rel).read_text()) if (wrong_tree / rel).is_file() else {}
        ref = test_functions((reference_tree / rel).read_text())
        for name, body in ref.items():
            if name in mid and mid[name] == body:
                if old.get(name) != body:
                    part1.setdefault(rel, []).append(name)
            else:
                part2.setdefault(rel, []).append(name)
    (out / "candidates.json").write_text(
        json.dumps({"part1": part1, "part2": part2}, indent=2, sort_keys=True) + "\n"
    )
    (out / "task.json").write_text(
        json.dumps({"slug": recipe["slug"], "scope": recipe["scope"], "start": start, "test_files": files}, indent=2) + "\n"
    )
    n1 = sum(len(v) for v in part1.values())
    n2 = sum(len(v) for v in part2.values())
    print(f"{task_id}: {len(files)} test files, {n1} part-1 and {n2} part-2 candidates")


def main() -> None:
    recipes = json.loads((HERE / "recipes.json").read_text())
    ids = sys.argv[1:] or sorted(recipes)
    with tempfile.TemporaryDirectory(prefix="team2-prepare-") as tmp:
        for task_id in ids:
            work = Path(tmp) / task_id
            work.mkdir()
            prepare(task_id, recipes[task_id], work)


if __name__ == "__main__":
    main()
