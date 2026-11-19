#!/usr/bin/env bash
# Records how the fixtures of the upgrade were made (step 5, slice 1).
#
# The fixtures are projects that pin 0.4.2, 0.5.0, 0.6.1 or 0.7.0. Only the real
# binaries of those versions give the exact bytes and markers of the files that
# `init` writes, so the generated files are committed. CI does not run this
# script. The tests read what it records.
#
# What a person writes and what the script writes:
#
#   v04/, v05/, brownfield/   The documents, the sources and `spec-dna.toml`
#                             are written by hand. The script writes the
#                             adapters: `AGENTS.md`, `CLAUDE.md` and
#                             `.claude/skills/spec-dna/SKILL.md`.
#                             v04/ is `tests/fixtures/corpus` of the tag
#                             v0.4.2 with the pin 0.4.2. v05/ is the same
#                             folder of the tag v0.5.0 with the pin 0.5.0,
#                             and more roots, budgets and documents (one for
#                             each type, and each status of a change).
#                             brownfield/ has the documents of
#                             `tests/fixtures/brownfield` of the tag v0.5.0,
#                             more documents, and a 0.5 `spec-dna.toml`. The
#                             documents of the tags stay byte for byte.
#   harness-lean/,            Overlays of `v05/`. Each one has the files that
#   harness-full/             `init` of 0.5.0 adds or changes when the file
#                             `spec-dna.toml.append` is added to the
#                             configuration: the agent files, `settings.json`
#                             and `CLAUDE.md`. The helper
#                             `Repo::from_fixture` adds the text of the file
#                             `.append` at the end of the file of the same
#                             name.
#   harness-edited/           An overlay of `harness-lean/`. It has `checker.md`
#                             with one more line, so its hash in the marker
#                             does not match any more (SD501).
#   top-files/                An overlay of `v05/` with documents only, written
#                             by hand: a roadmap change and notes and
#                             references that `upgrade --layout` makes top
#                             files. It has no generated file, so this script
#                             records nothing for it.
#   v061/                     A project of 0.6.1 with a change that has an
#                             accepted intent, an approved spec and an
#                             approved plan. The documents (`docs/`), the
#                             script (`src/`) and its test (`tests/`) are
#                             written by hand, in the domain of the other
#                             fixtures. The script runs `init --answers` of
#                             0.6.1 (`standard`, `lean`, no CI, no hook,
#                             `skeleton`) on them, and it writes the rest:
#                             `spec-dna.toml`, `AGENTS.md`, `CLAUDE.md`, the
#                             adapter skill, `.claude/`, `REVIEW.md` and
#                             `evals/`. The tests of `upgrade_v061.rs` read it.
#   v070/                     A project of 0.7.0 with the profile `standard`,
#                             the harness `lean`, the branch approval, a
#                             roadmap and a questions file, a decision, a
#                             change with an approved spec and an approved
#                             plan (the tasks have `Hours:`), and a
#                             `.pre-commit-config.yaml` with the hook
#                             `typos`. The documents (`docs/`), the script
#                             (`src/`), its test (`tests/`) and
#                             `.pre-commit-config.yaml` are written by hand,
#                             in the domain of the other fixtures. The script
#                             runs `init --answers` of 0.7.0 on them, and it
#                             writes the rest: `spec-dna.toml`, `AGENTS.md`,
#                             `CLAUDE.md`, the adapter skill, `.claude/`,
#                             `REVIEW.md` and `evals/`. The answers file sets
#                             the graph and the approver, so the result does
#                             not depend on the git configuration of the
#                             machine. The tests of `upgrade_v070.rs` read it.
#                             0.7.0 prints no note about `typos`: that note
#                             is a feature of `upgrade` of 0.7.1.
#   recorded/<case>.txt       The result of the real binary on the composed
#                             tree of each case: the exit code of
#                             `check --all`, each document that it loaded, each
#                             finding, and each file of the tree with its git
#                             blob id. A test compares the blob ids, so an edit
#                             of a fixture file needs a new run of this script.
#
# Use:
#
#   tests/fixtures/upgrade/make.sh [old|v061|v070]
#
# With no argument, the script makes each fixture. `old` makes the fixtures of
# 0.4.2 and 0.5.0, `v061` makes the fixture of 0.6.1, and `v070` makes the
# fixture of 0.7.0.
#
# The script needs `git`, `cargo` and `python3`. It builds the tags `v0.4.2`,
# `v0.5.0`, `v0.6.1` and `v0.7.0` of this repository in a temporary `git worktree`, with
# `cargo install --locked --root`. It deletes the worktrees and their build
# folders at the end. The disk needs about 1 GB. It never uses the binary of
# `~/.cargo/bin` unless you give its path in a variable.
# To use binaries that exist already, set `SPEC_DNA_042`, `SPEC_DNA_050`,
# `SPEC_DNA_061` and `SPEC_DNA_070` to their paths.
#
# The script changes only files in this folder. Run it twice: the second run
# must give no diff (`git status --short tests/fixtures/upgrade`).

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../../.." && pwd)"
work="$(mktemp -d "${TMPDIR:-/tmp}/spec-dna-fixtures.XXXXXX")"

cleanup() {
  for tag in 0.4.2 0.5.0 0.6.1 0.7.0; do
    if [ -d "$work/wt-$tag" ]; then
      git -C "$repo" worktree remove --force "$work/wt-$tag" || true
    fi
  done
  git -C "$repo" worktree prune
  rm -rf "$work"
}
trap cleanup EXIT

# The binary of a version: the variable, or a build from the tag.
binary() {
  local version="$1" var="$2"
  if [ -n "${!var:-}" ]; then
    echo "${!var}"
    return
  fi
  git -C "$repo" worktree add --detach "$work/wt-$version" "v$version" >&2
  cargo install --locked --root "$work/bin-$version" \
    --target-dir "$work/target" --path "$work/wt-$version" >&2
  echo "$work/bin-$version/bin/spec-dna"
}

# The groups that the arguments select. No argument selects each group.
selected() {
  [ "$#" -ge 1 ] || return 1
  local group="$1"
  shift
  [ "$#" -eq 0 ] && return 0
  local arg
  for arg in "$@"; do
    [ "$arg" = "$group" ] && return 0
  done
  return 1
}

# The builds come before the change of HOME: cargo needs the real home to find
# its toolchain and its crates.
if selected old "$@"; then
  bin042="$(binary 0.4.2 SPEC_DNA_042)"
  bin050="$(binary 0.5.0 SPEC_DNA_050)"
fi
if selected v061 "$@"; then
  bin061="$(binary 0.6.1 SPEC_DNA_061)"
fi
if selected v070 "$@"; then
  bin070="$(binary 0.7.0 SPEC_DNA_070)"
fi

# No file of the person or of the machine reaches the result.
export HOME="$work/home"
export GIT_CONFIG_GLOBAL=/dev/null
export GIT_CONFIG_NOSYSTEM=1
mkdir -p "$HOME"

fixed_git() {
  git -c user.name=Test -c user.email=test@example.org \
    -c commit.gpgsign=false "$@"
}

# A new repository in $1 with the files of $1 in one commit.
commit_tree() {
  git -C "$1" init -q -b main
  git -C "$1" add -A
  fixed_git -C "$1" commit -q -m fixture
}

# The text of `AGENTS.md` before `init` adds its line. The same text is the
# start of `AGENTS.md` in each fixture.
seed_agents() {
  printf '# Agents\n\nRun the tests before a commit.\n' >"$1/AGENTS.md"
}

# Writes the adapters of one fixture with the binary $2.
make_adapters() {
  local name="$1" bin="$2" tmp="$work/adapters-$1"
  rm -rf "$tmp"
  cp -R "$here/$name" "$tmp"
  rm -rf "$tmp/CLAUDE.md" "$tmp/.claude"
  seed_agents "$tmp"
  commit_tree "$tmp"
  (cd "$tmp" && "$bin" init --non-interactive >/dev/null)
  rm -rf "$here/$name/CLAUDE.md" "$here/$name/.claude"
  cp "$tmp/AGENTS.md" "$tmp/CLAUDE.md" "$here/$name/"
  mkdir -p "$here/$name/.claude/skills/spec-dna"
  cp "$tmp/.claude/skills/spec-dna/SKILL.md" \
    "$here/$name/.claude/skills/spec-dna/SKILL.md"
}

# Writes the overlay of one harness level with the binary 0.5.0.
make_harness() {
  local level="$1" tmp="$work/harness-$1" out="$here/harness-$1"
  rm -rf "$tmp" "$out"
  cp -R "$here/v05" "$tmp"
  commit_tree "$tmp"
  printf '\n[harness]\nlevel = "%s"\n' "$level" >"$work/append-$level"
  cat "$work/append-$level" >>"$tmp/spec-dna.toml"
  (cd "$tmp" && "$bin050" init --non-interactive >/dev/null)
  mkdir -p "$out"
  cp "$work/append-$level" "$out/spec-dna.toml.append"
  # Each file that `init` added or changed, except the configuration.
  (cd "$tmp" && git status --porcelain -uall | cut -c4- | sort) |
    while IFS= read -r file; do
      [ "$file" = spec-dna.toml ] && continue
      mkdir -p "$out/$(dirname "$file")"
      cp "$tmp/$file" "$out/$file"
    done
}

# One more line in `checker.md`, before the marker. The marker keeps the hash
# of the text that `init` wrote.
make_edited() {
  local out="$here/harness-edited" src="$here/harness-lean/.claude/agents"
  rm -rf "$out"
  mkdir -p "$out/.claude/agents"
  awk '{ lines[NR] = $0 } END {
    for (i = 1; i < NR; i++) print lines[i]
    print "Run the linter after the tests."
    print lines[NR]
  }' "$src/checker.md" >"$out/.claude/agents/checker.md"
}

# The composed tree of a case: the fixture, then each overlay.
compose() {
  local dest="$1"
  shift
  mkdir -p "$dest"
  local part
  for part in "$@"; do
    (cd "$here/$part" && find . -type f | sort) | while IFS= read -r file; do
      file="${file#./}"
      mkdir -p "$dest/$(dirname "$file")"
      case "$file" in
        *.append) cat "$here/$part/$file" >>"$dest/${file%.append}" ;;
        *) cp "$here/$part/$file" "$dest/$file" ;;
      esac
    done
  done
}

# Runs `check --all --json` of the old binary on a composed case, and writes
# `recorded/<case>.txt`.
record() {
  local name="$1" bin="$2"
  shift 2
  local tmp="$work/record-$name"
  rm -rf "$tmp"
  compose "$tmp" "$@"
  commit_tree "$tmp"
  local code=0
  (cd "$tmp" && "$bin" check --all --json >"$work/result.json") || code=$?
  mkdir -p "$here/recorded"
  {
    echo "# case $name, recorded with $("$bin" --version)"
    echo "exit $code"
    python3 -I - "$work/result.json" <<'PY'
import json
import sys

with open(sys.argv[1]) as f:
    result = json.load(f)
for d in sorted(result["documents"], key=lambda d: d["path"]):
    print("doc", d["path"], d["id"], d["type"], d["status"] or "-")
for d in sorted(result["diagnostics"], key=lambda d: (d["path"], d["code"], d["line"])):
    print("finding", d["code"], d["severity"], d["path"])
PY
    (cd "$tmp" && git ls-files -s | awk '{ print "file", $2, $4 }' | LC_ALL=C sort -k3)
  } >"$here/recorded/$name.txt"
}

# The fixture of 0.6.1. The hand-written folders stay, and `init` of 0.6.1
# writes the rest.
make_v061() {
  local bin="$1" tmp="$work/gen-v061" out="$here/v061"
  rm -rf "$tmp"
  mkdir -p "$tmp"
  local dir
  for dir in docs src tests; do
    cp -R "$out/$dir" "$tmp/$dir"
  done
  seed_agents "$tmp"
  commit_tree "$tmp"
  cat >"$work/answers-v061.toml" <<'ANSWERS'
profile = "standard"
harness = "lean"
ci = "none"
evals = "skeleton"
hook = "none"
ANSWERS
  (cd "$tmp" && "$bin" init --answers "$work/answers-v061.toml" >/dev/null)
  find "$out" -mindepth 1 -maxdepth 1 ! -name docs ! -name src ! -name tests \
    -exec rm -rf {} +
  # Each file that `init` added or changed.
  (cd "$tmp" && git status --porcelain -uall | cut -c4- | sort) |
    while IFS= read -r file; do
      mkdir -p "$out/$(dirname "$file")"
      cp "$tmp/$file" "$out/$file"
    done
}

# The fixture of 0.7.0. The hand-written files stay, and `init` of 0.7.0 writes
# the rest. The answers set the graph and the approver, because `init` reads
# `user.email` of git otherwise.
make_v070() {
  local bin="$1" tmp="$work/gen-v070" out="$here/v070"
  rm -rf "$tmp"
  mkdir -p "$tmp"
  local item
  for item in docs src tests .pre-commit-config.yaml; do
    cp -R "$out/$item" "$tmp/$item"
  done
  seed_agents "$tmp"
  commit_tree "$tmp"
  cat >"$work/answers-v070.toml" <<'ANSWERS'
profile = "standard"
graph = "default"
approvers = ["owner@example.test"]
harness = "lean"
ci = "none"
evals = "skeleton"
hook = "none"
ANSWERS
  (cd "$tmp" && "$bin" init --answers "$work/answers-v070.toml" >/dev/null)
  find "$out" -mindepth 1 -maxdepth 1 ! -name docs ! -name src ! -name tests \
    ! -name .pre-commit-config.yaml -exec rm -rf {} +
  # Each file that `init` added or changed.
  (cd "$tmp" && git status --porcelain -uall | cut -c4- | sort) |
    while IFS= read -r file; do
      mkdir -p "$out/$(dirname "$file")"
      cp "$tmp/$file" "$out/$file"
    done
}

if selected old "$@"; then
  make_adapters v04 "$bin042"
  make_adapters v05 "$bin050"
  make_adapters brownfield "$bin050"
  make_harness lean
  make_harness full
  make_edited

  mkdir -p "$here/recorded"
  for name in v04 v05 brownfield harness-lean harness-full harness-edited; do
    rm -f "$here/recorded/$name.txt"
  done
  record v04 "$bin042" v04
  record v05 "$bin050" v05
  record brownfield "$bin050" brownfield
  record harness-lean "$bin050" v05 harness-lean
  record harness-full "$bin050" v05 harness-full
  record harness-edited "$bin050" v05 harness-lean harness-edited
fi

if selected v061 "$@"; then
  make_v061 "$bin061"
  record v061 "$bin061" v061
fi

if selected v070 "$@"; then
  make_v070 "$bin070"
  record v070 "$bin070" v070
fi
