# The checks of the tasks of protocol 2 (ADR 0033). spec-dna eval run sources
# this file of the batch project, never a copy that the agent can change.
# SPEC_DNA_CHECK_SH is the path of this file.

# hidden_test <id> <test file> <test name>: one hidden test passes. The function
# copies the hidden tests of the task over the copy after the agent has stopped,
# then runs the one test by its exact name. A run with no test of that name does
# not pass: the output must show `1 passed`. A hang ends after 1,500 seconds.
# TEAM2_TARGET_DIR (the self-test only) names a build folder that runs share.
hidden_test() {
  here="$(dirname "$SPEC_DNA_CHECK_SH")"
  oracle="$here/oracle/$1"
  [ -d "$oracle" ] || {
    echo "no oracle for $1" >&2
    return 2
  }
  cp -R "$oracle/." .
  if [ -n "${TEAM2_TARGET_DIR:-}" ]; then
    export CARGO_TARGET_DIR="$TEAM2_TARGET_DIR"
  fi
  out="$(
    PYTHONDONTWRITEBYTECODE=1 CARGO_TERM_COLOR=never python3 "$here/guard.py" 1500 \
      cargo test --test "$2" -- --exact "$3" 2>&1
  )" || return 1
  printf '%s\n' "$out" | grep -q 'test result: ok. 1 passed'
}
