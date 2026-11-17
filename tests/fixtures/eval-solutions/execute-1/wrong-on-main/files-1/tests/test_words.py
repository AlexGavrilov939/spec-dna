import unittest

from words import count_words


class WordsTest(unittest.TestCase):
    def test_count_words(self):
        self.assertEqual(count_words("a b  c"), 3)
