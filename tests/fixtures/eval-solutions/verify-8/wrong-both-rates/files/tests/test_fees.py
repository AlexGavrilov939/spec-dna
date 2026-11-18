# covers: requirements/billing#overdue-fee
import unittest

from fees import late_fee, overdue_fee


class FeesTest(unittest.TestCase):
    def test_old_fee(self):
        self.assertEqual(late_fee(1000), 50)

    def test_fee(self):
        self.assertEqual(overdue_fee(1000), 80)
