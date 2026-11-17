# Review

## Findings

- Important: `src/stock.sh:18` `count_bin` uses `grep -c ",$1"`, which also counts the bin `A10` when the bin is `A1`. Match the whole field.
- Important: `src/audit.sh:9` `prune_audit` deletes the audit log after 30 days. The rule `keep-audit` says 90 days.

## Answers to the comments

- `src/stock.sh` `lookup_bin`, delete it: rejected. `report_bins` in `src/report.sh` calls it, and `tests/test_report.sh` uses `report_bins`. The code stays.
