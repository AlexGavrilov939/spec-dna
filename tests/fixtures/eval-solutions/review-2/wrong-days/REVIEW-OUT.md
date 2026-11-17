# Review

## Findings

- Important: `src/stock.sh:18` `count_bin` uses `grep -c ",$1"`, which also counts the bin `A10` when the bin is `A1`. Match the whole field.
- Important: `src/report.sh:15` `report_filter` passes its argument to `eval`, so a filter can run a command. Use `grep -e` with the argument in quotes.
- Note: `prune_audit` looks fine. The file has 300 lines in 2030 places.

## Answers to the comments

- `src/stock.sh` `lookup_bin`, delete it: rejected. `report_bins` in `src/report.sh` calls it, and `tests/test_report.sh` uses `report_bins`. The code stays.
