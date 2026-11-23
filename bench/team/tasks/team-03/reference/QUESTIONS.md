# Questions

## Rounding of the fee

- Request: round the fee half up.
- Rule: M-2 in `docs/rules/money.md` says that money rounds half to even.
- Done: `fee_cents` rounds half to even, as the rule says.
- Question: do you want the rule to change for fees?
