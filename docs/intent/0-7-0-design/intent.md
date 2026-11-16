---
status: draft
class: higher-risk
---
# 0.7.0 design

## Intent

Design spec-dna 0.7.0 as one tool for the whole change. It keeps one deterministic core. It adds a workflow pack of 11 skills on top. Each skill binds to an artifact and to the checks of the core. It also adds 5 other parts:

- A lighter approval rule.
- A tree binary for the repository of spec-dna itself.
- An import from Spec Kit and OpenSpec.
- The fixes of the known limits of 0.6.1.
- Profiles.

The work is design only: [the design page](../../design/0.7.0.md) and ADRs 0010 to 0017. No product code changes.

## Why now

A change of 0.6.1 needed 4 pull requests. The skills that `next` names since 0.6.0 do not exist yet. Superpowers, Spec Kit and OpenSpec cover the stages of a change, but none of them controls a corpus over a long time. spec-dna has that control, and it needs the workflow on top to be one tool.

## Out of scope

- Product code, tests and the release of 0.7.0.
- The eval run that decides which skills are on by default. The owner pays for it after the design is accepted.
- A change to the locked text of an accepted ADR. The new ADRs amend them.
