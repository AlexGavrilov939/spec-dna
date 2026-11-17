import unittest

from inventory import Inventory


class InventoryTest(unittest.TestCase):
    def test_add_new(self):
        inv = Inventory()
        inv.add("a", 2)
        self.assertEqual(inv.items["a"], 2)

    def test_add_twice(self):
        inv = Inventory()
        inv.add("a")
        inv.add("a")
        self.assertEqual(inv.items["a"], 2)

    def test_starts_empty(self):
        self.assertEqual(Inventory().items, {})

    def test_total(self):
        inv = Inventory()
        inv.add("a", 2)
        self.assertEqual(inv.total(), 2)
