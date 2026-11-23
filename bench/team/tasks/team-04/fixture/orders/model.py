from dataclasses import dataclass
from datetime import datetime


@dataclass(frozen=True)
class Order:
    id: str
    placed_at: datetime  # aware, in UTC
