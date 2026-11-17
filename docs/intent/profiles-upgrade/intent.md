---
status: accepted
class: higher-risk
---
# Profiles at init and the upgrade from 0.6

## Intent

Ship slice 8 of [the design](../../design/0.7.0.md) (D37, D39 and D41) with the rules of [ADR 0023](../../adr/0023-default-skills-from-the-release-table.md). A new project gets the workflow of its profile from one answer. A project of 0.6.1 moves to 0.7.0 with one command, and a fixture that the 0.6.1 binary wrote proves it.

The change adds these items:

- `init` expands the profile into `[change] policy`, `gate` and `approval`, the skills, `review-md`, `protect-tests`, the evals and `protect-approvals` (D37, ADR 0018).
- One function gives the default of each skill from the table of the release and the constant `DEFAULT_ON_NEEDS_VERDICT_ON`. `init`, `upgrade`, `skill list` and SD507 use it.
- `upgrade` renames the skills of 0.6 to the new names that are on by default. Its report names each name that it leaves out, and the parts that `init --reconfigure` offers.
- `init` for `strict` prints a note on `spec-dna eval pack` and writes no pack.
- A fixture of 0.6.1 with its recorded `check`, and the tests of its upgrade.

## Why now

The table of the release comes from the full eval run, which runs during the build. The owner chooses the rule of the default after the run, so the code must make that choice one line. A release condition of 0.7.0 is an upgrade that works on a project of 0.6.1.

## Out of scope

- The value of the constant at the release and the rows of the table: the change `release-070`.
- The guide `docs/guides/upgrade-0.6.md`: the change `release-070`, because it names the skills that the verdicts turn on.
- The prompt of the approvers, SD926 and the hook `protect-approvals`: the change `approver-safety`.
- A new profile, a new key, or a preset in `upgrade` or in `--reconfigure` (ADR 0017, ADR 0023).
