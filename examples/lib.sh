# Shared functions of the examples. Each run.sh sources this file.
#
# The output must be the same on each run, because tests/examples.rs compares
# it with the pages of the examples. Thus git ignores the configuration of the
# user, and each commit has a fixed author and a fixed date.

set -euo pipefail

# A git hook or a shell of the user can set these variables, and each one
# changes which repository or configuration git reads.
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_CONFIG_COUNT GIT_CONFIG_PARAMETERS
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1
export GIT_AUTHOR_NAME=example GIT_AUTHOR_EMAIL=example@example.org
export GIT_COMMITTER_NAME=example GIT_COMMITTER_EMAIL=example@example.org
export GIT_AUTHOR_DATE=2026-11-16T12:00:00Z GIT_COMMITTER_DATE=2026-11-16T12:00:00Z

# git reads GIT_CONFIG_GLOBAL from version 2.32. An older git reads the
# configuration of the user, for example commit.gpgsign.
git_version=$(git --version | sed -n 's/^git version \([0-9]*\)\.\([0-9]*\).*/\1 \2/p')
read -r git_major git_minor <<EOF
${git_version:-0 0}
EOF
if [ "$git_major" -lt 2 ] || { [ "$git_major" -eq 2 ] && [ "$git_minor" -lt 32 ]; }; then
  echo "example: the examples need git 2.32 or later; $(git --version)" >&2
  exit 1
fi

if ! command -v spec-dna >/dev/null; then
  echo 'example: spec-dna is not on PATH; install it first (README.md, section "Try it in 1 minute")' >&2
  exit 1
fi

# Makes an empty directory for the scratch repository and enters it. The
# argument names the directory. Without it, the function uses a new temporary
# directory. A directory that exists must be empty.
start() {
  work=${1:-$(mktemp -d)}
  mkdir -p "$work"
  if [ -n "$(ls -A "$work")" ]; then
    echo "example: $work is not empty" >&2
    exit 1
  fi
  cd "$work"
  echo "example: the scratch repository is $work"
}

# Prints a command after "$ ", runs it, and prints its output. The first
# argument is the exit code that the step must give. Another code stops the
# script. The second argument is the command, as one string.
run() {
  local want=$1 cmd=$2 got=0
  printf '\n$ %s\n' "$cmd"
  eval "$cmd" 2>&1 || got=$?
  if [ "$got" != "$want" ]; then
    echo "example: \`$cmd\` gave exit code $got, not $want" >&2
    exit 1
  fi
}

# Prints a line of explanation. The prefix "== " is not in the output of a
# command, so tests/examples.rs can find the end of a step.
say() {
  printf '\n== %s\n' "$*"
}
