import unittest

from shop.app import Shop
from shop.display import format_cents


class ShopTest(unittest.TestCase):
    def test_format_cents(self):
        self.assertEqual(format_cents(1250), "12.50")
        self.assertEqual(format_cents(5), "0.05")

    def test_label(self):
        shop = Shop({"A1": 1000, "B2": 75})
        self.assertEqual(shop.label("A1"), "$10.00")
        self.assertEqual(shop.label("B2"), "$0.75")

    def test_total(self):
        shop = Shop({"A1": 1000, "B2": 75})
        self.assertEqual(shop.total(["A1", "B2", "B2"]), 1150)

    def test_label_shows_a_new_price(self):
        shop = Shop({"A1": 1000})
        self.assertEqual(shop.label("A1"), "$10.00")
        shop.set_price("A1", 1250)
        self.assertEqual(shop.label("A1"), "$12.50")

    def test_import_adds_new_skus(self):
        shop = Shop({"A1": 1000})
        shop.import_prices({"C3": 300, "D4": 450})
        self.assertEqual(shop.total(["A1", "C3", "D4"]), 1750)


if __name__ == "__main__":
    unittest.main()
