---
status: draft
---
# Tree binary

## Delta

None. This change adds, changes or removes no requirement. It changes the rule of SD503 for a tree binary, and it adds the CI job `Self-check`. No interface document describes them: the rule is in `docs/diagnostics.md`.

## Interfaces

None. No interface document exists for the rule of SD503 or for CI.

## Decisions

- [[adr/0013-self-hosting]]

## Concerns

- [ ] SD503 reads paths only today. The new rule reads the diff of one file. The tests must prove that a pin change with another line still counts.

## Open questions

- [ ] None.
