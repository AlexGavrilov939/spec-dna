import unittest

from pricing.rates import quote


class QuoteTest(unittest.TestCase):
    def test_local_standard(self):
        self.assertEqual(quote(1, "local"), 40)

    def test_overnight_by_position(self):
        self.assertEqual(quote(3, "national", "overnight"), 675)

    def test_express_costs_more_than_standard(self):
        standard = quote(5, "europe", service="standard")
        express = quote(5, "europe", service="express")
        self.assertEqual(standard, 900)
        self.assertEqual(express, 1350)


if __name__ == "__main__":
    unittest.main()
