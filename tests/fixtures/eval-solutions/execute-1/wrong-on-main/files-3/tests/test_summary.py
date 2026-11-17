import unittest

from summary import summarize


class SummaryTest(unittest.TestCase):
    def test_summarize(self):
        self.assertEqual(summarize("a b\nc\n"), "2 lines, 3 words")
