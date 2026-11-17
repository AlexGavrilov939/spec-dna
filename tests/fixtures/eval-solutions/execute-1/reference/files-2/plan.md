# Plan: text counts

The spec is `spec.md`.

## Order

- [x] t1: Add `count_words(text)` to `src/words.py`, with the test `test_count_words` in `tests/test_words.py`. Files: `src/words.py`, `tests/test_words.py`. Verify: `sh test.sh -k count_words`.
- [x] t2: Add `count_lines(text)` to `src/lines.py`, with the test `test_count_lines` in `tests/test_lines.py`. Files: `src/lines.py`, `tests/test_lines.py`. Verify: `sh test.sh -k count_lines`.
- [ ] t3: Add `summarize(text)` to `src/summary.py`, with the test `test_summarize` in `tests/test_summary.py`. Files: `src/summary.py`, `tests/test_summary.py`. Verify: `sh test.sh -k summarize`.
