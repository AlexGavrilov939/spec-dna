#!/bin/sh
# Run a command in a scrubbed environment, for a paid batch.
#
# Usage: run_scrubbed.sh <tools folder> <working folder> <command> [<argument>...]
#
# The environment has only what Claude Code needs to start and to find its login
# (HOME and CLAUDE_CONFIG_DIR). It has no CLAUDE_CODE_* variable, no
# CLAUDE_EFFORT and no CLAUDE_CODE_SUBAGENT_MODEL, so no variable of the host
# session changes the model, the effort, the plugins or the subagents of a run.
# PATH has the folder <tools folder>/bin first and no folder of the project.
# ZDOTDIR names a folder of startup files that only set PATH, so the shell of the
# agent reads no startup file of the user, and no older binary of spec-dna comes
# first on PATH.
set -eu
tools="$1"
workdir="$2"
shift 2
cd "$workdir"
exec env -i \
  HOME="$HOME" \
  USER="${USER:-user}" \
  LOGNAME="${USER:-user}" \
  LANG="en_US.UTF-8" \
  TERM="dumb" \
  TMPDIR="${TMPDIR:-/tmp}" \
  CLAUDE_CONFIG_DIR="${CLAUDE_CONFIG_DIR:?CLAUDE_CONFIG_DIR is not set}" \
  SHELL="/bin/zsh" \
  ZDOTDIR="$tools/zdot" \
  PATH="$tools/bin:/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin" \
  "$@"
