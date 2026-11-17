# Review

## Findings

- Important: `src/stock.sh:14` `stock_total` sums the third field with `$3`. The third field is the bin. The count is the second field.
- Important: `config/supplier.conf:4` holds the password of the supplier portal. The rule `no-secrets` forbids a password in the repository. Read it from the environment.

## Answers to the comments

- `src/stock.sh` `add_item`, remove the quotes: rejected. The rule `quote-variables` needs the quotes. With a folder name that has a space, the call breaks without them. The code stays.
