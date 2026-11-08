"""In-memory storage. The service keeps all state in one Store object."""

from library.models import Book, Loan, Member


class Store:
    def __init__(self) -> None:
        self.books: dict[str, Book] = {}
        self.members: dict[str, Member] = {}
        self.loans: dict[int, Loan] = {}
        self._next_loan_id = 1

    def add_book(self, book: Book) -> None:
        self.books[book.isbn] = book

    def add_member(self, member: Member) -> None:
        self.members[member.member_id] = member

    def new_loan_id(self) -> int:
        loan_id = self._next_loan_id
        self._next_loan_id += 1
        return loan_id

    def active_loans_for_book(self, isbn: str) -> list[Loan]:
        return [loan for loan in self.loans.values() if loan.isbn == isbn and loan.active]

    def active_loans_for_member(self, member_id: str) -> list[Loan]:
        return [
            loan
            for loan in self.loans.values()
            if loan.member_id == member_id and loan.active
        ]
