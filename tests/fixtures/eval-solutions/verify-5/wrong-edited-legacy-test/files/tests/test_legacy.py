import unittest

from pricing import with_tax


class LegacyTest(unittest.TestCase):
    def test_old_rate(self):
        self.assertEqual(with_tax(10), 12.0)
