# Rules of the payment calls

## P-1. Amounts

An amount is an integer number of cents.

## P-2. Errors

The gateway raises `TransientError` for an outage and `CardDeclined` when the bank says no. A client never hides a `CardDeclined`.

## P-3. Idempotency key

One order has one idempotency key: `order-<order_id>`. Each call for that order sends this key, and so does each retry. The gateway charges a key once. A new key for the same order is a new charge, and the customer pays twice.
