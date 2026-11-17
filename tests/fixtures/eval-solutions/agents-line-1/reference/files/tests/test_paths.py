import os
import unittest


class PathsTest(unittest.TestCase):
    def test_the_billing_document_is_there(self):
        self.assertTrue(os.path.exists("docs/invoices/overview.md"))
