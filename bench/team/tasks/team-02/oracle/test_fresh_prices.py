import unittest

from shop.app import Shop


class FreshPriceTest(unittest.TestCase):
    def test_price_is_new_after_set_price(self):
        shop = Shop({"A1": 1000})
        self.assertEqual(shop.price("A1"), 1000)
        shop.set_price("A1", 1250)
        self.assertEqual(shop.price("A1"), 1250)

    def test_total_is_new_after_set_price(self):
        shop = Shop({"A1": 1000, "B2": 75})
        self.assertEqual(shop.total(["A1", "B2"]), 1075)
        shop.set_price("A1", 1250)
        self.assertEqual(shop.total(["A1", "B2"]), 1325)

    def test_label_is_new_after_import_prices(self):
        shop = Shop({"A1": 1000, "B2": 75})
        self.assertEqual(shop.label("B2"), "$0.75")
        shop.import_prices({"B2": 80, "C3": 300})
        self.assertEqual(shop.label("B2"), "$0.80")
        self.assertEqual(shop.label("C3"), "$3.00")

    def test_total_is_new_after_import_prices(self):
        shop = Shop({"A1": 1000, "B2": 75})
        self.assertEqual(shop.total(["A1", "B2"]), 1075)
        shop.import_prices({"A1": 900, "B2": 80})
        self.assertEqual(shop.total(["A1", "B2"]), 980)

    def test_another_sku_keeps_its_price(self):
        shop = Shop({"A1": 1000, "B2": 75})
        self.assertEqual(shop.price("B2"), 75)
        shop.set_price("A1", 1250)
        self.assertEqual(shop.price("B2"), 75)
        self.assertEqual(shop.total(["B2", "B2"]), 150)

    def test_each_of_two_updates_in_a_row_shows(self):
        shop = Shop({"A1": 1000})
        self.assertEqual(shop.price("A1"), 1000)
        shop.set_price("A1", 1100)
        self.assertEqual(shop.price("A1"), 1100)
        shop.import_prices({"A1": 1200})
        self.assertEqual(shop.price("A1"), 1200)
        shop.set_price("A1", 1300)
        self.assertEqual(shop.total(["A1"]), 1300)


if __name__ == "__main__":
    unittest.main()
