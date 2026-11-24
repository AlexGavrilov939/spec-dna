---
status: approved
---
# The eval runner counts each segment, runs one arm alone, and keeps a floor of free space: spec

## Delta

None. This change adds, changes or removes no requirement, because this repository has no layer of requirements. It changes the arguments of `claude` (`--output-format stream-json --verbose`), the meaning of the key `turns`, and adds the key `segments` to each result of `eval run`, `--ab` and `--arms`. It adds the configuration key `[evals] min-free-mib`, the stop `disk-space` with exit code 3, the form of `--arms` with 1 arm, and the form of `eval verdict --compare` with more than one file, with exit code 3 for a run in 2 files. The [design](../../design/0.8.1-a-lifecycle-and-harness.md) states the contract in D95 and D100, and [ADR 0030](../../adr/0030-harness-arms-time-record-and-verdict-floor.md) holds the amendment of 2026-11-24.

## Interfaces

- None: no interface document describes these commands.

## Decisions

- [[adr/0008-harness-and-evals]]
- [[adr/0030-harness-arms-time-record-and-verdict-floor]]

## Concerns

- [x] The reader of D95 item 2 keeps a line only when it has the bytes `"type":"result"`. The stubs of `bench/team/` and `bench/team2/` write JSON with a space after each colon, and the self-test of `bench/team/` runs in `cargo test`. Resolved: the reader parses each line that holds the bytes `"result"` and keeps the event only when its top-level `type` is `result`. Each `result` event holds these bytes, with or with no space, so the filter misses no event. The memory holds the `result` events only, as D95 asks, and no file of `bench/` changes.
- [x] The stub of `tests/eval_run.rs` that prints a JSON array of events is no form of the stream. Resolved: it prints the same 2 events on 2 lines, and a stub that prints one compact object stays, because one line is a stream of one event.
- [x] A result that `--resume` reads from a file of 0.8.0 has no key `segments`. Resolved: the runner writes `segments` as `null` for it, as it writes `turns` as `null` for an unknown count.
- [x] Each test that runs `eval run` reads the free space of the machine, so a machine with less than 2,048 MiB free in its temporary folder stops these tests. Resolved: that is the rule of each machine, and the message names the key. The tests of the floor put a stub `df` on `PATH`, so they do not depend on the machine.
- [x] The slice of the design does not list `src/main.rs`, `src/init.rs` and `src/eval_pack.rs`. Resolved: `src/main.rs` holds the help text of `eval` and the arguments of `--compare`. A new field of the table `[evals]` changes each literal of the table, in `src/init.rs` and in the unit tests of `src/evals.rs` and `src/eval_pack.rs`, so the plan first gives the table one function with the default values.
- [x] `--max-turns` probably limits each segment, so `turns` can be more than `max-turns`. Resolved: the docs state it, and `segments` shows when it can occur (D95 item 6).

## Open questions

- [x] Does the owner approve one repro run of `claude -p` with a background task, `--max-turns 3` and `--output-format stream-json --verbose`, with its stream saved and `--max-budget-usd 2` (Q6 of the design)? The design recommends it before this change. The stub of the tests pins the assumed shape of the stream: one `result` event for each segment. A run that shows another shape changes the reader before the approval of the spec. Answer (the task of this session forbids eval spend): no repro run in 0.8.1 part A. The stub pins the assumed shape of the stream, and the real shape is checked by one run of at most 2 USD in part B, with the go-ahead of the owner.
