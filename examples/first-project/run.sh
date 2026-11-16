#!/usr/bin/env bash
# Example 1: a first project. README.md of this folder explains each step.
# Usage: run.sh [empty directory]
here=$(cd "$(dirname "$0")" && pwd)
# shellcheck source=../lib.sh
. "$here/../lib.sh"
start "${1:-}"

say "A small service: one function and its test."
cat > answers.toml <<'EOF'
change-policy = "none"  # no change process in this first example
harness = "none"        # no agent harness: only the instruction lines
evals = "none"
EOF
mkdir -p repo/src repo/tests
cd repo
cat > src/orders.py <<'EOF'
def cancel_order(order, minutes_since_order):
    if minutes_since_order > 30:
        raise ValueError("too late to cancel")
    order.state = "cancelled"
EOF
cat > tests/test_orders.py <<'EOF'
# covers: requirements/orders#cancel-window
def test_cancel_after_31_minutes_fails():
    ...
EOF
run 0 'git init -q -b main'
run 0 'spec-dna init --answers ../answers.toml'
run 0 'git add -A && git commit -q -m "build: adopt spec-dna"'

say "Write a requirement that cites the code, and an accepted decision."
mkdir -p docs/requirements docs/decisions
cat > docs/requirements/orders.md <<'EOF'
---
status: active
---
# Orders

## Requirements

- **cancel-window**: The service SHALL accept a cancellation only in the first 30 minutes after the order. The check is in `src/orders.py` (`cancel_order()`).
EOF
cat > docs/decisions/storage.md <<'EOF'
---
status: accepted
---
# Storage

## Context and Problem Statement

The service keeps orders on 3 hosts. It needs one database for all of them.

## Considered Options

- PostgreSQL
- SQLite

## Decision Outcome

Chosen option: PostgreSQL, because all 3 hosts must read the same orders.

## Consequences

- Good: one database for all hosts.
- Bad: one more service to run.

## Status

The service uses PostgreSQL.

## Amendments
EOF
run 0 'spec-dna check'
run 0 'git add -A && git commit -q -m "docs: add the orders requirements and the storage decision"'

say "Before an edit, an agent asks for the sections that apply to the file."
run 0 'spec-dna context src/orders.py'

say "The agent renames the function and the file. It also rewrites the accepted decision to match a new idea."
git mv src/orders.py src/cancel.py
perl -pi -e 's/cancel_order/cancel/' src/cancel.py
perl -pi -e 's/Chosen option: PostgreSQL, because all 3 hosts must read the same orders\./Chosen option: SQLite, because one file is simpler./' docs/decisions/storage.md
run 1 'spec-dna check'

say "The fix: restore the accepted text, write the new fact in an amendment, and update the citation."
git checkout -q -- docs/decisions/storage.md
cat >> docs/decisions/storage.md <<'EOF'

- 2026-11-16: The function `cancel()` in `src/cancel.py` checks the cancel window.
EOF
perl -pi -e 's/`src\/orders.py` \(`cancel_order\(\)`\)/`src\/cancel.py` (`cancel()`)/' docs/requirements/orders.md
run 0 'spec-dna check'

say "Each requirement shows the tests that cover it."
run 0 'spec-dna show requirements/orders'
