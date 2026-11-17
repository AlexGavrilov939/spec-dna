# Plan: text styles

The tasks have no other spec.

## Order

- [x] t1: Add `upper_first(text)` to `src/caps.py`. It returns the text with its first letter in upper case. Add the test `test_upper_first` in `tests/test_caps.py`. Files: `src/caps.py`, `tests/test_caps.py`. Verify: `sh test.sh -k upper_first`.
- [x] t2: Add `title_case(text)` to `src/titles.py`. It applies `upper_first` to each word. Add the test `test_title_case` in `tests/test_titles.py`. Files: `src/titles.py`, `tests/test_titles.py`. Verify: `sh test.sh -k title_case`.
- [ ] t3: Add `shout(text)` to `src/shout.py`. It returns the text in upper case, followed by `!`. Add the test `test_shout` in `tests/test_shout.py`. Files: `src/shout.py`, `tests/test_shout.py`. Verify: `sh test.sh`.
