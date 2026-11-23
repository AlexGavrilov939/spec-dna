import unittest
from datetime import date, datetime, timezone

from orders.model import Order
from orders.report import orders_of_day
from orders.store import Store

NEW_YORK = "America/New_York"


def utc(text):
    """An aware UTC datetime from 'YYYY-MM-DD HH:MM:SS'."""
    naive = datetime.strptime(text, "%Y-%m-%d %H:%M:%S")
    return naive.replace(tzinfo=timezone.utc)


def store_of(pairs):
    return Store([Order(order_id, utc(text)) for order_id, text in pairs])


class ClockChangeTest(unittest.TestCase):
    def test_a_day_of_23_hours_in_spring(self):
        # 2024-03-10 in New York runs from 05:00 UTC to 04:00 UTC on the 11th.
        store = store_of(
            [
                ("m1", "2024-03-10 04:59:59"),
                ("m2", "2024-03-10 05:00:00"),
                ("m3", "2024-03-11 03:59:59"),
                ("m4", "2024-03-11 04:00:00"),
                ("m5", "2024-03-11 04:30:00"),
            ]
        )
        self.assertEqual(orders_of_day(store, date(2024, 3, 10), NEW_YORK), ["m2", "m3"])

    def test_a_day_of_25_hours_in_autumn(self):
        # 2024-11-03 in New York runs from 04:00 UTC to 05:00 UTC on the 4th.
        store = store_of(
            [
                ("n1", "2024-11-03 03:59:59"),
                ("n2", "2024-11-03 04:00:00"),
                ("n3", "2024-11-03 05:30:00"),
                ("n4", "2024-11-03 06:30:00"),
                ("n5", "2024-11-04 04:30:00"),
                ("n6", "2024-11-04 04:59:59"),
                ("n7", "2024-11-04 05:00:00"),
            ]
        )
        got = orders_of_day(store, date(2024, 11, 3), NEW_YORK)
        self.assertEqual(got, ["n2", "n3", "n4", "n5", "n6"])

    def test_the_day_after_a_day_of_25_hours(self):
        store = store_of(
            [
                ("n5", "2024-11-04 04:30:00"),
                ("n6", "2024-11-04 04:59:59"),
                ("n7", "2024-11-04 05:00:00"),
                ("n8", "2024-11-05 04:59:59"),
                ("n9", "2024-11-05 05:00:00"),
            ]
        )
        self.assertEqual(orders_of_day(store, date(2024, 11, 4), NEW_YORK), ["n7", "n8"])

    def test_a_day_of_25_hours_south_of_the_equator(self):
        # 2024-04-07 in Sydney runs from 2024-04-06 13:00 UTC to 2024-04-07 14:00 UTC.
        store = store_of(
            [
                ("s1", "2024-04-06 12:59:59"),
                ("s2", "2024-04-06 13:00:00"),
                ("s3", "2024-04-07 13:30:00"),
                ("s4", "2024-04-07 14:00:00"),
            ]
        )
        got = orders_of_day(store, date(2024, 4, 7), "Australia/Sydney")
        self.assertEqual(got, ["s2", "s3"])

    def test_the_order_is_by_time_then_by_id(self):
        store = store_of(
            [
                ("c", "2024-11-03 10:00:00"),
                ("a", "2024-11-03 12:00:00"),
                ("b", "2024-11-03 10:00:00"),
                ("0", "2024-11-03 13:00:00"),
            ]
        )
        got = orders_of_day(store, date(2024, 11, 3), NEW_YORK)
        self.assertEqual(got, ["b", "c", "a", "0"])

    def test_utc_is_the_default_and_a_zone_can_be_utc(self):
        store = store_of(
            [
                ("u1", "2024-03-09 23:59:59"),
                ("u2", "2024-03-10 00:00:00"),
                ("u3", "2024-03-10 23:59:59"),
                ("u4", "2024-03-11 00:00:00"),
            ]
        )
        self.assertEqual(orders_of_day(store, date(2024, 3, 10)), ["u2", "u3"])
        self.assertEqual(orders_of_day(store, date(2024, 3, 10), "UTC"), ["u2", "u3"])


if __name__ == "__main__":
    unittest.main()
