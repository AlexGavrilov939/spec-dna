import unittest

from table import format_row


class TableTest(unittest.TestCase):
    def test_format_row(self):
        self.assertEqual(format_row(["a", "b"], [2, 3]), "a  b  ")
