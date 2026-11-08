import unittest
from datetime import date

from library.api import handle, sample_store

TODAY = date(2026, 3, 2)
BOOK = "978-0-14-143951-8"


class ApiTest(unittest.TestCase):
    def setUp(self) -> None:
        self.store = sample_store()

    def call(self, method: str, path: str, body: dict | None = None):
        return handle(self.store, method, path, body or {}, TODAY)

    def test_list_books(self) -> None:
        status, body = self.call("GET", "/books")
        self.assertEqual(status, 200)
        self.assertEqual(len(body["books"]), 2)

    def test_create_and_return_a_loan(self) -> None:
        status, loan = self.call("POST", "/loans", {"member_id": "m1", "isbn": BOOK})
        self.assertEqual(status, 201)
        self.assertEqual(loan["due"], "2026-03-16")
        _, book = self.call("GET", f"/books/{BOOK}")
        self.assertEqual(book["available"], 1)
        status, done = self.call("POST", f"/loans/{loan['id']}/return")
        self.assertEqual(status, 200)
        self.assertEqual(done["returned"], "2026-03-02")

    def test_member_loans(self) -> None:
        self.call("POST", "/loans", {"member_id": "m1", "isbn": BOOK})
        status, body = self.call("GET", "/members/m1/loans")
        self.assertEqual(status, 200)
        self.assertEqual(len(body["loans"]), 1)

    def test_errors(self) -> None:
        self.assertEqual(self.call("POST", "/loans", {"member_id": "m1"})[0], 400)
        self.assertEqual(self.call("POST", "/loans", {"member_id": "zz", "isbn": BOOK})[0], 404)
        self.assertEqual(self.call("POST", "/loans/99/renew")[0], 404)
        self.assertEqual(self.call("GET", "/nowhere")[0], 404)


if __name__ == "__main__":
    unittest.main()
