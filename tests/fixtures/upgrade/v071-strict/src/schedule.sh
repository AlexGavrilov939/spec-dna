#!/usr/bin/env bash
# Prints the lines of a schedule: the drip lines first.
set -euo pipefail

lines="${1:-}"
for kind in drip sprinkler; do
  grep "^${kind}" <<<"${lines}" || true
done
