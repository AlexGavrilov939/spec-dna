import unittest

from paging import paginate


class PagingTest(unittest.TestCase):
    def test_explicit_size(self):
        self.assertEqual(len(paginate(list(range(25)), 10)), 3)

    def test_empty(self):
        self.assertEqual(paginate([], 5), [])

    def test_default_size(self):
        pages = paginate(list(range(45)))
        self.assertEqual([len(p) for p in pages], [20, 20, 5])
