#!/usr/bin/env bash
# Example 2: adopt spec-dna in a repository that has documents and old
# problems. README.md of this folder explains each step.
# Usage: run.sh [empty directory]
here=$(cd "$(dirname "$0")" && pwd)
# shellcheck source=../lib.sh
. "$here/../lib.sh"
start "${1:-}"

say "A repository with 2 years of documents: decisions in docs/adr/, specs in docs/specs/ and a runbook."
mkdir -p repo/src repo/docs/adr repo/docs/specs repo/docs/runbooks
cd repo
cat > src/billing.go <<'EOF'
package billing

func ChargeInvoice(id string) error { return nil }
EOF
cat > docs/adr/0001-use-postgres.md <<'EOF'
---
status: accepted
---
# Use PostgreSQL

## Context and Problem Statement

The billing service needs one database.

## Decision Outcome

Chosen option: PostgreSQL, because the team knows it.
EOF
cat > docs/specs/billing.md <<'EOF'
---
status: active
---
# Billing

## Requirements

- **charge-once**: The service SHALL charge an invoice only once. The check is in `src/billing.go` (`ChargeInvoice`).
EOF
cat > docs/runbooks/deploy.md <<'EOF'
# Deploy

1. Read [the release checklist](release-checklist.md).
2. Run `make deploy`.
EOF
run 0 'git init -q -b main && git add -A && git commit -q -m "the history before spec-dna"'

say "init maps each folder to a type. No document moves."
cat > ../answers.toml <<'EOF'
graph = "default"       # the default graph has a layer for procedures
change-policy = "none"
harness = "none"
evals = "none"

[[roots]]               # each folder keeps its name and gets one type
path = "docs/adr"
type = "decision"

[[roots]]
path = "docs/specs"
type = "requirements"

[[roots]]
path = "docs/runbooks"
type = "procedure"
EOF
run 0 'spec-dna init --answers ../answers.toml'
run 0 'git add -A && git commit -q -m "build: adopt spec-dna"'

say "The baseline: each old problem once. The old problems fail check --all."
run 1 'spec-dna check --all'

say "A change on a branch. The gate compares it with main and reports nothing old."
git switch -q -c refunds
cat >> docs/specs/billing.md <<'EOF'
- **refund-window**: The service SHALL accept a refund in the first 14 days.
EOF
run 0 'git commit -q -am "docs: add the refund window"'
run 0 'spec-dna check --base main'

say "The next commit adds a link to a page that does not exist."
perl -pi -e 's/in the first 14 days\./in the first 14 days. See [the refund policy](refund-policy.md)./' docs/specs/billing.md
run 0 'git commit -q -am "docs: link the refund policy"'
run 1 'spec-dna check --base main'
