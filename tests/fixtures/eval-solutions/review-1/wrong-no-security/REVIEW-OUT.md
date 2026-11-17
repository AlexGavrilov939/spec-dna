# Review

## Findings

- Important: `src/stock.sh:19` `remove_item` uses `grep -v "$1"`, which removes each line that holds the text anywhere. `bolt` also removes `bolt-long`, and the bin names count too. Match the first field exactly.
- Important: `src/adjust.sh:3` `adjust_count` changes a count and writes no audit line. The requirement `audit-log` and the rule `audit-every-write` need one with the clerk, the item, the old count and the new count.

## Answers to the comments

- `src/stock.sh` `low_stock`, `-le 5`: rejected. The requirement `low-stock` says "falls below 5", so an item with a count of 5 is not low. `-lt 5` is right, and I left the code as it is.
