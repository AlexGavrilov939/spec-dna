"""Loan rules: checkout, return, renewal and fines."""

from datetime import date, timedelta

from library.models import Loan
from library.store import Store

LOAN_DAYS = 14
MAX_RENEWALS = 1
MAX_ACTIVE_LOANS = 5
DAILY_FINE_CENTS = 25


class LoanError(Exception):
    """A request that the loan rules do not allow. `code` is stable."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code


def checkout_book(store: Store, member_id: str, isbn: str, today: date) -> Loan:
    if member_id not in store.members:
        raise LoanError("unknown_member", f"no member {member_id}")
    book = store.books.get(isbn)
    if book is None:
        raise LoanError("unknown_book", f"no book {isbn}")
    if len(store.active_loans_for_member(member_id)) >= MAX_ACTIVE_LOANS:
        raise LoanError("loan_limit", "the member has too many active loans")
    if len(store.active_loans_for_book(isbn)) >= book.copies:
        raise LoanError("not_available", "no copy of the book is available")
    loan = Loan(
        loan_id=store.new_loan_id(),
        isbn=isbn,
        member_id=member_id,
        start=today,
        due=today + timedelta(days=LOAN_DAYS),
    )
    store.loans[loan.loan_id] = loan
    return loan


def fine_for(loan: Loan, today: date) -> int:
    """The fine in cents for a loan that is late on `today`."""
    end = loan.returned or today
    late_days = (end - loan.due).days
    return max(0, late_days) * DAILY_FINE_CENTS


def return_book(store: Store, loan_id: int, today: date) -> Loan:
    loan = _active_loan(store, loan_id)
    loan.returned = today
    loan.fines_cents = fine_for(loan, today)
    return loan


def renew_loan(store: Store, loan_id: int, today: date) -> Loan:
    loan = _active_loan(store, loan_id)
    if loan.renewals >= MAX_RENEWALS:
        raise LoanError("renewal_limit", "the loan has no renewals left")
    if today > loan.due:
        raise LoanError("overdue", "an overdue loan cannot be renewed")
    loan.due = loan.due + timedelta(days=LOAN_DAYS)
    loan.renewals += 1
    return loan


def _active_loan(store: Store, loan_id: int) -> Loan:
    loan = store.loans.get(loan_id)
    if loan is None:
        raise LoanError("unknown_loan", f"no loan {loan_id}")
    if not loan.active:
        raise LoanError("returned", "the loan is already returned")
    return loan
