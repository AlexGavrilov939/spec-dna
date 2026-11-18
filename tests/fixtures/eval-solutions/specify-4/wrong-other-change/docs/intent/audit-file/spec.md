---
status: draft
---
# Keep the audit lines in their own file

## Delta

- CHANGE requirements/inventory#add-item@f8d576d06906dec8: When a clerk adds an item, the shelf service SHALL store its name, its count and its bin, and SHALL write nothing to the file `audit.log`.

## Interfaces

- none

## Decisions

- [[decisions/flat-file-storage]]
- The decision `sqlite-later` does not apply, because the project has not accepted it.

## Concerns

- [ ] The file `audit.log` can grow without a limit.

## Open questions

- [ ] Does a clerk need a way to read the file `audit.log`?
