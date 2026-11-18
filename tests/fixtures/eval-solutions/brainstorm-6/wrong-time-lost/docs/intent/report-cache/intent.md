---
status: draft
class: routine
---
# Keep the items of the report in memory

## Summary

The report keeps the items in memory for a short time, so that it does not read the CSV file on each call.

## Motivation

The report reads the whole CSV file on each call, and a clerk asked for a faster way.

## Proposal

The report keeps the items in memory for a short time. After that time, it reads the CSV file again. The CSV file does not change.

## Alternatives

- Read the CSV file on each call, as now. This is slow.

## Unresolved

- Does a clerk need to see a new item before the a short time end?
