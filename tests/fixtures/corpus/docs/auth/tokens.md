---
status: deprecated
supersedes: [auth/legacy]
---
# Tokens

## Requirements

- **signing**: The issuer SHALL sign every token with Ed25519.
- **lifetime**: If a token is older than 15 minutes, the gateway SHALL reject it.
