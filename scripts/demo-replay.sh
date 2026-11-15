#!/usr/bin/env bash
# Replays the demo of docs/demo.md without an agent: the setup of a new
# project from docs/quickstart.md, an upgrade with a rollback, and the
# uninstall. It runs the "Start" and "Uninstall" blocks of the quickstart as
# they are written, so the test fails if the quickstart is wrong.
#
# Usage: scripts/demo-replay.sh [work directory]
# Needs: git, cargo, python3, awk. It does not use the network after the build.
set -euo pipefail

repo=$(cd "$(dirname "$0")/.." && pwd)
work=${1:-$(mktemp -d)}
mkdir -p "$work"
work=$(cd "$work" && pwd)
seed="$repo/tests/fixtures/demo"
quickstart="$repo/docs/quickstart.md"

step() { printf '\n== %s\n' "$*"; }
fail() {
    printf 'FAIL: %s\n' "$*" >&2
    exit 1
}
# Prints the first `sh` block after the heading "## $1" of the quickstart.
block() {
    awk -v h="## $1" '$0 == h { s = 1; next } s && /^```sh$/ { c = 1; next } c && /^```$/ { exit } c' "$quickstart"
}
export GIT_CONFIG_NOSYSTEM=1 GIT_AUTHOR_NAME=Demo GIT_AUTHOR_EMAIL=demo@example.org
export GIT_COMMITTER_NAME=Demo GIT_COMMITTER_EMAIL=demo@example.org
gitc() { command git -c commit.gpgsign=false -c tag.gpgsign=false "$@"; }

version=$(sed -n 's/^version = "\(.*\)"/\1/p' "$repo/Cargo.toml" | head -n 1)
# The next patch version, unreleased: 0.1.1 gives 0.1.2-dev.
base=${version%%-*}
nextv="${base%.*}.$((${base##*.} + 1))-dev"
step "build the current version $version and an unreleased $nextv"
cargo build --quiet --locked --manifest-path "$repo/Cargo.toml" --target-dir "$work/target"
mkdir -p "$work/bin-current" "$work/bin-next"
cp "$work/target/debug/spec-dna" "$work/bin-current/spec-dna"
next="$work/next"
rm -rf "$next"
mkdir -p "$next"
cp -R "$repo/Cargo.toml" "$repo/Cargo.lock" "$repo/src" "$repo/schema" "$next/"
sed -i.bak "s/^version = \"$version\"/version = \"$nextv\"/" "$next/Cargo.toml"
cargo build --quiet --offline --manifest-path "$next/Cargo.toml" --target-dir "$work/target-next"
cp "$work/target-next/debug/spec-dna" "$work/bin-next/spec-dna"
"$work/bin-next/spec-dna" --version | grep -qx "spec-dna $nextv" || fail "the next binary has the wrong version"

use() { export PATH="$work/bin-$1:$base_path"; }
base_path=$PATH
use current
[ "$(spec-dna --version)" = "spec-dna $version" ] || fail "spec-dna $version is not first on PATH"

step "step 3: a new project follows the quickstart"
demo="$work/demo"
rm -rf "$demo"
mkdir -p "$demo"
cp -R "$seed/app/." "$demo/"
cd "$demo"
gitc init -q -b main
gitc add -A
gitc commit -q -m "feat: add the library loans service"
before_claude=$(cat CLAUDE.md)
# The "Start" block of the quickstart, as it is written.
start=$(block Start)
[ -n "$start" ] || fail "no sh block under \"## Start\" in the quickstart"
(
    git() { gitc "$@"; }
    eval "$start"
) || fail "the Start block of the quickstart failed"
[ "$(gitc log --format=%s -1)" = "build: adopt spec-dna" ] || fail "the Start block did not commit"
# The spec-dna line and the model-team line of the lean harness.
added=$(diff <(printf '%s\n' "$before_claude") CLAUDE.md | grep -c '^> .' || true)
[ "$added" = 2 ] || fail "init added $added non-empty lines to CLAUDE.md, not 2"
grep -q '^Model team: ' CLAUDE.md || fail "init wrote no model-team line"
for f in verifier Explore fallback-advisor model-team; do
    [ -f ".claude/agents/$f.md" ] || fail "init wrote no .claude/agents/$f.md"
done
# The stage skills, REVIEW.md and the starter files of the evals (the defaults of
# a new project). The "Start" block adds them to the commit.
for f in .claude/skills/spec-dna-intent/SKILL.md .claude/skills/spec-dna-review/SKILL.md REVIEW.md evals/check.sh evals/example.json evals/fixtures/example/notes.txt; do
    [ -f "$f" ] || fail "init wrote no $f"
done
[ -e .github/workflows/agent-evals.yml ] && fail "init wrote the workflow of the evals for the CI answer none"
grep -q '^\[evals\]' spec-dna.toml || fail "init wrote no [evals] table"
[ -z "$(gitc status --porcelain)" ] || fail "the Start block left a file of init out of its commit"
grep -q '@' CLAUDE.md && fail "init wrote an import into CLAUDE.md"
[ -e AGENTS.md ] && fail "init created AGENTS.md next to an existing CLAUDE.md"
bytes=$(spec-dna init | sed -n 's/^per-request prefix: \([0-9]*\) bytes.*/\1/p')
[ "$bytes" -le 300 ] || fail "the adapters add $bytes bytes to the prompt prefix"
echo "prompt prefix: $bytes bytes"

# The small graph of `init` has no `interfaces` layer, and the demo has an
# interface document. `layer add` writes the layer. The skill lists the types of
# the layers, so `init` writes it again. A change of `spec-dna.toml` is a change
# of its own (SD503): the skill is a file that `init` writes, so it does not
# count.
step "add the interfaces layer"
spec-dna layer add interfaces --types interface --moved-from intent
spec-dna init
spec-dna check
gitc add spec-dna.toml .claude/skills/spec-dna/SKILL.md
gitc commit -q -m "build: add the interfaces layer"

cp -R "$seed/docs" .
spec-dna check
gitc add docs
gitc commit -q -m "docs: add the specifications"
python3 -m unittest discover -s tests -t . 2>&1 | tail -n 1

step "daily use: context, show, check, mv"
spec-dna context library/loans.py >"$work/context.txt"
head -n 3 "$work/context.txt"
grep -q '^<!-- requirements/loans#renewals h:[0-9a-f]* requirements/active cites -->$' "$work/context.txt" ||
    fail "context library/loans.py does not serve requirements/loans#renewals"
spec-dna show requirements/loans#loan-period | grep -q 'LOAN_DAYS' || fail "show requirements/loans#loan-period"
spec-dna mv decisions/in-memory-store decisions/storage
grep -q '^aliases:' docs/decisions/storage.md || fail "mv did not keep the old id"
spec-dna show decisions/in-memory-store >/dev/null || fail "the old id does not resolve"
gitc commit -q -am "docs: rename the storage decision"
# A change that removes a cited identifier gives SD607 in `check --base`, as in CI.
sed -i.bak 's/MAX_RENEWALS/RENEWAL_LIMIT/' library/loans.py
rm library/loans.py.bak
gitc commit -q -am "refactor: rename the renewal limit"
ci="$work/ci"
rm -rf "$ci"
gitc clone -q "$demo" "$ci"
(cd "$ci" && spec-dna check --base HEAD~1) | tee "$work/drift.txt"
grep -q 'warning SD607 cited-symbol-removed: `MAX_RENEWALS`' "$work/drift.txt" ||
    fail "check --base did not report the removed identifier"
gitc revert --no-edit HEAD >/dev/null

step "step 5: upgrade to $nextv, then roll back"
use next
set +e
spec-dna check 2>"$work/pin.txt"
code=$?
set -e
cat "$work/pin.txt"
[ "$code" = 3 ] || fail "a pin mismatch gave exit code $code, not 3"
grep -q "pins version $version, but this binary is $nextv" "$work/pin.txt" || fail "pin message"
sed -i.bak "s/^spec-dna = \"$version\"/spec-dna = \"$nextv\"/" spec-dna.toml
rm spec-dna.toml.bak
spec-dna init
gitc diff --stat
grep -q "<!-- spec-dna v$nextv h:" CLAUDE.md || fail "init did not regenerate the CLAUDE.md line"
grep -q "spec-dna:begin skill v$nextv" .claude/skills/spec-dna/SKILL.md || fail "init did not regenerate the skill"
grep -q "<!-- spec-dna harness v$nextv h:" .claude/agents/verifier.md || fail "init did not regenerate the harness files"
spec-dna check
gitc commit -q -am "build: upgrade spec-dna to $nextv"
use current
set +e
spec-dna check 2>"$work/pin.txt"
code=$?
set -e
[ "$code" = 3 ] || fail "the old binary ran against the new pin"
gitc revert --no-edit HEAD >/dev/null
spec-dna check
grep -q "<!-- spec-dna v$version h:" CLAUDE.md || fail "the rollback did not restore the adapters"

step "step 6: uninstall"
docs_before=$(gitc ls-files -s docs)
# The quickstart first turns the harness off with `init --reconfigure`.
printf 'harness = "none"\n' >"$work/none.toml"
spec-dna init --reconfigure --answers "$work/none.toml"
[ -e .claude/agents/verifier.md ] && fail "init --reconfigure did not remove the agent files"
[ -e REVIEW.md ] && fail "init --reconfigure did not remove REVIEW.md"
[ -e .claude/skills/spec-dna-intent/SKILL.md ] && fail "init --reconfigure did not remove the stage skills"
grep -q '^\[evals\]' spec-dna.toml || fail "init --reconfigure removed the [evals] table of the level none"
[ -f evals/check.sh ] || fail "init --reconfigure removed the files of the evals"
uninstall=$(block Uninstall | grep -v '^cargo uninstall')
[ -n "$uninstall" ] || fail "no sh block under \"## Uninstall\" in the quickstart"
eval "$uninstall"
# The text after the block: remove the folder of the evals too.
rm -r evals
[ "$(cat CLAUDE.md)" = "$before_claude" ] || fail "CLAUDE.md differs from the text before init"
gitc add -A
[ "$(gitc ls-files -s docs)" = "$docs_before" ] || fail "the uninstall changed a document"
gitc diff --cached --name-status | tee "$work/uninstall.txt"
# The uninstall removes the harness agent files, the stage skills, REVIEW.md
# and the files of the evals. `.claude/settings.json` stays, because it can
# hold keys of the project.
if grep -v -E '^D[[:space:]]+(spec-dna\.toml|REVIEW\.md|evals/.*|\.claude/skills/spec-dna(-[a-z]+)?/SKILL\.md|\.claude/agents/[A-Za-z-]+\.md)$|^M[[:space:]]+(CLAUDE\.md|\.claude/settings\.json)$' "$work/uninstall.txt"; then
    fail "the uninstall changed other files"
fi
grep -rl 'spec-dna' docs && fail "a document refers to spec-dna"
python3 -m unittest discover -s tests -t . 2>&1 | tail -n 1

step "ok"
