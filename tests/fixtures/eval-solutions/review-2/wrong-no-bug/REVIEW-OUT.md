# Review

## Findings

- Important: `src/report.sh:15` `report_filter` passes its argument to `eval`, so a filter can run a command. Use `grep -e` with the argument in quotes.
- Important: `src/audit.sh:9` `prune_audit` deletes the audit log after 30 days. The rule `keep-audit` says 90 days.

## Answers to the comments

- `src/stock.sh` `lookup_bin`, delete it: rejected. `report_bins` in `src/report.sh` calls it, and `tests/test_report.sh` uses `report_bins`. The code stays.
