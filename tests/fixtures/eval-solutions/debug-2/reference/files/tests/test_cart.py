import unittest

from cart import Cart


class CartTest(unittest.TestCase):
    def test_add_two(self):
        cart = Cart()
        cart.add("a")
        cart.add("b", 2)
        self.assertEqual(cart.count(), 3)

    def test_a_new_cart_is_empty(self):
        first = Cart()
        first.add("a")
        self.assertEqual(Cart().count(), 0)
