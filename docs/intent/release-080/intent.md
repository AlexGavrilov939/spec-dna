---
status: draft
class: higher-risk
---
# Release of 0.8.0: the last owner answers and the tag

## Intent

Apply the answers of the owner of 2026-11-23 to the release candidate, and release 0.8.0. The acceptance pass of the agent `acceptor` gets a key that sets its advisor (design D81). The reruns of `write-plan` and `finish` reach the floor of 15 pairs, and the table of the release follows. The CI workflow of the evals installs Claude Code 2.1.330. The rows of `ownership.md` for the new files, the paragraph on the acceptance record in `AGENTS.md` and the agent file `acceptor` of this repository come in. ADR 0032 is accepted. The notes of the release state what the evidence does not prove. A tag `v0.8.0` follows when each check of the release passes.

## Why now

The release candidate is merged and has no tag. The owner has answered Q51 to Q55, and the tag waits for these answers.

## Out of scope

- The review mode and the persons of `[owners]`, and the other work of 0.8.1 (issues 109 to 113).
- A new rerun of `execute` or `review`.
- A hard switch of the advisor for one subagent. Claude Code has none.
