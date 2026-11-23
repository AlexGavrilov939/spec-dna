import unittest
from datetime import date, datetime, timezone

from orders.model import Order
from orders.report import orders_of_day
from orders.store import Store


def at(month, day, hour, minute=0):
    return datetime(2024, month, day, hour, minute, tzinfo=timezone.utc)


class OrdersOfDayTest(unittest.TestCase):
    def setUp(self):
        self.store = Store(
            [
                Order("c", at(6, 15, 23, 59)),
                Order("a", at(6, 15, 0, 0)),
                Order("d", at(6, 16, 0, 0)),
                Order("b", at(6, 15, 12, 30)),
                Order("z", at(6, 14, 23, 59)),
            ]
        )

    def test_the_day_in_utc_is_the_default(self):
        self.assertEqual(orders_of_day(self.store, date(2024, 6, 15)), ["a", "b", "c"])

    def test_a_zone_behind_utc(self):
        # New York is 4 hours behind UTC.
        got = orders_of_day(self.store, date(2024, 6, 15), "America/New_York")
        self.assertEqual(got, ["b", "c", "d"])

    def test_a_zone_ahead_of_utc(self):
        # Kolkata is 5 hours 30 minutes ahead of UTC.
        got = orders_of_day(self.store, date(2024, 6, 15), "Asia/Kolkata")
        self.assertEqual(got, ["z", "a", "b"])

    def test_an_empty_day(self):
        self.assertEqual(orders_of_day(self.store, date(2024, 6, 20), "UTC"), [])


if __name__ == "__main__":
    unittest.main()
