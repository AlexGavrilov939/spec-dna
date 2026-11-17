import unittest

from invoice import line


class InvoiceTest(unittest.TestCase):
    def test_line(self):
        self.assertEqual(line("tea", 250), "tea 2.50")
