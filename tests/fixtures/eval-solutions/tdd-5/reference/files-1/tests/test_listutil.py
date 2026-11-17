import unittest

from listutil import chunk, first


class ListTest(unittest.TestCase):
    # covers: listutil#first
    def test_first(self):
        self.assertEqual(first([4, 5]), 4)
        self.assertIsNone(first([]))

    # covers: listutil#chunk-size
    def test_chunk(self):
        self.assertEqual(chunk([1, 2, 3, 4, 5], 2), [[1, 2], [3, 4], [5]])
        self.assertEqual(chunk([], 3), [])

    # covers: listutil#chunk-invalid
    def test_chunk_invalid(self):
        with self.assertRaises(ValueError):
            chunk([1], 0)
