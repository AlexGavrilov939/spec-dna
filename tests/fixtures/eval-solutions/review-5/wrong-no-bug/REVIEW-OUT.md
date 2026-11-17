# Review

## Findings

- Important: `src/adjust.sh:3` `adjust_count` writes the file `/tmp/items.tmp`. The name is the same for each user, so another user can plant a link there. Use `mktemp`.
- Important: `src/adjust.sh:8` `adjust_count` expands `$tmp` and `$STOCK_DIR` with no quotes. The rule `quote-variables` forbids it.

## Answers to the comments

- Reviewer 1, refuse a count that is not a whole number: accepted. I changed `add_item` so that it returns 1 and writes nothing for such a count.
- Reviewer 2, end each line with `\r\n`: rejected. The reader of the file (`read -r` in `low_stock`) would then keep the `\r` in the bin, and a compare would fail. The file uses LF, and the code stays.
