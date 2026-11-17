---
status: draft
class: routine
from: docs/incidents/stock-sync-dup.md
---
# Make the nightly sync idempotent

## Intent

A second call of the nightly sync with the same file changes no count.

## Why now

The sync ran twice on 2 March, and the counts of 40 items doubled.

## Out of scope

- A change of the format of the supplier file.
