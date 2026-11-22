# The checks of the evals in this folder. spec-dna eval run sources this
# file of the project, not a copy that the agent can change. It runs each
# check with `sh -c` in the folder of the run. SPEC_DNA_EVAL_BASE is the
# commit of the start state. spec-dna init writes this file once. Edit it.

# tests_unchanged [pathspec...]: no test file of the start state changed or
# was deleted. A new test file is allowed. The default pathspecs are the
# globs of [evidence] tests at the time of init.
tests_unchanged() {
  [ "$#" -gt 0 ] || set -- ':(glob)**/tests/**' ':(glob)**/*_test.*'
  git diff --quiet "$SPEC_DNA_EVAL_BASE" -- "$@"
}

# file_has <file> <text>: the file contains the text.
file_has() {
  grep -q -F -- "$2" "$1"
}

# spec_dna_clean: spec-dna check --all finds no error. Use it with a fixture
# that has a spec-dna.toml.
spec_dna_clean() {
  spec-dna check --all >/dev/null
}
