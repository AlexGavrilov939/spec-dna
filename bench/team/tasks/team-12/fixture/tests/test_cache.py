import unittest

from pricing.cache import memoize


class MemoizeTest(unittest.TestCase):
    def test_the_same_call_runs_once(self):
        calls = []

        @memoize
        def double(n):
            calls.append(n)
            return n * 2

        self.assertEqual(double(4), 8)
        self.assertEqual(double(4), 8)
        self.assertEqual(calls, [4])

    def test_other_arguments_run_again(self):
        @memoize
        def add(a, b):
            return a + b

        self.assertEqual(add(1, 2), 3)
        self.assertEqual(add(1, 5), 6)


if __name__ == "__main__":
    unittest.main()
