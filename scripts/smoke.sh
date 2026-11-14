#!/usr/bin/env bash
# Runs a spec-dna binary on a new git repository: --version, init, check,
# context and show. The release workflow (.github/workflows/release.yml) runs
# it on a native runner of each target before it publishes a release.
#
# Usage: scripts/smoke.sh BINARY [VERSION]
#
# With VERSION, `--version` must print `spec-dna VERSION`.
set -euo pipefail

die() {
  echo "smoke: $*" >&2
  exit 1
}

[ $# -ge 1 ] || die "usage: scripts/smoke.sh BINARY [VERSION]"
bin=$1
want=${2:-}
[ -f "$bin" ] || die "no file: $bin"
chmod +x "$bin"
case $bin in
  /*) ;;
  *) bin=$PWD/$bin ;;
esac

out=$("$bin" --version)
echo "$out"
if [ -n "$want" ] && [ "$out" != "spec-dna $want" ]; then
  die "--version printed '$out', expected 'spec-dna $want'"
fi

dir=$(mktemp -d)
trap 'rm -rf "$dir"' EXIT
cd "$dir"
gitc() { command git -c commit.gpgsign=false "$@"; }
gitc init -q
gitc config user.name smoke
gitc config user.email smoke@example.invalid
mkdir docs src
cat >docs/valves.md <<'EOF'
---
type: spec
status: active
---
# Valves

## Requirements

- **shutoff**: The controller SHALL close a valve after 15 minutes. The check is in `src/valves.rs`.
EOF
echo 'fn main() {}' >src/valves.rs
gitc add -A
gitc commit -q -m seed

"$bin" init --non-interactive
"$bin" check --all
"$bin" show valves#shutoff | grep -q 'SHALL close' || die "show did not print the requirement"
"$bin" context src/valves.rs | grep -q 'SHALL close' || die "context did not serve the citing section"
echo "smoke: ok"
