---
type: decision
status: accepted
---
# Flat daily fine

## Context and Problem Statement

A late return must cost the member something. The staff at the desk must be
able to explain the rule in one sentence.

## Considered Options

1. A flat fine for each late day.
2. A fine that grows each week.
3. No fine, but a block on new loans.

## Decision Outcome

Chosen option: 1, a flat fine of 25 cents for each late day, without a cap.
`fine_for()` in `library/loans.py` computes it. The requirement is
[daily-fine](../loans.md#daily-fine).
