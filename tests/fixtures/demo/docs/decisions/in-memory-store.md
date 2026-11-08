---
type: decision
status: accepted
---
# In-memory store

## Context and Problem Statement

The service is small, and its first users run it on one machine.

## Decision Outcome

Chosen option: keep all state in one `Store` object in memory
(`library/store.py`). A restart loses the data. A database comes with the
first production use.
