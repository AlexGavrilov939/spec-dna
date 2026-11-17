#!/usr/bin/env bash
# Measures the cost of the branch walk of ADR 0012 (`approval = "branch"`):
# the git forms that read the paths of each commit, and `spec-dna check` with
# the walk against `check` with `approval = "base"` (no walk).
#
# usage: walk.sh [binary] [files] [commits] [dir]
#   binary   the spec-dna binary. Default: target/release/spec-dna of this
#            repository. Use a release build to compare with results.md.
#   files    the tracked files of the fixture. Default: 2000.
#   commits  the commits of the branch, 2 edited files in each. Default: 500.
#   dir      the folder of the fixture. Default: a new folder from `mktemp -d`.
#            The script deletes the folder at its end.
#
# The check output is never discarded without a test: a pin that differs from
# the binary gives exit code 3, which is fast and meaningless. The script stops
# if `check` gives an exit code other than 0 or 1.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
bin=${1:-$root/target/release/spec-dna}
files=${2:-2000}
commits=${3:-500}
dir=${4:-}
runs=5

if [ ! -x "$bin" ]; then
  echo "walk.sh: no binary at $bin; run 'cargo build --release' first" >&2
  exit 2
fi
bin="$(cd "$(dirname "$bin")" && pwd)/$(basename "$bin")"
if [ -z "$dir" ]; then
  dir=$(mktemp -d)
  trap 'rm -rf "$dir"' EXIT
else
  rm -rf "$dir"
  mkdir -p "$dir"
fi
version=$("$bin" --version | awk '{print $2}')

ms() { python3 -c 'import time; print(int(time.time() * 1000))'; }
median() { sort -n | awk '{a[NR] = $1} END {print a[int((NR + 1) / 2)]}'; }

# run5 <label> <command...>: the median of 5 runs, in ms. A command that
# fails with a code other than 0 or 1 stops the script.
run5() {
  local label=$1
  shift
  local t=() s e rc
  for _ in $(seq 1 "$runs"); do
    s=$(ms)
    rc=0
    "$@" >/dev/null 2>&1 || rc=$?
    e=$(ms)
    if [ "$rc" -gt 1 ]; then
      echo "walk.sh: '$label' gave exit code $rc" >&2
      exit 1
    fi
    t+=($((e - s)))
  done
  MEDIAN=$(printf '%s\n' "${t[@]}" | median)
  printf '%-44s %6s ms (runs: %s)\n' "$label" "$MEDIAN" "${t[*]}"
}

write_config() {
  cat >spec-dna.toml <<EOF
spec-dna = "$version"

[[roots]]
path = "docs"

[change]
policy = "three-class"
gate = "block"
approval = "$1"
approvers = ["owner@example.test"]
EOF
}

cd "$dir"
git init -q -b main
git config user.email owner@example.test
git config user.name Owner
git config commit.gpgsign false
mkdir -p src docs/intent/walk
for i in $(seq 1 "$files"); do
  d=src/m$((i % 50))
  mkdir -p "$d"
  echo "line $i" >"$d/f$i.rs"
done
write_config branch
cat >docs/intent/walk/intent.md <<'EOF'
---
status: accepted
class: routine
---

# Walk

## Intent

Measure the walk.
EOF
cat >docs/intent/walk/plan.md <<'EOF'
---
status: approved
---

# Walk

## Files

- `src/**`

## Order

- [ ] t1: Edit the files. Files: none. Verify: true.
EOF
git add -A
git commit -q -m base
git checkout -q -b work
for c in $(seq 1 "$commits"); do
  for k in 1 2; do
    n=$(((c * 7 + k * 13) % files + 1))
    echo "edit $c $k" >>"src/m$((n % 50))/f$n.rs"
  done
  git commit -q -am "c$c"
done
echo "binary: $bin ($version)"
echo "git: $(git --version)"
echo "load average at the start: $(uptime | sed 's/.*load averages*: //')"
echo "repo: $files files, $(git rev-list --count main..work) commits"
echo

# The git forms. "as now" is the form of 0.7.0 part A.
pairs=$(git rev-list --reverse --topo-order --parents main..work --)
per_commit() {
  while read -r c p; do
    git diff-tree -r -z --no-commit-id --name-status --no-renames "$p" "$c"
  done <<<"$pairs"
}
stdin_form() {
  git diff-tree --stdin -r -z --name-status --no-renames <<<"$pairs"
}
log_form() {
  git log --reverse --topo-order --no-renames -z --name-status \
    --format='%H %P%x00%ae' main..work --
}
run5 "rev-list --parents" git rev-list --reverse --topo-order --parents main..work --
run5 "log %ae" git log --format='%H %ae' main..work --
run5 "diff-tree, one process per commit" per_commit
run5 "diff-tree --stdin, one process" stdin_form
run5 "log --name-status, one process" log_form
a=$(per_commit | tr '\0' '\n' | sort | cksum)
b=$(stdin_form | tr '\0' '\n' | grep -v -E '^[0-9a-f]{40}$' | sort | cksum)
echo "same paths, per commit and --stdin: $([ "$a" = "$b" ] && echo yes || echo NO)"
echo

# `check` with the walk against `check` with no walk. The exit code of the
# first run must be 0 or 1. The 2 modes alternate in each round, so a change of
# the load of the host hits both the same way. The branch mode runs on the
# clean tree. The base mode has the one changed line of `spec-dna.toml`, and it
# reads no commit.
rc=0
"$bin" check --base main --no-cache >/dev/null 2>&1 || rc=$?
if [ "$rc" -gt 1 ]; then
  echo "walk.sh: check gave exit code $rc; the fixture is wrong" >&2
  exit 1
fi
timed_check() {
  local s e rc=0
  s=$(ms)
  "$bin" check --base main --no-cache >/dev/null 2>&1 || rc=$?
  e=$(ms)
  if [ "$rc" -gt 1 ]; then
    echo "walk.sh: check gave exit code $rc" >&2
    exit 1
  fi
  echo $((e - s))
}
bt=()
st=()
for _ in $(seq 1 "$runs"); do
  git checkout -q -- spec-dna.toml
  bt+=("$(timed_check)")
  write_config base
  st+=("$(timed_check)")
done
git checkout -q -- spec-dna.toml
branch_ms=$(printf '%s\n' "${bt[@]}" | median)
base_ms=$(printf '%s\n' "${st[@]}" | median)
printf '%-44s %6s ms (runs: %s)\n' "check --base main, approval = branch" "$branch_ms" "${bt[*]}"
printf '%-44s %6s ms (runs: %s)\n' "check --base main, approval = base" "$base_ms" "${st[*]}"
python3 - "$branch_ms" "$base_ms" <<'EOF2'
import sys
branch, base = float(sys.argv[1]), float(sys.argv[2])
print(f"ratio branch / base: {branch / base:.2f} (target: at most 2.00)")
EOF2
echo "load average at the end: $(uptime | sed 's/.*load averages*: //')"
