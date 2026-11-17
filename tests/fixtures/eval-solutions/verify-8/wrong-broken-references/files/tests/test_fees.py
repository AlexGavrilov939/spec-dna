# covers: requirements/billing#late-fee
import unittest

from fees import late_fee


class FeesTest(unittest.TestCase):
    def test_fee(self):
        self.assertEqual(late_fee(1000), 80)
