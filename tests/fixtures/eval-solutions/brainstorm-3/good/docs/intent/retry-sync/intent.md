---
status: draft
class: routine
---
# Retry a failed call of the nightly sync

## Intent

The nightly sync tries a failed call to the supplier 3 times before it fails.

## Why now

The sync fails when the network drops once, and the clerks then start the next day with old data.

## Out of scope

- The supplier format.
