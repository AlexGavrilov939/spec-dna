import unittest

from bucket import bucket


class BucketTest(unittest.TestCase):
    def test_inside_the_range(self):
        self.assertEqual(bucket(25), 2)
        self.assertEqual(bucket(0), 0)

    def test_capped_at_10(self):
        self.assertEqual(bucket(150), 10)

    def test_not_below_0(self):
        self.assertEqual(bucket(-5), 0)
