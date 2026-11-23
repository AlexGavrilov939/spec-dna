import unittest

from slugs.store import SlugStore


class SlugStoreTest(unittest.TestCase):
    def test_returns_the_slug(self):
        self.assertEqual(SlugStore().add("Anna Smith"), "anna-smith")

    def test_adds_a_suffix_on_a_clash(self):
        store = SlugStore()
        self.assertEqual(store.add("Anna Smith"), "anna-smith")
        self.assertEqual(store.add("anna smith"), "anna-smith-2")
        self.assertEqual(store.add("Anna  Smith"), "anna-smith-3")

    def test_different_names_have_no_suffix(self):
        store = SlugStore()
        self.assertEqual(store.add("Anna Smith"), "anna-smith")
        self.assertEqual(store.add("Ben Smith"), "ben-smith")


if __name__ == "__main__":
    unittest.main()
