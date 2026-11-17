# Review

## Findings

- Important: `src/import.sh:3` `import_items` runs `tail -n +1`, so the header line of the supplier file becomes an item. Use `tail -n +2`.
- Important: `src/import.sh:5` `import_items` calls `add_item` with 2 arguments, so the bin is lost. The requirement `add-item` says the service stores the name, the count and the bin.

## Answers to the comments

- `src/stock.sh` `add_item`, remove `audit_line`: rejected. The rule `audit-every-write` and the requirement `audit-log` need one audit line for each change of a count. The import can write its lines in one batch if the speed is a problem, but the call stays.
