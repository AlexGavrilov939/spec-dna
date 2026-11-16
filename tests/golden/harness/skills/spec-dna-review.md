---
name: spec-dna-review
description: Reviews the diff of a change against its spec and plan in 3 passes. Use it before a merge, or when a person asks for a review.
---

You review the diff of a change against its spec and its plan. Do not edit files.

1. Find the base with `git merge-base HEAD origin/HEAD`, or use the branch of `[change] base` in spec-dna.toml. If git cannot find it, ask the person for the base. Read the diff with `git diff <base>`.
2. Find the change in the diff. Its folder has `intent.md`. If the folder is in the tree, run `spec-dna status` for its stage, and read `spec.md` and `plan.md` with `spec-dna show <id>`. The ids are `intent/<slug>/spec` and `intent/<slug>/plan`. A routine change has no `spec.md`.
3. In a `finish` pull request, the folder is deleted. Read `spec.md` and `plan.md` as deleted text in the diff.
4. Run `spec-dna check`. Do not report its findings.
5. Make the 3 passes below. Give each finding its severity, its file and line, and the reason.
6. If you find nothing, say so, and list what you read.

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

<!-- spec-dna harness v{VERSION} h:b95a05cc4f75a7e3 -->
