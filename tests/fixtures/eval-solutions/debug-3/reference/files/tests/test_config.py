import unittest

from config import load, parse_line


class ConfigTest(unittest.TestCase):
    def test_parse_line(self):
        self.assertEqual(parse_line("name = demo"), ("name", "demo"))

    def test_load_skips_comments(self):
        self.assertEqual(load("# c\na=1\n\nb=2"), {"a": "1", "b": "2"})

    def test_value_keeps_an_equal_sign(self):
        self.assertEqual(parse_line("q=a=1"), ("q", "a=1"))
