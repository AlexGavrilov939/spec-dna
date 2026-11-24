---
status: accepted
class: higher-risk
---
# A new project has no advisor, and the advisor is consulted only in a higher-risk change

## Intent

`init` writes the role `advisor` of a new configuration as `none` in the profiles `lite` and `standard`, and as `fable` in `strict` (design of lifecycle and harness, D97). The role default of the code stays `fable`, so a file of 0.8.0 means the same in 0.8.1, and `upgrade` changes no project. Question 13 shows the default of the profile, and `--reconfigure` moves only a value that is the default of the old profile. Where the advisor is on, the model-team line of `CLAUDE.md` and the page `model-team.md` ask for a consult only in a change of class `higher-risk`, as a rule of text (D98). This repository gets the same rule in `CLAUDE.md` and in 3 files of `.claude/agents/`.

## Why now

The owner decided both points after protocol 2 of the model-team measurement: `lean` against `lean-no-advisor` gave -0.10 on the main measure at 1.49 times the cost, and no run showed a gain. The amendment of ADR 0004 of 2026-11-24 records the decision. Each request of a new project of `standard` thus carries 432 bytes of advisor sentences and the description of `fallback-advisor` (225 bytes) for no measured gain.

## Out of scope

- A gate that stops work until the advisor reads the changes (D99: not planned).
- A change of the role default of the code, or of the table of a current project by `upgrade`.
- A setting that turns off an advisor of the user settings (Q3 of the design).
- The text of a skill and of the agent file `acceptor.md`: they are frozen for the skill evals (`docs/evals.md`, section "The freeze of the skill texts for the reruns").
- The harness arms of the eval runner: each arm writes its own table, and the arm `lean` keeps the advisor.
