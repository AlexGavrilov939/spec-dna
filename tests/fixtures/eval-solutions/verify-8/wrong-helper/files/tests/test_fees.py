# covers: requirements/billing#overdue-fee
import unittest

from fees import late_fee, percent_of


class FeesTest(unittest.TestCase):
    def test_fee(self):
        self.assertEqual(late_fee(1000), 50)

    def test_helper(self):
        self.assertEqual(percent_of(1000), 80)
