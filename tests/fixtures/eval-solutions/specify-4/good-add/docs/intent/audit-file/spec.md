---
status: draft
---
# Keep the audit lines in their own file

## Delta

- ADD requirements/inventory#audit-file: The shelf service SHALL write the audit lines to the file `audit.log` of the data folder.

## Interfaces

- none

## Decisions

- [[decisions/flat-file-storage]]

## Concerns

- [ ] The file `audit.log` can grow without a limit.

## Open questions

- [ ] Does a clerk need a way to read the file `audit.log`?
