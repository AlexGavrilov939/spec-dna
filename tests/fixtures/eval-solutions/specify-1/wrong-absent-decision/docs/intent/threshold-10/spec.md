---
status: draft
---
# Raise the low-stock threshold to 10

## Delta

- CHANGE requirements/inventory#low-stock@17be391a70c17290: When the count of an item falls below 10, the shelf service SHALL list the item in the low-stock report.

## Interfaces

- none

## Decisions

- [[decisions/no-such]]

## Concerns

- [ ] The report may list many items after the change.

## Open questions

- [ ] Does a clerk need a way to see the old rule?
