# Review

## Findings

- Important: `src/stock.sh:9` `init_store` runs `chmod 777` on the data folder. Each user can then write the items and the audit log. Use `chmod 750`.
- Important: `config/supplier.conf:4` holds the password of the supplier portal. The rule `no-secrets` forbids a password in the repository. Read it from the environment.

## Answers to the comments

- `src/stock.sh` `add_item`, remove the quotes: rejected. The rule `quote-variables` needs the quotes. With a folder name that has a space, the call breaks without them. The code stays.
