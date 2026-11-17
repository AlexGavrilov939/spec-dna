import unittest

from removing import remove_item


class RemovingTest(unittest.TestCase):
    def test_remove_item(self):
        self.assertEqual(remove_item([1, 2, 1], 1), [2, 1])
