import unittest

from money import to_cents


class MoneyTest(unittest.TestCase):
    def test_whole_and_half(self):
        self.assertEqual(to_cents("1.50"), 150)
        self.assertEqual(to_cents("2"), 200)

    def test_cents_are_not_cut(self):
        self.assertEqual(to_cents("0.29"), 29)
        self.assertEqual(to_cents("19.99"), 1999)
