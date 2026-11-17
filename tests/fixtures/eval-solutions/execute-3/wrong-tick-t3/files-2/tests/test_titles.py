import unittest

from titles import title_case


class TitlesTest(unittest.TestCase):
    def test_title_case(self):
        self.assertEqual(title_case("a b"), "A B")
