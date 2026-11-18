---
status: draft
class: routine
---
# Keep the items of the report in memory

## Intent

The report keeps the items in memory for 5 minutes. It does not change the CSV file.

## Why now

The report reads the whole CSV file on each call, and a clerk asked for a faster way.

## Out of scope

- The CSV file.
