---
status: draft
---
# Quote the variables of the importers

## Files

- `src/importers/a.sh` Quote the variables.
- `src/importers/b.sh` Quote the variables.
- `src/importers/c.sh` Quote the variables.
- `src/importers/d.sh` Quote the variables.

## Order

- [ ] t1: Quote the variables of the importers a and b. Files: `src/importers/a.sh`, `src/importers/b.sh`. Verify: `grep -c '"\$1"' src/importers/a.sh src/importers/b.sh`.
- [ ] t2: Quote the variables of the importers c and d. Files: `src/importers/c.sh`, `src/importers/d.sh`. Verify: `grep -c '"\$1"' src/importers/c.sh src/importers/d.sh`.

## Risks

- A quoted empty value now counts as an argument.

## Proof

- `sh tests/test_stock.sh` passes.
