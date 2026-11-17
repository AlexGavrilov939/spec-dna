import unittest

from stats import mean


class StatsTest(unittest.TestCase):
    # covers: stats#mean
    def test_mean(self):
        self.assertEqual(mean([1, 2, 3]), 2)

    # covers: stats#mean-empty
    def test_mean_empty(self):
        self.assertEqual(mean([]), 0)
