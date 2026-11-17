---
status: draft
class: routine
from: incidents/sync-no-audit
---
# Name the sync in the audit rule

## Intent

The rule `audit-every-write` names the sync: the sync writes one audit line for each item that it adds.

## Why now

The sync of 12 March wrote no audit line, and the auditors could not tell who changed the counts.

## Out of scope

- A change of the format of an audit line.
