import unittest

from shipping import ship_cost


class ShippingTest(unittest.TestCase):
    def test_cost(self):
        self.assertEqual(ship_cost(2, "a"), 8)

    def test_weight_out_of_range(self):
        for weight in (0, 31):
            with self.assertRaises(ValueError):
                ship_cost(weight, "a")
