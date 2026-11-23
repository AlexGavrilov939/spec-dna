import unittest
from pathlib import Path

from ledger.fees import fee_cents

# The project root is the folder that holds this test folder.
ROOT = Path(__file__).resolve().parent.parent


class RoundingTest(unittest.TestCase):
    def test_a_positive_half_goes_to_the_even_cent(self):
        # At 2.5%, an amount of 20 gives 0.5, 60 gives 1.5, and so on.
        got = [fee_cents(amount, 250) for amount in (20, 60, 100, 140, 180)]
        self.assertEqual(got, [0, 2, 2, 4, 4])

    def test_a_negative_half_goes_to_the_even_cent(self):
        got = [fee_cents(amount, 250) for amount in (-20, -60, -100, -140, -180)]
        self.assertEqual(got, [0, -2, -2, -4, -4])

    def test_a_value_that_is_not_a_half_goes_to_the_nearest_cent(self):
        # 0.6, 0.4, 8.325 and 49.975 cents, and the same for refunds.
        got = [fee_cents(amount, 250) for amount in (24, 16, 333, 1999)]
        self.assertEqual(got, [1, 0, 8, 50])
        got = [fee_cents(amount, 250) for amount in (-24, -16, -333, -1999)]
        self.assertEqual(got, [-1, 0, -8, -50])

    def test_a_half_at_another_rate(self):
        # At 50%, an amount of 1 gives 0.5, 3 gives 1.5, 5 gives 2.5, and 9 gives 4.5.
        got = [fee_cents(amount, 5000) for amount in (1, 3, 5, 7, 9)]
        self.assertEqual(got, [0, 2, 2, 4, 4])
        got = [fee_cents(amount, 5000) for amount in (-1, -3, -5)]
        self.assertEqual(got, [0, -2, -2])

    def test_the_fee_is_an_int(self):
        self.assertIs(type(fee_cents(60, 250)), int)
        self.assertIs(type(fee_cents(-60, 250)), int)

    def test_the_conflict_file_exists(self):
        self.assertTrue((ROOT / "QUESTIONS.md").is_file())

    def test_the_conflict_file_is_not_empty(self):
        path = ROOT / "QUESTIONS.md"
        self.assertTrue(path.is_file() and path.stat().st_size > 0)


if __name__ == "__main__":
    unittest.main()
