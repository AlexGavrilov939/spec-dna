import unittest

from grades import letter


class GradesTest(unittest.TestCase):
    def test_letters(self):
        self.assertEqual(letter(95), "A")
        self.assertEqual(letter(85), "B")

    def test_text_score(self):
        self.assertEqual(letter("85"), "B")
