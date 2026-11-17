---
status: draft
class: higher-risk
---
# Limits of 0.6.1: index, area names and a plain move

## Intent

Close 3 known limits of 0.6.1, as slice 7 of [the design](../../design/0.7.0.md) says (D29 to D31, section 15) and [ADR 0015](../../adr/0015-limits-of-0-6-1.md) gives the reasons. D28 is done in slice 3a (`task-checks`).

The change adds these items:

- D29: `index` writes the first status of the type of its place, so it never writes a document that `check` rejects. The refresh of an index by `finish` uses the same rule. A place that gives no type stops `index` with exit code 2. The cause is proven by `failure-analyst` in a reproduction of 2026-11-17: `registry::render` writes no `status` line, and `index` never resolves the type of the place.
- D30: the configuration reader compares the names of `areas` in lowercase ASCII, and a pair that differs only in case is exit code 3. `new` and `mv` stop with exit code 2 for an area that differs from a name of `areas` only in case. `init` proposes one name of such a pair.
- D31: `log` and `stats --sdlc` read a rename of `intent.md` out of the intent folder as a move when the 3 conditions of D31 are true. A plain `git mv` thus no longer ends a change.

## Why now

These limits give a wrong result or a message with no fix. The person who meets them has no workaround except a hand edit. Slice 3b (`hours-and-links`) is merged, so `src/git.rs` and `src/sdlc.rs` are free for D31.

## Out of scope

- D28 (done in slice 3a).
- `layer add` with a `types` list on many lines (the sixth limit, ADR 0015).
- The text of the guard message of `mv`, `split` and `finish` for an error with an empty target. D29 no longer reaches it.
- A folder that holds only `intent.md`, with a near copy and the alias in the commit of `finish`. It still reads as a move (D31).
