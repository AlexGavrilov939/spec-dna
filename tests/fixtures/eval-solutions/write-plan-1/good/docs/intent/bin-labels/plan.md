---
status: draft
---
# Print a label for each bin

## Files

- `src/label.sh` The new label function.
- `src/stock.sh` The new function `bin_items`.
- `src/report.sh` The new option `--label`.
- `tests/test_label.sh` The new test.
- `docs/requirements/inventory.md` The new requirement `label-bin`.

## Order

- [ ] t1: Add the function `bin_items`. Files: `src/stock.sh`. Verify: `sh tests/test_stock.sh`.
- [ ] t2: Write the label function and its test. Files: `src/label.sh`, `tests/test_label.sh`. Verify: `sh tests/test_label.sh`.
- [ ] t3: Add the option `--label` and the requirement. Files: `src/report.sh`, `docs/requirements/inventory.md`. Verify: `spec-dna check`.

## Risks

- A bin with no item gives an empty label.

## Proof

- `sh tests/test_label.sh` covers requirements/inventory#label-bin.
