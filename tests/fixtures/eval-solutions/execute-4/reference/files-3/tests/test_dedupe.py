import unittest

from dedupe import dedupe


class DedupeTest(unittest.TestCase):
    def test_dedupe(self):
        self.assertEqual(dedupe([2, 1, 2, 3, 1]), [2, 1, 3])
