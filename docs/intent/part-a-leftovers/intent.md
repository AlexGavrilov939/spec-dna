---
status: accepted
class: higher-risk
---
# The open points of part A of 0.8.1

## Intent

This change closes the 7 open points of part A of 0.8.1 with the smallest fix for each:

- The team page of a `lite` project with no advisor names no step on the advisor.
- `init --reconfigure` with a new profile keeps the role `advisor` of the file, so a value `none` that a person set does not become `fable` in `strict`.
- `init --reconfigure` that turns the advisor off removes `advisorModel` from `.claude/settings.json` when the key has the model of the old configuration.
- A plan that a commit added before the pin 0.8.0 and that repeats the id of a deviation gets its own code, SD942. `next` and `accept` stop before the acceptor on it, and `next` names the problem of a record that a commit refused.
- SD646 is a warning in `strict`, so `finish` does not stop after a change removes the last requirement of an area.
- The retrieval benchmark of the synthetic corpora has the numbers of the current binary.
- The test that copies `scripts/verify-release.sh` runs the copy with `bash`, so no test fails with "Text file busy".

## Why now

Part A of 0.8.1 left these points open, and the release candidate of 0.8.1 must not have them. A new `lite` project tells a person to run `/advisor`, which it does not have. A switch of the profile or of the advisor gives a team that the configuration does not describe. An acceptor run of an upgraded project fails at its commit, and `next` then sends the acceptor to the same failure. In `strict`, no edit clears SD646 after the last requirement of an area goes. The reference file of the retrieval benchmark is from the first commit of the repository. 3 of the 6 failed test jobs since pull request #91 are the same race.

## Out of scope

- The key `advisor` of the entry `acceptor`. No question of `init` asks for it, so a new profile still moves its default value, as `docs/config.md` states.
- The other settings keys that a new configuration no longer sets: they stay, with their `kept` line, as before.
- A repeated item of `Invariants` in a spec. No rule reads it, but no lab run showed a failure from it.
- The job "Run aarch64-apple-darwin" that GitHub cancelled with 0 steps ("The job was not acquired by Runner of type hosted"). The cause is runner capacity, and no file of the repository can fix it.
- The retrieval numbers of corpus B and corpus D. Their tables name the binary of their measurement.
