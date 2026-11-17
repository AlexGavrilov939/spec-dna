import unittest

from pricing import shipping, with_tax


class PricingTest(unittest.TestCase):
    def test_with_tax(self):
        self.assertEqual(with_tax(10), 12.0)

    def test_shipping(self):
        self.assertEqual(shipping(50), 0)
        self.assertEqual(shipping(49.99), 5)
