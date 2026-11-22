# The work of an acceptor in the evals `accept-1` to `accept-5` (design 0.8.0, D62).
# A scenario runs this file with `SLUG=<slug> . accept-solve.sh`. It writes the
# real skeleton with `spec-dna accept`, fills it from `$SOL/record.tpl`, and
# commits the record alone. A record that `spec-dna check` finds invalid (SD928)
# fails the scenario, so the records of the folder are valid records, and a
# wrong record is wrong for the oracle only. A scenario with `FORM=1` has a
# record in a form that format 2 does not allow (a word of 0.7.1, a path with
# `./`): the oracle must not fail it for the form, so the check is off.
spec-dna accept "$SLUG" --session s1 || exit 1
python3 "$SOL/../../accept-fill.py" "$SOL/record.tpl" "docs/intent/$SLUG/acceptance.toml" || exit 1
git add "docs/intent/$SLUG/acceptance.toml"
git commit -q --no-verify -m "Record the acceptance of $SLUG" || exit 1
if [ -z "$FORM" ] && spec-dna check --base main | grep -q 'SD928'; then
  echo "the record of $SLUG is invalid" >&2
  exit 1
fi
true
