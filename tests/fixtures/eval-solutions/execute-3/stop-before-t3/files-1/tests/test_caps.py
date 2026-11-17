import unittest

from caps import upper_first


class CapsTest(unittest.TestCase):
    def test_upper_first(self):
        self.assertEqual(upper_first("abc"), "Abc")
