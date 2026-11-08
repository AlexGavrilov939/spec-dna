---
type: contract
status: active
---
# HTTP API

The service serves JSON over HTTP. `handle()` in `library/api.py` maps each
request to a status and a body. An error has the body
`{"error": <code>, "message": <text>}`.

## Endpoints

- **list-books**: When a client sends `GET /books`, the service SHALL return status 200 and each book with its number of available copies.
- **get-book**: When a client sends `GET /books/{isbn}`, the service SHALL return status 200 and the book.
- **create-loan**: When a client sends `POST /loans` with `member_id` and `isbn`, the service SHALL return status 201 and the new loan.
- **return-loan**: When a client sends `POST /loans/{id}/return`, the service SHALL return status 200 and the returned loan.
- **renew-loan**: When a client sends `POST /loans/{id}/renew`, the service SHALL return status 200 and the renewed loan.
- **member-loans**: When a client sends `GET /members/{id}/loans`, the service SHALL return status 200 and the active loans of the member.

## Errors

- **bad-request**: If a request body is not a JSON object or misses a required field, then the service SHALL return status 400 with the error `bad_request`.
- **not-found**: If a member, a book or a loan does not exist, then the service SHALL return status 404 with the error `unknown_member`, `unknown_book` or `unknown_loan`.
- **conflict**: If a loan rule refuses a request, then the service SHALL return status 409 with the error code of the rule (`ERROR_STATUS`).

The rules are in the [loans](../loans.md) spec.
