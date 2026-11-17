import unittest

from adding import add_item


class AddingTest(unittest.TestCase):
    def test_add_item(self):
        self.assertEqual(add_item([1], 2), [1, 2])
