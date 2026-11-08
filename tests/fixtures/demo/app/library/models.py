"""Data types of the library service."""

from dataclasses import dataclass, field
from datetime import date


@dataclass
class Book:
    isbn: str
    title: str
    copies: int = 1


@dataclass
class Member:
    member_id: str
    name: str
    kind: str = "regular"  # "regular" or "staff"


@dataclass
class Loan:
    loan_id: int
    isbn: str
    member_id: str
    start: date
    due: date
    returned: date | None = None
    renewals: int = 0
    fines_cents: int = field(default=0)

    @property
    def active(self) -> bool:
        return self.returned is None
