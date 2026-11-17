import unittest

from duration import parse_duration


class DurationTest(unittest.TestCase):
    # covers: duration#seconds
    def test_seconds(self):
        self.assertEqual(parse_duration("90s"), 90)

    # covers: duration#minutes
    def test_minutes(self):
        self.assertEqual(parse_duration("5m"), 300)

    # covers: duration#hours
    def test_hours(self):
        self.assertEqual(parse_duration("2h"), 7200)
