import unittest

from feed.listing import list_page

# 12 items. The input is not sorted, and the order of the ids is not the order of
# the feed. Many items share one `created`.
RAW = [
    ("i07", 200),
    ("i01", 300),
    ("i11", 100),
    ("i03", 300),
    ("i05", 200),
    ("i02", 100),
    ("i09", 400),
    ("i04", 300),
    ("i12", 100),
    ("i06", 200),
    ("i10", 300),
    ("i08", 100),
]
DATA = [{"id": i, "created": c, "title": "t" + i} for i, c in RAW]
EXPECTED = [i["id"] for i in sorted(DATA, key=lambda i: (i["created"], i["id"]))]
LIMITS = (1, 2, 3, 4, 5, 7)


def walk(items, limit):
    """Read each page, from the first one. Fail if the walk does not end."""
    pages = []
    cursor = None
    for _ in range(len(items) + 2):
        page = list_page(items, limit, cursor)
        pages.append(page)
        cursor = page["next_cursor"]
        if cursor is None:
            return pages
    raise AssertionError(f"limit {limit}: the walk does not end")


def ids(pages):
    return [item["id"] for page in pages for item in page["items"]]


class PagingTest(unittest.TestCase):
    def test_each_item_shows_once_and_in_order(self):
        for limit in LIMITS:
            with self.subTest(limit=limit):
                self.assertEqual(ids(walk(DATA, limit)), EXPECTED)

    def test_each_page_has_at_most_limit_items(self):
        for limit in LIMITS:
            with self.subTest(limit=limit):
                sizes = [len(page["items"]) for page in walk(DATA, limit)]
                self.assertTrue(all(1 <= size <= limit for size in sizes), sizes)

    def test_only_the_last_page_has_no_cursor(self):
        for limit in LIMITS:
            with self.subTest(limit=limit):
                pages = walk(DATA, limit)
                for page in pages[:-1]:
                    self.assertIsInstance(page["next_cursor"], str)
                self.assertIsNone(pages[-1]["next_cursor"])

    def test_a_limit_of_the_size_of_the_list_or_more_gives_one_page(self):
        for limit in (len(DATA), len(DATA) + 1, 100):
            with self.subTest(limit=limit):
                pages = walk(DATA, limit)
                self.assertEqual(len(pages), 1)
                self.assertEqual(ids(pages), EXPECTED)

    def test_items_with_one_created_value_only(self):
        data = [{"id": f"j{n:02d}", "created": 500, "title": "t"} for n in (4, 9, 1, 7, 3, 8, 2, 6, 5)]
        expected = [f"j{n:02d}" for n in range(1, 10)]
        for limit in (1, 2, 3, 4, 8, 9):
            with self.subTest(limit=limit):
                self.assertEqual(ids(walk(data, limit)), expected)

    def test_an_empty_feed_gives_one_empty_page(self):
        page = list_page([], 3)
        self.assertEqual(page["items"], [])
        self.assertIsNone(page["next_cursor"])


if __name__ == "__main__":
    unittest.main()
