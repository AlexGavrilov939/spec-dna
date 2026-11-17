import unittest

from slug import slug_lower, slug_trim


class SlugTest(unittest.TestCase):
    # covers: slug#lower
    def test_lower(self):
        self.assertEqual(slug_lower("AbC"), "abc")

    # covers: slug#trim-dashes
    def test_trim_dashes(self):
        self.assertEqual(slug_trim("--a-b--"), "a-b")
        self.assertEqual(slug_trim("-"), "")
        self.assertEqual(slug_trim("a"), "a")
