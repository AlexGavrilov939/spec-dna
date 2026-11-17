import unittest

from stock import shown


class StockTest(unittest.TestCase):
    def test_small_count(self):
        self.assertEqual(shown(5), 5)

    def test_capped_at_99(self):
        self.assertEqual(shown(150), 99)
