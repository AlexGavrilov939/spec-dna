# Plan: list operations

The spec is `spec.md`.

## Order

- [x] t1: Add `add_item(items, item)` to `src/adding.py`. Add the test `test_add_item` in `tests/test_adding.py`. Files: `src/adding.py`, `tests/test_adding.py`. Verify: `sh test.sh -k add_item`.
- [x] t2: Add `remove_item(items, item)` to `src/removing.py`. Add the test `test_remove_item` in `tests/test_removing.py`. Files: `src/removing.py`, `tests/test_removing.py`. Verify: `sh test.sh -k remove_item`.
- [x] t3: Add `dedupe(items)` to `src/dedupe.py`. Add the test `test_dedupe` in `tests/test_dedupe.py`. Files: `src/dedupe.py`, `tests/test_dedupe.py`. Verify: `sh test.sh -k dedupe`.
