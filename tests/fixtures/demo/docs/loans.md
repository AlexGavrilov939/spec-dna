---
status: active
---
# Loans

A member borrows a copy of a book, renews the loan, and returns the copy. The
rules are in `library/loans.py`.

## Checkout

- **loan-period**: When a member borrows a book, the library SHALL set the due date `LOAN_DAYS` (14) days after the checkout date.
- **active-loan-limit**: If a member has `MAX_ACTIVE_LOANS` (5) active loans, then the library SHALL refuse a new loan with the error `loan_limit`.
- **availability**: If every copy of a book is on loan, then the library SHALL refuse a new loan of that book with the error `not_available`.

## Renewals

- **renewal-limit**: If a loan has `MAX_RENEWALS` (1) renewals, then `renew_loan()` SHALL refuse a further renewal with the error `renewal_limit`.
- **renewal-period**: When a member renews a loan, the library SHALL move the due date `LOAN_DAYS` days later.
- **no-overdue-renewal**: If a loan is overdue, then the library SHALL refuse its renewal with the error `overdue`.

## Returns and fines

- **return**: When a member returns a book, the library SHALL record the return date and the fine of the loan.
- **daily-fine**: When a member returns a book after the due date, the library SHALL charge `DAILY_FINE_CENTS` (25 cents) for each late day.

The [flat daily fine](decisions/flat-daily-fine.md) decision gives the reason for this fine.
