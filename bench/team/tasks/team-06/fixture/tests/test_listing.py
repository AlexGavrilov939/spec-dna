import unittest

from feed.listing import list_items, list_page

ITEMS = [
    {"id": "i03", "created": 130, "title": "c"},
    {"id": "i01", "created": 110, "title": "a"},
    {"id": "i05", "created": 150, "title": "e"},
    {"id": "i02", "created": 120, "title": "b"},
    {"id": "i04", "created": 140, "title": "d"},
]


def ids(items):
    return [item["id"] for item in items]


class ListItemsTest(unittest.TestCase):
    def test_oldest_first(self):
        self.assertEqual(ids(list_items(ITEMS)), ["i01", "i02", "i03", "i04", "i05"])


class ListPageTest(unittest.TestCase):
    def test_first_page(self):
        page = list_page(ITEMS, 2)
        self.assertEqual(ids(page["items"]), ["i01", "i02"])
        self.assertIsInstance(page["next_cursor"], str)

    def test_second_page_follows_the_first(self):
        first = list_page(ITEMS, 2)
        second = list_page(ITEMS, 2, first["next_cursor"])
        self.assertEqual(ids(second["items"]), ["i03", "i04"])

    def test_last_page_has_no_cursor(self):
        page = list_page(ITEMS, 2)
        page = list_page(ITEMS, 2, page["next_cursor"])
        page = list_page(ITEMS, 2, page["next_cursor"])
        self.assertEqual(ids(page["items"]), ["i05"])
        self.assertIsNone(page["next_cursor"])

    def test_a_big_limit_gives_one_page(self):
        page = list_page(ITEMS, 10)
        self.assertEqual(len(page["items"]), 5)
        self.assertIsNone(page["next_cursor"])


if __name__ == "__main__":
    unittest.main()
