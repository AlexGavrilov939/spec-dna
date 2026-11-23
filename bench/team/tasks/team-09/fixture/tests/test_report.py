import unittest

from ledger.report import parse_line, parse_lines, rank_names, top_name


class ParseTest(unittest.TestCase):
    def test_reads_a_line(self):
        self.assertEqual(parse_line("ann 40"), ("ann", 40))

    def test_skips_blank_lines(self):
        self.assertEqual(parse_lines(["ann 1", "", "   ", "bob 2"]), [("ann", 1), ("bob", 2)])


class RankTest(unittest.TestCase):
    def test_ranks_by_total(self):
        lines = ["ann 10", "bob 30", "cy 20"]
        self.assertEqual(rank_names(lines), ["bob", "cy", "ann"])

    def test_adds_the_amounts_of_a_name(self):
        lines = ["ann 10", "bob 15", "ann 10"]
        self.assertEqual(rank_names(lines), ["ann", "bob"])

    def test_the_top_name(self):
        self.assertEqual(top_name(["ann 10", "bob 30", "cy 20"]), "bob")


if __name__ == "__main__":
    unittest.main()
