# covers: requirements/billing#overdue-fee
import unittest

from fees import overdue_fee


class FeesTest(unittest.TestCase):
    def test_fee(self):
        self.assertEqual(overdue_fee(1000), 50)
