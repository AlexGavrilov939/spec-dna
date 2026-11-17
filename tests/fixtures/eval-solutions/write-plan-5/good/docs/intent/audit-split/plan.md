---
status: draft
---
# Split the audit code

## Files

- `src/audit.sh` The old audit code.
- `src/audit_write.sh` The writer.
- `src/audit_read.sh` The reader.
- `src/audit_prune.sh` The pruner.
- `src/stock.sh` The calls.
- `tests/test_audit.sh` The test of the writer and the reader.
- `tests/test_prune.sh` The test of the pruner.

## Order

- [ ] t1: Move the writer. Files: `src/audit.sh`, `src/audit_write.sh`. Verify: `sh tests/test_audit.sh`.
- [ ] t2: Move the reader. Files: `src/audit_read.sh`, `tests/test_audit.sh`. Verify: `sh tests/test_audit.sh`.
- [ ] t3: Write the pruner. Files: `src/audit_prune.sh`, `tests/test_prune.sh`. Verify: `sh tests/test_prune.sh`.
- [ ] t4: Call the new files. Files: `src/stock.sh`. Verify: `sh tests/test_stock.sh`.

## Risks

- A caller that sources `src/audit.sh` breaks, so t1 keeps a stub.

## Proof

- `sh tests/test_audit.sh` and `sh tests/test_prune.sh` pass.
