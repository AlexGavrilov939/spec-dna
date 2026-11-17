import unittest

from tax import tax_cents


class TaxTest(unittest.TestCase):
    def test_whole_cents(self):
        self.assertEqual(tax_cents(1000, 10), 100)
        self.assertEqual(tax_cents(0, 10), 0)

    def test_rounds_to_the_nearest_cent(self):
        self.assertEqual(tax_cents(1999, 10), 200)
        self.assertEqual(tax_cents(335, 7), 23)
