import unittest

from cache import cached, make_key


class CacheTest(unittest.TestCase):
    def test_same_parts_same_key(self):
        self.assertEqual(make_key("a", 1), make_key("a", 1))

    def test_cached_computes_once(self):
        calls = []
        for _ in range(2):
            cached("once", ("x",), lambda: calls.append(1) or 5)
        self.assertEqual(len(calls), 1)

    def test_split_parts_make_other_keys(self):
        self.assertNotEqual(make_key("ab", "c"), make_key("a", "bc"))
