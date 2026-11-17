import unittest

from money import format_price


class MoneyTest(unittest.TestCase):
    def test_amount(self):
        self.assertEqual(format_price(1050), "10.50")

    def test_whole_number(self):
        self.assertEqual(format_price(1000), "10")
        self.assertEqual(format_price(5), "0.05")
