#!/usr/bin/env bash
# Makes the release assets of ADR 0003 from the binaries of the build jobs:
# the 7 binaries `spec-dna-<version>-<target>[.exe]`, `SHA256SUMS` and
# `spec-dna-pin.toml`. The release workflow (.github/workflows/release.yml)
# calls it.
#
# Usage: scripts/release-assets.sh ARTIFACTS OUT VERSION
#
# ARTIFACTS has one directory `spec-dna-<target>` for each target, with the
# binary `spec-dna` or `spec-dna.exe` in it. OUT must not exist.
set -euo pipefail

die() {
  echo "release-assets: $*" >&2
  exit 1
}

[ $# -eq 3 ] || die "usage: scripts/release-assets.sh ARTIFACTS OUT VERSION"
artifacts=$1
out=$2
version=$3
[ -d "$artifacts" ] || die "no directory: $artifacts"
[ ! -e "$out" ] || die "$out exists"
mkdir -p "$out"

for target in $("$(dirname "$0")/cross-build.sh" --list --all); do
  ext=
  case $target in
    *-windows-*) ext=.exe ;;
  esac
  src=$artifacts/spec-dna-$target/spec-dna$ext
  [ -f "$src" ] || die "no binary for $target: $src"
  cp "$src" "$out/spec-dna-$version-$target$ext"
  chmod +x "$out/spec-dna-$version-$target$ext"
done

if command -v sha256sum >/dev/null; then
  sha256() { sha256sum "$@"; }
else
  sha256() { shasum -a 256 "$@"; }
fi

cd "$out"
# Both files list the assets in the byte order of their names.
names=$(printf '%s\n' spec-dna-"$version"-* | LC_ALL=C sort)
# shellcheck disable=SC2086 # The asset names contain no spaces.
sha256 $names >SHA256SUMS
{
  printf 'spec-dna = "%s"\n\n[spec-dna-sha256]\n' "$version"
  while read -r sum name; do
    printf '"%s" = "%s"\n' "$name" "$sum"
  done <SHA256SUMS
} >spec-dna-pin.toml

[ "$(wc -l <SHA256SUMS)" -eq 7 ] || die "SHA256SUMS does not have 7 lines"
cat SHA256SUMS
