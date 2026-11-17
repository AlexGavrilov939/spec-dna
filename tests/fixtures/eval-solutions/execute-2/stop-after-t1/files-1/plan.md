# Plan: table rows

The tasks have no other spec.

## Order

- [x] t1: Add `title_row(cells)` to `src/header.py`. It returns the cells in upper case, joined by ` | `. Add the test `test_title_row` in `tests/test_header.py`. Files: `src/header.py`, `tests/test_header.py`. Verify: `sh test.sh -k title_row`.
- [ ] t2: Add `format_row(cells, widths)` to `src/table.py`. It pads each cell to its width with `pad_right` of `src/text.py`, and joins the cells with a space. Add the test `test_format_row` in `tests/test_table.py`. Files: `src/table.py`, `tests/test_table.py`. Verify: `sh test.sh -k format_row`.
- [ ] t3: Add `pad_right(text, width)` to `src/text.py`. It pads the text on the right with spaces. Add the test `test_pad_right` in `tests/test_text.py`. Files: `src/text.py`, `tests/test_text.py`. Verify: `sh test.sh -k pad_right`.
