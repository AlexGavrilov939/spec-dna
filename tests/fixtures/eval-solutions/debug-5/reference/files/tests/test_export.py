import unittest

from export import columns


class ExportTest(unittest.TestCase):
    def test_columns_are_sorted(self):
        rows = [{"delta": 1, "alpha": 2}, {"charlie": 3, "bravo": 4}]
        self.assertEqual(columns(rows), ["alpha", "bravo", "charlie", "delta"])
