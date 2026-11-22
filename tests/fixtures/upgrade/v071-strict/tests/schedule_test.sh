#!/usr/bin/env bash
# covers: requirements/valves#drip-first
set -euo pipefail

out="$(src/schedule.sh $'sprinkler-1\ndrip-1')"
[ "${out%%$'\n'*}" = "drip-1" ]

# covers: requirements/valves#close-on-timeout
out="$(src/schedule.sh $'drip-1')"
[ "${out}" = "drip-1" ]
