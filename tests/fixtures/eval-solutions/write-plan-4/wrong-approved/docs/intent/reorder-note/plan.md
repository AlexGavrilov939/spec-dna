---
status: approved
---
# Add a note to the reorder list

## Files

- `src/report.sh` The note line.
- `tests/test_report.sh` The test of the note.

## Order

- [ ] t1: Add the note line and its test. Files: `src/report.sh`, `tests/test_report.sh`. Verify: `sh tests/test_report.sh`.

## Risks

- The date format can differ between systems.

## Proof

- `sh tests/test_report.sh` passes.
