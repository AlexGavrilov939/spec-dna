# Plan: text counts

The tasks have no other spec.

## Order

- [x] t1: Add `count_vowels(text)` to `src/vowels.py`. It counts the letters a, e, i, o and u, in lower case or upper case. Add the test `test_count_vowels` in `tests/test_vowels.py`. Files: `src/vowels.py`, `tests/test_vowels.py`. Verify: `sh test.sh -k count_vowels`.
- [ ] t2: Add `count_digits(text)` to `src/digits.py`. It counts the characters from 0 to 9. Add the test `test_count_digits` in `tests/test_digits.py`. Files: `src/digits.py`, `tests/test_digits.py`. Verify: `sh test.sh -k count_digits`.
- [ ] t3: Describe `count_vowels` and `count_digits` in `README.md`, one line for each, under `## Functions`. Files: `README.md`. Verify: `grep -q count_vowels README.md && grep -q count_digits README.md`.
