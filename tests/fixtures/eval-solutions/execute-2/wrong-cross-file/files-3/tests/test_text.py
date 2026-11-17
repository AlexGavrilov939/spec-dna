import unittest

from text import pad_left, pad_right


class TextTest(unittest.TestCase):
    def test_pad_left(self):
        self.assertEqual(pad_left("a", 3), "  a")

    def test_pad_right(self):
        self.assertEqual(pad_right("a", 3), "a  ")
