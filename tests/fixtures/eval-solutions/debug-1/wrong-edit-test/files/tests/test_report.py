import unittest

from report import format_total


class ReportTest(unittest.TestCase):
    def test_two_lines(self):
        lines = [("19.99", 1), ("0.29", 3)]
        self.assertEqual(format_total(lines), "20.82")
