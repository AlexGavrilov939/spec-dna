---
depends-on: [auth/tokens]
verified-by: [src/gateway.rs, tests/session.rs]
owner: team-auth
---
# Sessions

A session keeps a user signed in between requests. The store is chosen in ADR-1.

## Requirements

- **expiry**: When a session expires, the gateway SHALL reject the request with status 401.
- **renewal**: The session SHALL be renewed quickly, if possible.
- **R-7**: While a session is active, the gateway SHALL refresh its expiry on each request.
- **Bad_Key**: The gateway MUST log the session id.

## Rationale

Short sessions limit the damage of a stolen cookie. See [the token rules](tokens.md#requirements) and [a missing anchor](tokens.md#nowhere).
