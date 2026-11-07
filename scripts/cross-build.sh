#!/usr/bin/env bash
# Cross-builds release binaries for the 7 targets of ADR 0003 with cargo-zigbuild.
#
# Usage: scripts/cross-build.sh [--manifest-path PATH] [--bin NAME] [TARGET...]
#        scripts/cross-build.sh --list
#
# If you give no TARGET, it builds all 7. CI (.github/workflows/build.yml)
# calls it once for each target. It needs zig, cargo-zigbuild, and rustup with
# its cargo first on PATH. It uses CARGO_TARGET_DIR if it is set. It prints the
# `file` output and the SHA-256 of each binary.
set -euo pipefail

targets_all=(
  aarch64-apple-darwin
  x86_64-apple-darwin
  x86_64-unknown-linux-gnu
  aarch64-unknown-linux-gnu
  x86_64-unknown-linux-musl
  x86_64-pc-windows-gnu
  aarch64-pc-windows-gnullvm
)

# The *-linux-gnu binaries need this glibc or a newer one. 2.17 is the minimum
# for Rust itself.
glibc=2.17
# The binaries need this macOS or a newer one. Without this setting, zig writes
# its own newer default (15.0 with zig 0.17). 11.0 is the first release on Apple
# silicon.
export MACOSX_DEPLOYMENT_TARGET=11.0

manifest=Cargo.toml
bin=spec-dna
targets=()

die() {
  echo "cross-build: $*" >&2
  exit 1
}

while [ $# -gt 0 ]; do
  case $1 in
    --manifest-path)
      [ $# -ge 2 ] || die "--manifest-path needs a value"
      manifest=$2
      shift 2
      ;;
    --bin)
      [ $# -ge 2 ] || die "--bin needs a value"
      bin=$2
      shift 2
      ;;
    --list)
      printf '%s\n' "${targets_all[@]}"
      exit 0
      ;;
    -h | --help)
      sed -n '2,10s/^# \{0,1\}//p' "$0"
      exit 0
      ;;
    -*) die "unknown option: $1" ;;
    *)
      targets+=("$1")
      shift
      ;;
  esac
done
if [ ${#targets[@]} -eq 0 ]; then
  targets=("${targets_all[@]}")
fi

for tool in rustup cargo cargo-zigbuild zig file; do
  command -v "$tool" >/dev/null || die "$tool not found on PATH"
done
if command -v sha256sum >/dev/null; then
  sha256() { sha256sum "$1"; }
else
  sha256() { shasum -a 256 "$1"; }
fi

target_dir=$(cargo metadata --format-version 1 --no-deps --locked --manifest-path "$manifest" |
  sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')
[ -n "$target_dir" ] || die "cannot read the target directory from cargo metadata"

for target in "${targets[@]}"; do
  case " ${targets_all[*]} " in
    *" $target "*) ;;
    *) die "unknown target: $target (see --list)" ;;
  esac

  echo "==> $target"
  rustup target add "$target"
  sysroot=$(rustc --print sysroot)
  [ -d "$sysroot/lib/rustlib/$target" ] ||
    die "$(command -v rustc) has no standard library for $target; put rustup's cargo and rustc first on PATH"

  zig_target=$target
  case $target in
    *-linux-gnu) zig_target=$target.$glibc ;;
  esac

  TIMEFORMAT="build time $target: %3R s"
  time cargo zigbuild --locked --release --manifest-path "$manifest" --bin "$bin" --target "$zig_target"

  exe=$target_dir/$target/release/$bin
  case $target in
    *-windows-*) exe=$exe.exe ;;
  esac
  [ -f "$exe" ] || die "binary not found: $exe"
  file "$exe"
  sha256 "$exe"
done
