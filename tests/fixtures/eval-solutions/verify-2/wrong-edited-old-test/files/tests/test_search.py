import unittest

from search import search_pages


class SearchTest(unittest.TestCase):
    def test_three_pages_for_25_hits(self):
        words = ["a%d" % i for i in range(25)]
        self.assertEqual(len(search_pages(words, "a")), 2)
