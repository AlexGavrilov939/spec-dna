#!/usr/bin/env bash
# Verifies the assets of a release (ADR 0031). The release workflow
# (.github/workflows/release.yml) runs it on the draft before the publication,
# and the release checklist (.claude/agents/release-engineer.md) runs it again
# after the publication.
#
# Usage: scripts/verify-release.sh TAG DIR
#        scripts/verify-release.sh --download TAG
#
# TAG is `v<version>`. DIR holds the assets of the release. With --download,
# the script first downloads the assets of the release into a temporary
# folder. GH_REPO sets the repository (default: AlexGavrilov939/spec-dna).
# It needs gh and the network.
#
# Steps:
#   1. DIR holds exactly these files: the binary of each target of
#      `scripts/cross-build.sh --list --all`, SHA256SUMS and spec-dna-pin.toml.
#   2. SHA256SUMS has the right sum of each binary.
#   3. `gh attestation verify` finds 1 attestation or more for each asset of
#      this repository, signed by the release workflow for this tag, on a
#      runner that GitHub hosts. A failed asset does not stop the loop.
#
# Exit codes: 0 each asset is verified, 1 a failure, 2 a usage error. The
# summary line `verify-release: <n> of <n> assets verified for <tag>` is on
# standard output only when each asset is verified.
#
# Note: `gh attestation verify` prints nothing on success when its output goes
# to a file or a pipe. This script reads the output with `--jq length`, so a
# command that prints nothing is a failure, not a success. Always check the
# exit code of this script.
set -euo pipefail

usage() {
  echo "usage: scripts/verify-release.sh TAG DIR | scripts/verify-release.sh --download TAG" >&2
  exit 2
}

fail() {
  echo "verify-release: $*" >&2
  exit 1
}

download=false
case ${1-} in
  -h | --help)
    sed -n '2,/^set -/{/^set -/!s/^# \{0,1\}//p;}' "$0"
    exit 0
    ;;
  --download)
    [ $# -eq 2 ] || usage
    download=true
    tag=$2
    dir=
    ;;
  -*) usage ;;
  *)
    [ $# -eq 2 ] || usage
    tag=$1
    dir=$2
    ;;
esac
case $tag in
  v[0-9]*.[0-9]*.[0-9]*) ;;
  *) echo "verify-release: the tag must be v<version>, not $tag" >&2; usage ;;
esac
if ! $download && [ ! -d "$dir" ]; then
  echo "verify-release: no directory: $dir" >&2
  exit 2
fi

repo=${GH_REPO:-AlexGavrilov939/spec-dna}
version=${tag#v}
command -v gh >/dev/null || fail "gh not found on PATH"

if command -v sha256sum >/dev/null; then
  sha256() { sha256sum "$1"; }
else
  sha256() { shasum -a 256 "$1"; }
fi

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

if $download; then
  dir=$work/assets
  mkdir "$dir"
  if ! gh release download "$tag" --repo "$repo" --dir "$dir" 2>"$work/err"; then
    fail "cannot download the assets of $tag: $(head -n 1 "$work/err")"
  fi
fi

# The names of the binaries, then the 2 other assets.
# Read the list first: a failed `cross-build.sh` must stop the script, not
# give a shorter list of assets that then passes.
if ! targets=$("$(dirname "$0")/cross-build.sh" --list --all 2>"$work/err"); then
  fail "cannot read the targets with cross-build.sh --list --all: $(head -n 1 "$work/err")"
fi
[ -n "$targets" ] || fail "cross-build.sh --list --all printed no target"
binaries=
for target in $targets; do
  ext=
  case $target in
    *-windows-*) ext=.exe ;;
  esac
  binaries="$binaries
spec-dna-$version-$target$ext"
done
binaries=${binaries#?}
expected=$(printf '%s\nSHA256SUMS\nspec-dna-pin.toml\n' "$binaries" | LC_ALL=C sort)
total=$(printf '%s\n' "$expected" | wc -l | tr -d ' ')

# Step 1: the folder holds exactly the assets. Nothing runs gh before this.
actual=$(find "$dir" -mindepth 1 -maxdepth 1 -print | sed 's|.*/||' | LC_ALL=C sort)
missing=$(LC_ALL=C comm -23 <(printf '%s\n' "$expected") <(printf '%s\n' "$actual"))
extra=$(LC_ALL=C comm -13 <(printf '%s\n' "$expected") <(printf '%s\n' "$actual"))
bad=0
for name in $missing; do
  echo "verify-release: FAILED $name: absent from $dir" >&2
  bad=$((bad + 1))
done
# A name with a space is not an asset, so read the extra files by line.
while IFS= read -r name; do
  [ -n "$name" ] || continue
  echo "verify-release: FAILED $name: not an asset of $tag" >&2
  bad=$((bad + 1))
done <<EOF
$extra
EOF
[ "$bad" -eq 0 ] || fail "$bad file(s) of $dir are wrong, so no asset was checked with gh"

# Step 2: SHA256SUMS has the sum of each binary.
for name in $binaries; do
  have=$(sha256 "$dir/$name" | awk '{ print $1 }')
  want=$(awk -v n="$name" '$2 == n || $2 == "*" n { print $1 }' "$dir/SHA256SUMS")
  if [ -z "$want" ]; then
    echo "verify-release: FAILED $name: no line in SHA256SUMS" >&2
    bad=$((bad + 1))
  elif [ "$want" != "$have" ]; then
    echo "verify-release: FAILED $name: the sum differs from SHA256SUMS" >&2
    bad=$((bad + 1))
  fi
done
[ "$bad" -eq 0 ] || fail "$bad sum(s) are wrong, so no asset was checked with gh"

# Step 3: the attestations. The exit code and the output stay apart, and no
# command here hides an exit code.
ok=0
for name in $expected; do
  if out=$(gh attestation verify "$dir/$name" \
    --repo "$repo" \
    --signer-workflow "$repo/.github/workflows/release.yml" \
    --source-ref "refs/tags/$tag" \
    --deny-self-hosted-runners \
    --format json --jq length 2>"$work/err"); then
    rc=0
  else
    rc=$?
  fi
  reason=
  if [ "$rc" -ne 0 ]; then
    reason="gh exit code $rc: $(head -n 1 "$work/err")"
  elif ! [[ $out =~ ^[0-9]+$ ]]; then
    reason="gh exit code 0, but it printed ${out:-nothing}, not a count of attestations"
  elif [ "$((10#$out))" -lt 1 ]; then
    reason="gh found no attestation"
  fi
  if [ -n "$reason" ]; then
    echo "verify-release: FAILED $name: $reason" >&2
    bad=$((bad + 1))
  else
    echo "verified $name: $((10#$out)) attestation(s)"
    ok=$((ok + 1))
  fi
done

[ "$bad" -eq 0 ] || fail "FAILED: $ok of $total assets verified for $tag"
[ "$ok" -eq "$total" ] || fail "FAILED: $ok of $total assets verified for $tag"
echo "verify-release: $ok of $total assets verified for $tag"
