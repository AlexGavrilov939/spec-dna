---
status: draft
---
# Keep the audit lines in their own file

## Delta

- CHANGE requirements/inventory#audit-log@4d6b9ea015d11715: When a clerk changes a count, the shelf service SHALL write one audit line with the clerk, the item, the old count and the new count to the file `audit.log` of the data folder.

## Interfaces

- none

## Decisions

- [[decisions/flat-file-storage]]
- The decision `sqlite-later` does not apply, because the project has not accepted it.

## Concerns

- [ ] The file `audit.log` can grow without a limit.

## Open questions

- [ ] Does a clerk need a way to read the file `audit.log`?
