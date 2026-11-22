#!/usr/bin/env bash
# Example 3: one change from the intent to finish, in 4 pull requests.
# README.md of this folder explains each step.
# Usage: run.sh [empty directory]
here=$(cd "$(dirname "$0")" && pwd)
# shellcheck source=../lib.sh
. "$here/../lib.sh"
start "${1:-}"

# Merges the current branch into main as one squashed commit, as GitHub does
# for a pull request, and pushes main to the bare origin.
merge() {
  local branch
  branch=$(git branch --show-current)
  git switch -q main
  git merge -q --squash "$branch" >/dev/null
  git commit -q -m "$1"
  git push -q origin main
  git branch -q -D "$branch"
}

say "An irrigation controller. A bare repository is the origin, as on GitHub."
git init -q --bare -b main origin.git
mkdir -p repo/src repo/tests
cd repo
cat > src/valves.py <<'EOF'
def close_valve(bed):
    bed.valve_open = False


def on_overflow(bed):
    close_valve(bed)
EOF
cat > tests/test_valves.py <<'EOF'
# covers: requirements/valves#overflow
def test_overflow_closes_the_valve():
    ...
EOF
cat > ../answers.toml <<'EOF'
harness = "none"
evals = "none"
EOF
git init -q -b main
git remote add origin ../origin.git
run 0 'spec-dna init --answers ../answers.toml'
mkdir -p docs/requirements
cat > docs/requirements/valves.md <<'EOF'
---
status: active
---
# Valves

## Requirements

- **overflow**: When a bed overflows, the controller SHALL close the valve of that bed. The check is in `src/valves.py` (`on_overflow()`).
EOF
git add -A
git commit -q -m "build: adopt spec-dna"
git push -q -u origin main
git remote set-head origin main

say "Pull request 1: the intent. The owner accepts it."
git switch -q -c intent
run 0 'spec-dna new intent sensor-timeout'
cat > docs/intent/sensor-timeout/intent.md <<'EOF'
---
status: accepted
class: higher-risk
---
# Sensor timeout

## Intent

When a moisture sensor times out, the controller closes the valve of its bed. The gardener loses no water.

## Why now

Last week, a sensor timeout left a valve open for 6 hours.

## Out of scope

- An alarm for the gardener.
EOF
run 0 'spec-dna check'
git add -A
git commit -q -m "docs: accept the intent sensor-timeout"
merge "docs: accept the intent sensor-timeout (#1)"

say "Pull request 2: the spec. Its delta adds a requirement. The owner approves it."
git switch -q -c spec
run 0 'spec-dna next sensor-timeout'
run 0 'spec-dna new spec sensor-timeout'
cat > docs/intent/sensor-timeout/spec.md <<'EOF'
---
status: approved
---
# Sensor timeout

## Delta

- ADD requirements/valves#sensor-timeout: When a moisture sensor gives no value for 60 seconds, the controller SHALL close the valve of its bed. The check is in `src/valves.py`.

## Interfaces

None.

## Decisions

None.

## Concerns

- [x] A slow sensor can close a valve that must stay open. Resolution: the limit of 60 seconds is 4 times the slowest read in the logs.

## Open questions

- [x] Does the valve open again when the sensor comes back? Answer: no. The gardener opens it.
EOF
run 0 'spec-dna check'
git add -A
git commit -q -m "docs: approve the spec sensor-timeout"
merge "docs: approve the spec sensor-timeout (#2)"

say "Pull request 3: the plan. The owner approves it."
git switch -q -c plan
run 0 'spec-dna new plan sensor-timeout'
cat > docs/intent/sensor-timeout/plan.md <<'EOF'
---
status: approved
---
# Sensor timeout

## Files

- `src/valves.py` The timeout closes the valve.
- `tests/test_valves.py` The test of the timeout.
- `docs/requirements/valves.md` finish adds the requirement.

## Order

- [ ] t1: Close the valve when the sensor times out. Files: `src/valves.py`. Verify: the test passes.
- [ ] t2: Test the timeout. Files: `tests/test_valves.py`. Verify: `spec-dna finish sensor-timeout --dry-run`.

## Risks

- A slow sensor closes a valve that must stay open. The limit is 60 seconds.

## Proof

The test `test_sensor_timeout_closes_the_valve` covers `requirements/valves#sensor-timeout`.
EOF
run 0 'spec-dna check'
git add -A
git commit -q -m "docs: approve the plan sensor-timeout"
merge "docs: approve the plan sensor-timeout (#3)"

say "Pull request 4: the code. The agent writes the code and a test with no covers: tag."
git switch -q -c code
cat >> src/valves.py <<'EOF'


def on_sensor_timeout(bed):
    close_valve(bed)
EOF
cat >> tests/test_valves.py <<'EOF'


def test_sensor_timeout_closes_the_valve():
    ...
EOF
perl -pi -e 's/^- \[ \] t/- [x] t/' docs/intent/sensor-timeout/plan.md
git commit -q -am "feat: close the valve on a sensor timeout"
run 0 'spec-dna next sensor-timeout'

say "The agent links the test to the new requirement."
perl -0pi -e 's/\n\ndef test_sensor_timeout/\n\n# covers: requirements\/valves#sensor-timeout\ndef test_sensor_timeout/' tests/test_valves.py
git commit -q -am "test: cover the sensor timeout"

say "The change is higher-risk, so finish needs an acceptance record from a second agent. It stops and writes nothing."
run 1 'spec-dna finish sensor-timeout'

say "The acceptor, an agent that did not write the code, reads the brief and writes the record. The script stands in for it."
run 0 'spec-dna accept sensor-timeout --brief | sed -n 1p'
brief=$(spec-dna accept sensor-timeout --brief | tail -n 1)
run 0 'spec-dna accept sensor-timeout --session s1'
perl -pi -e 's/^result = "pending"/result = "accepted"/; s/^brief = "pending"/brief = "'"${brief#brief }"'"/' docs/intent/sensor-timeout/acceptance.toml
cat >> docs/intent/sensor-timeout/acceptance.toml <<'EOF'

[[attempt]]
acceptor = 1
text = "Read `on_sensor_timeout` in `src/valves.py`: it calls `close_valve` for the bed, and `close_valve` sets `valve_open` to false, as the requirement says."
outcome = "held"

[[finding]]
acceptor = 1
severity = "note"
path = "tests/test_valves.py"
text = "The test has no body yet."
EOF
git add docs/intent/sensor-timeout/acceptance.toml
git commit -q -m "docs: accept the change sensor-timeout"

say "Now finish applies the delta and deletes the folder."
run 0 'spec-dna finish sensor-timeout'
run 0 'git diff --cached -U0 -- docs/requirements'
git commit -q -m "docs: finish sensor-timeout"
run 0 'spec-dna check --base origin/main'
merge "feat: close the valve on a sensor timeout (#4)"

say "git keeps the history of the change."
run 0 'spec-dna log'
