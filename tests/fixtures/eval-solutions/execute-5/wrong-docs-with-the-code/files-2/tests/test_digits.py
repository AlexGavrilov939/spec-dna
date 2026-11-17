import unittest

from digits import count_digits


class DigitsTest(unittest.TestCase):
    def test_count_digits(self):
        self.assertEqual(count_digits("a1b22"), 3)
