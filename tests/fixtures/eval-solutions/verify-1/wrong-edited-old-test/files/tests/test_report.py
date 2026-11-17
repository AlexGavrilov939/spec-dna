import unittest

from report import summary


class ReportTest(unittest.TestCase):
    def test_missing_score(self):
        self.assertEqual(summary([95, 70]), "A C")
