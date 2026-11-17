---
status: draft
---
# Export the low-stock report as CSV

## Files

- `src/export.sh` The CSV writer.
- `tests/test_export.sh` The test of the export.
- `src/report.sh` The option `--csv`.

## Order

- [ ] t1: Write the CSV writer and its test. Files: `src/export.sh`, `tests/test_export.sh`. Verify: `sh tests/test_export.sh`.
- [ ] t2: Add the option `--csv` to the report. Files: `src/report.sh`. Verify: `sh tests/test_export.sh`.

## Risks

- A name with a comma breaks a CSV line, so the writer quotes it.

## Proof

- `sh tests/test_export.sh`.
