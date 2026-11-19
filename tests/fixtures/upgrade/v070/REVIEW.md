# Review policy

## Passes

Review the diff in 3 passes. For each finding, give the file and the line.

1. Bugs: a logic error, a missing check, a wrong edge case, and a test that does not prove the change.
2. Security: a leaked secret, an unsafe input, and an access that the change does not need.
3. Compliance: compare the diff with `spec.md` and `plan.md` of the change, and with the documents of the layer `rules`. Report a broken requirement, a delta line that the code does not meet, and a file that the `Files` of the plan do not list.

## Severity

- Important: a finding that breaks behavior, leaks data or breaches a policy.
- Nit: each other finding. Report at most 5 Nits in one review.

## Do not report

- A finding of `spec-dna check`. CI runs it.
- The text of a file that spec-dna generates: each file with a `spec-dna harness` marker, `.claude/skills/spec-dna/SKILL.md`, the spec-dna lines of `CLAUDE.md` and `AGENTS.md`, and the entries of `.claude/settings.json` for `model`, `modelSettings`, `fallbackModel`, `advisorModel`, `permissions.deny`, `env.CLAUDE_CODE_SUBAGENT_MODEL` and each `spec-dna hook` command.

spec-dna generates this file. Change `[harness]` in `spec-dna.toml` and run `spec-dna init --reconfigure`.

<!-- spec-dna harness v0.7.0 h:1648a4525739428b -->
