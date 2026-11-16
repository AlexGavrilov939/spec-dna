---
status: accepted
class: higher-risk
---
# Tree binary

## Intent

Add the tree binary of [ADR 0013](../../adr/0013-self-hosting.md) (decisions D18 to D20 of [the design](../../design/0.7.0.md)). A binary that the checkout of spec-dna builds finds this fact from `Cargo.toml`, with no new key. In this mode, SD503 does not count a change of the pin line of `spec-dna.toml`. Also add a CI job that builds the tree binary and runs `check` on this repository.

## Why now

No pull request checks the documents of this repository today. The release of 0.6.1 needed a pull request of its own for the pin. Each later slice of 0.7.0 then uses the self-check.

## Out of scope

- The approval in the branch (slice 2).
- A new configuration key.
- A change to the exact pin rule for each other rule: a pin that differs from the version still stops `check`, `finish` and `eval run`.
