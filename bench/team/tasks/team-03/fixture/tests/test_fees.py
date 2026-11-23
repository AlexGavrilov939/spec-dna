import unittest

from ledger.fees import fee_cents, net_cents
from ledger.money import format_cents


class FeeTest(unittest.TestCase):
    def test_an_exact_fee(self):
        self.assertEqual(fee_cents(10000, 250), 250)

    def test_a_fee_below_half_a_cent_goes_down(self):
        self.assertEqual(fee_cents(333, 250), 8)  # 8.325

    def test_a_fee_above_half_a_cent_goes_up(self):
        self.assertEqual(fee_cents(1999, 250), 50)  # 49.975
        self.assertEqual(fee_cents(-1999, 250), -50)

    def test_a_half_cent(self):
        self.assertEqual(fee_cents(60, 250), 2)  # 1.5
        self.assertEqual(fee_cents(140, 250), 4)  # 3.5

    def test_the_fee_is_an_int(self):
        self.assertIsInstance(fee_cents(1999, 250), int)

    def test_net(self):
        self.assertEqual(net_cents(10000, 250), 9750)

    def test_format(self):
        self.assertEqual(format_cents(1250), "12.50")
        self.assertEqual(format_cents(-5), "-0.05")


if __name__ == "__main__":
    unittest.main()
