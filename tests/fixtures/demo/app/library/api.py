"""HTTP JSON API. `handle` maps one request to a status and a JSON body, so
the tests call it without a server."""

import argparse
import json
import re
from datetime import date
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from typing import Any, Callable

from library import loans
from library.models import Book, Loan, Member
from library.store import Store

Response = tuple[int, dict[str, Any]]

ERROR_STATUS = {
    "unknown_member": 404,
    "unknown_book": 404,
    "unknown_loan": 404,
    "loan_limit": 409,
    "not_available": 409,
    "renewal_limit": 409,
    "overdue": 409,
    "returned": 409,
}


def loan_json(loan: Loan) -> dict[str, Any]:
    return {
        "id": loan.loan_id,
        "isbn": loan.isbn,
        "member_id": loan.member_id,
        "start": loan.start.isoformat(),
        "due": loan.due.isoformat(),
        "returned": loan.returned.isoformat() if loan.returned else None,
        "renewals": loan.renewals,
        "fines_cents": loan.fines_cents,
    }


def book_json(store: Store, book: Book) -> dict[str, Any]:
    on_loan = len(store.active_loans_for_book(book.isbn))
    return {
        "isbn": book.isbn,
        "title": book.title,
        "copies": book.copies,
        "available": book.copies - on_loan,
    }


def handle(
    store: Store, method: str, path: str, body: dict[str, Any], today: date
) -> Response:
    routes: list[tuple[str, str, Callable[..., Response]]] = [
        ("GET", r"/books", _list_books),
        ("GET", r"/books/(?P<isbn>[^/]+)", _get_book),
        ("POST", r"/loans", _create_loan),
        ("POST", r"/loans/(?P<loan_id>\d+)/return", _return_loan),
        ("POST", r"/loans/(?P<loan_id>\d+)/renew", _renew_loan),
        ("GET", r"/members/(?P<member_id>[^/]+)/loans", _member_loans),
    ]
    for m, pattern, fn in routes:
        match = re.fullmatch(pattern, path)
        if match and m == method:
            try:
                return fn(store, body, today, **match.groupdict())
            except loans.LoanError as e:
                return ERROR_STATUS.get(e.code, 400), {"error": e.code, "message": str(e)}
    return 404, {"error": "not_found", "message": f"no route {method} {path}"}


def _list_books(store: Store, body: dict, today: date) -> Response:
    return 200, {"books": [book_json(store, b) for b in store.books.values()]}


def _get_book(store: Store, body: dict, today: date, isbn: str) -> Response:
    book = store.books.get(isbn)
    if book is None:
        return 404, {"error": "unknown_book", "message": f"no book {isbn}"}
    return 200, book_json(store, book)


def _create_loan(store: Store, body: dict, today: date) -> Response:
    member_id, isbn = body.get("member_id"), body.get("isbn")
    if not isinstance(member_id, str) or not isinstance(isbn, str):
        return 400, {"error": "bad_request", "message": "member_id and isbn are required"}
    return 201, loan_json(loans.checkout_book(store, member_id, isbn, today))


def _return_loan(store: Store, body: dict, today: date, loan_id: str) -> Response:
    return 200, loan_json(loans.return_book(store, int(loan_id), today))


def _renew_loan(store: Store, body: dict, today: date, loan_id: str) -> Response:
    return 200, loan_json(loans.renew_loan(store, int(loan_id), today))


def _member_loans(store: Store, body: dict, today: date, member_id: str) -> Response:
    if member_id not in store.members:
        return 404, {"error": "unknown_member", "message": f"no member {member_id}"}
    active = store.active_loans_for_member(member_id)
    return 200, {"loans": [loan_json(loan) for loan in active]}


def sample_store() -> Store:
    store = Store()
    store.add_book(Book("978-0-14-143951-8", "Pride and Prejudice", copies=2))
    store.add_book(Book("978-0-452-28423-4", "Nineteen Eighty-Four", copies=1))
    store.add_member(Member("m1", "Ada"))
    store.add_member(Member("s1", "Grace", kind="staff"))
    return store


def serve(port: int) -> None:
    store = sample_store()

    class Handler(BaseHTTPRequestHandler):
        def _send(self, method: str) -> None:
            length = int(self.headers.get("Content-Length") or 0)
            raw = self.rfile.read(length) if length else b"{}"
            try:
                body = json.loads(raw or b"{}")
            except json.JSONDecodeError:
                body = None
            if not isinstance(body, dict):
                status, payload = 400, {"error": "bad_request", "message": "invalid JSON"}
            else:
                status, payload = handle(store, method, self.path, body, date.today())
            data = json.dumps(payload).encode()
            self.send_response(status)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)

        def do_GET(self) -> None:
            self._send("GET")

        def do_POST(self) -> None:
            self._send("POST")

    ThreadingHTTPServer(("127.0.0.1", port), Handler).serve_forever()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Library loans service")
    parser.add_argument("--port", type=int, default=8080)
    serve(parser.parse_args().port)
