---
status: draft
---
# Raise the low-stock threshold to 10

## Delta

- CHANGE requirements/inventory#low-stock@17be391a70c17290: When the count of an item falls below 10, the shelf service SHALL list the item in the low-stock report.

## Interfaces

- none

## Decisions

- none

## Concerns

- [x] Does the new threshold apply to all bins, or only to bin A? Answer: it applies to every bin.
- [x] Should the report show the old threshold for one release? Answer: no, the report shows only the new threshold.

## Open questions

- [x] Which count does the report use for an item that is in 2 bins? Answer: the report adds the counts of the 2 bins.
