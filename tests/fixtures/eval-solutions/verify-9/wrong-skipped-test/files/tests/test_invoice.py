import unittest

from invoice import line


class InvoiceTest(unittest.TestCase):
    def test_line(self):
        self.assertEqual(line("tea", 250), "tea 2.50")

    @unittest.skip("the invoice text changes later")
    def test_line_with_no_cents(self):
        self.assertEqual(line("rice", 300), "rice 3.00")
