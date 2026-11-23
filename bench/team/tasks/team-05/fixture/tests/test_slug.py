import unittest

from slugs.slug import slugify


class SlugifyTest(unittest.TestCase):
    def test_lower_case_and_dashes(self):
        self.assertEqual(slugify("Anna  Smith"), "anna-smith")

    def test_edges_have_no_dash(self):
        self.assertEqual(slugify("  Anna Smith! "), "anna-smith")

    def test_keeps_an_accent(self):
        self.assertEqual(slugify("Zo\u00eb Park"), "zo\u00eb-park")


if __name__ == "__main__":
    unittest.main()
