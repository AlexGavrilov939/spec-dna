import unittest
from datetime import date, timedelta

from library import loans
from library.api import sample_store
from library.loans import LoanError

BOOK = "978-0-452-28423-4"
TODAY = date(2026, 3, 2)


class CheckoutTest(unittest.TestCase):
    def setUp(self) -> None:
        self.store = sample_store()

    def test_due_date_is_loan_days_later(self) -> None:
        loan = loans.checkout_book(self.store, "m1", BOOK, TODAY)
        self.assertEqual(loan.due, TODAY + timedelta(days=14))

    def test_no_copy_left(self) -> None:
        loans.checkout_book(self.store, "m1", BOOK, TODAY)
        with self.assertRaises(LoanError) as e:
            loans.checkout_book(self.store, "s1", BOOK, TODAY)
        self.assertEqual(e.exception.code, "not_available")

    def test_active_loan_limit(self) -> None:
        for i in range(loans.MAX_ACTIVE_LOANS):
            self.store.books[f"isbn-{i}"] = type(self.store.books[BOOK])(f"isbn-{i}", "x")
            loans.checkout_book(self.store, "m1", f"isbn-{i}", TODAY)
        with self.assertRaises(LoanError) as e:
            loans.checkout_book(self.store, "m1", BOOK, TODAY)
        self.assertEqual(e.exception.code, "loan_limit")


class RenewalTest(unittest.TestCase):
    def setUp(self) -> None:
        self.store = sample_store()
        self.loan = loans.checkout_book(self.store, "m1", BOOK, TODAY)

    def test_renewal_extends_the_due_date(self) -> None:
        loans.renew_loan(self.store, self.loan.loan_id, TODAY)
        self.assertEqual(self.loan.due, TODAY + timedelta(days=28))

    def test_renewal_limit(self) -> None:
        loans.renew_loan(self.store, self.loan.loan_id, TODAY)
        with self.assertRaises(LoanError) as e:
            loans.renew_loan(self.store, self.loan.loan_id, TODAY)
        self.assertEqual(e.exception.code, "renewal_limit")

    def test_overdue_loan_cannot_be_renewed(self) -> None:
        with self.assertRaises(LoanError) as e:
            loans.renew_loan(self.store, self.loan.loan_id, TODAY + timedelta(days=15))
        self.assertEqual(e.exception.code, "overdue")


class FineTest(unittest.TestCase):
    def setUp(self) -> None:
        self.store = sample_store()
        self.loan = loans.checkout_book(self.store, "m1", BOOK, TODAY)

    def test_no_fine_on_time(self) -> None:
        loan = loans.return_book(self.store, self.loan.loan_id, self.loan.due)
        self.assertEqual(loan.fines_cents, 0)

    def test_daily_fine(self) -> None:
        loan = loans.return_book(self.store, self.loan.loan_id, self.loan.due + timedelta(days=3))
        self.assertEqual(loan.fines_cents, 3 * loans.DAILY_FINE_CENTS)

    def test_return_twice(self) -> None:
        loans.return_book(self.store, self.loan.loan_id, TODAY)
        with self.assertRaises(LoanError) as e:
            loans.return_book(self.store, self.loan.loan_id, TODAY)
        self.assertEqual(e.exception.code, "returned")


if __name__ == "__main__":
    unittest.main()
