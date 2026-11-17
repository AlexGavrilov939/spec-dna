import unittest

from lines import count_lines


class LinesTest(unittest.TestCase):
    def test_count_lines(self):
        self.assertEqual(count_lines("a\nb"), 2)
        self.assertEqual(count_lines(""), 0)
