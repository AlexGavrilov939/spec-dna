import unittest

from vowels import count_vowels


class VowelsTest(unittest.TestCase):
    def test_count_vowels(self):
        self.assertEqual(count_vowels("Hello"), 2)
