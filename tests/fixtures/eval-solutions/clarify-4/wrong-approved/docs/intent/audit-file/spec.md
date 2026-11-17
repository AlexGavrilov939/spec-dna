---
status: approved
---
# Keep the audit lines in their own file

## Delta

- ADD requirements/inventory#audit-file: The shelf service SHALL write the audit lines to the file `audit.log` of the data folder.

## Interfaces

- none

## Decisions

- [[decisions/flat-file-storage]]

## Concerns

- [x] An old audit line may stay in the file of the items. Answer: the tool leaves an old audit line where it is, and a note in the release text says so.

## Open questions

- [x] Which folder holds the file `audit.log`? Answer: the folder `data/`.
