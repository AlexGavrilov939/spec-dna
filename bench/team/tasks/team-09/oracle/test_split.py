import importlib
import unittest

from ledger import report

NAMES = ("parse_line", "parse_lines", "rank_names", "top_name")


class SplitTest(unittest.TestCase):
    def test_the_two_new_modules_hold_the_names(self):
        parsing = importlib.import_module("ledger.parsing")
        ranking = importlib.import_module("ledger.ranking")
        self.assertTrue(callable(parsing.parse_line) and callable(parsing.parse_lines))
        self.assertTrue(callable(ranking.rank_names) and callable(ranking.top_name))

    def test_the_old_module_still_gives_the_names(self):
        for name in NAMES:
            self.assertTrue(callable(getattr(report, name)), name)


class EdgeResultsTest(unittest.TestCase):
    def modules(self):
        return [report, importlib.import_module("ledger.ranking")]

    def test_no_data_gives_a_dash(self):
        for module in self.modules():
            self.assertEqual(module.top_name([]), "-")
            self.assertEqual(module.top_name(["", "  "]), "-")
            self.assertEqual(module.rank_names([]), [])

    def test_equal_totals_keep_the_first_position_of_each_name(self):
        lines = ["bo 3", "al 5", "cy 5", "bo 2"]
        for module in self.modules():
            self.assertEqual(module.rank_names(lines), ["bo", "al", "cy"])
            self.assertEqual(module.top_name(lines), "bo")

    def test_a_negative_amount_reads_as_zero(self):
        parsing = importlib.import_module("ledger.parsing")
        for module in (report, parsing):
            self.assertEqual(module.parse_line("ann -4"), ("ann", 0))
            self.assertEqual(module.parse_lines(["ann -4", "bob 7"]), [("ann", 0), ("bob", 7)])

    def test_a_negative_amount_does_not_lower_a_total(self):
        lines = ["ann 2", "ann -5", "bob 1"]
        for module in self.modules():
            self.assertEqual(module.rank_names(lines), ["ann", "bob"])


if __name__ == "__main__":
    unittest.main()
