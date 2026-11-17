import unittest

from legacy import LINE_WIDTH


class LegacyTest(unittest.TestCase):
    def test_old_width(self):
        self.assertEqual(LINE_WIDTH, 80)
