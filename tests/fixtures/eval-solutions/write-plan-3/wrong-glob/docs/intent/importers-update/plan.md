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

- [ ] t1: Quote the variables of the importers. Files: `src/importers/*`. Verify: `sh tests/test_stock.sh`.

## Risks

- A quoted empty value now counts as an argument.

## Proof

- `sh tests/test_stock.sh` passes.
