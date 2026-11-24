# Example: a first project

This example adds spec-dna to a small service with one function and one test. It writes an accepted decision and a requirement that cites the code. Then an agent renames the code and rewrites the decision, and `check` stops the change. Read it to see the core loop in 1 second, with no agent and no change process.

## Run it

```sh
examples/first-project/run.sh
```

You need `spec-dna` on your `PATH`, git 2.32 or later, bash and perl. [The list of examples](../README.md#run-an-example) gives the details.

## 1. Start spec-dna

The script answers 3 questions of `init` in a file. This example has no change process and no agent harness, so `init` writes only the configuration and the instruction lines for agents:

```toml
change-policy = "none"  # no change process in this first example
harness = "none"        # no agent harness: only the instruction lines
evals = "none"
```

```console
$ spec-dna init --answers ../answers.toml
spec-dna init: greenfield repository; profile standard; 1 root (see spec-dna.toml)
wrote spec-dna.toml
wrote AGENTS.md
wrote CLAUDE.md
wrote .claude/skills/spec-dna/SKILL.md
...
```

`AGENTS.md` and `CLAUDE.md` get one line each. The line tells an agent to run `spec-dna context` before an edit and `spec-dna check` before the work is complete. Without `--answers`, `init` asks its questions and proposes the change process and the Claude Code harness ([docs/init.md](../../docs/init.md)).

## 2. Write a requirement and a decision

The place of a file gives its type. `docs/requirements/orders.md` is a `requirements` document with the id `requirements/orders`:

```md
- **cancel-window**: The service SHALL accept a cancellation only in the first 30 minutes after the order. The check is in `src/orders.py` (`cancel_order()`).
```

The requirement has the id `requirements/orders#cancel-window`. The test links to it with a comment:

```python
# covers: requirements/orders#cancel-window
def test_cancel_after_31_minutes_fails():
```

`docs/decisions/storage.md` is a decision with `status: accepted`. Its text is now locked:

```md
Chosen option: PostgreSQL, because all 3 hosts must read the same orders.
```

## 3. Give an agent the context of a file

Before an agent edits `src/orders.py`, it asks for the sections that apply to the file. spec-dna gives the 2 sections of the document that cites the file, and no other document:

```console
$ spec-dna context src/orders.py
<!-- spec-dna context 0.6.1: src/orders.py; 2 sections in full, 0 listed; 354 of 8000 characters; about 89 tokens (estimated) -->

<!-- requirements/orders#requirements h:c70424c66f956daa requirements/active cites -->
## Requirements

- **cancel-window**: The service SHALL accept a cancellation only in the first 30 minutes after the order. The check is in `src/orders.py` (`cancel_order()`).

<!-- requirements/orders#orders h:ea12bbb19fb132f0 requirements/active cites -->
# Orders
```

The first comment line gives the size of the result and the budget, 8000 characters by default. The comment above a section gives its id, its hash, its type, its status and the reason for the selection.

## 4. Stop a bad change

The agent renames `src/orders.py` to `src/cancel.py` and `cancel_order` to `cancel`. It also rewrites the outcome of the accepted decision to match a new idea:

```console
$ spec-dna check
docs/decisions/storage.md:1: error SD405 decision-locked: the text outside the amendment and live sections changed after the decision was accepted
docs/requirements/orders.md:8: warning SD606 cited-path-removed: `src/orders.py` was removed in this change (renamed to `src/cancel.py`), but this document still cites it
docs/requirements/orders.md:8: warning SD607 cited-symbol-removed: `cancel_order` was removed from `src/cancel.py` in this change, but this document still cites it
...
spec-dna check: 2 documents; 1 new error and 2 new warnings against b32abef70b97 (1 error and 2 warnings in total)
```

`check` exits with code 1, so a commit hook or CI stops the change. The output also gives the fix of each code.

## 5. Fix the change

The script restores the accepted text and writes the new fact in the amendment section:

```md
- 2026-11-16: The function `cancel()` in `src/cancel.py` checks the cancel window.
```

It also updates the citation in the requirement. Then `check` passes:

```console
$ spec-dna check
spec-dna check: 2 documents; 0 new errors and 0 new warnings against b32abef70b97 (0 errors and 0 warnings in total)
```

`show` prints the document with the tests of each requirement:

```console
$ spec-dna show requirements/orders
...
<!-- evidence -->
- requirements/orders#cancel-window: tests/test_orders.py:1 [test test_cancel_after_31_minutes_fails]
```

## If it fails

| Message | Cause | Fix |
| --- | --- | --- |
| `example: spec-dna is not on PATH` | The binary is not installed | Install it ([README](../../README.md#try-it-in-1-minute)) |
| `example: <dir> is not empty` | The directory that you gave has files | Give an empty or a new directory |
| The output differs from this page | Your binary has another version (`spec-dna --version`) | Install the current release |
