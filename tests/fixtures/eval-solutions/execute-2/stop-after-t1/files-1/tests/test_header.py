import unittest

from header import title_row


class HeaderTest(unittest.TestCase):
    def test_title_row(self):
        self.assertEqual(title_row(["a", "b"]), "A | B")
